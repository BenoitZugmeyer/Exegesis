use ::kuchiki;
use std::error::Error;
use std::fmt;
use std::str;
use date::parse_date;
use part::Part;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PartType {
    Date,
    Emphasis,
    Header1,
    Header2,
    Header3,
    Image,
    Link,
    List,
    ListItem,
    Paragraph,
    Title,
}

impl fmt::Display for PartType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(match *self {
            PartType::Date => "date",
            PartType::Emphasis => "emphasis",
            PartType::Header1 => "header1",
            PartType::Header2 => "header2",
            PartType::Header3 => "header3",
            PartType::Image => "image",
            PartType::Link => "link",
            PartType::List => "list",
            PartType::ListItem => "list-item",
            PartType::Paragraph => "paragraph",
            PartType::Title => "title",
        })
    }
}

struct SimpleDebugValue(&'static str);

impl fmt::Debug for SimpleDebugValue {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

macro_rules! sdv_from_option{
    ($value:expr, $s:expr) => (
        SimpleDebugValue(if $value.is_some() {
            concat!("Some(", $s, ")")
        }
        else {
            "None"
        })
    )
}

pub struct Selector {
    part: PartType,
    query: kuchiki::Selectors, // absolute: bool,
    priority: u16,
}

impl fmt::Debug for Selector {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.debug_struct("Selector")
            .field("part", &self.part)
            .field("query", &SimpleDebugValue("..."))
            .field("priority", &self.priority)
            .finish()
    }
}

impl Selector {
    pub fn new(part: PartType, query: kuchiki::Selectors) -> Selector {
        Selector {
            part: part,
            query: query,
            priority: 0,
        }
    }

    pub fn priority(mut self, p: u16) -> Selector {
        self.priority = p;
        self
    }
}

#[derive(Default)]
pub struct ExtractorOptions {
    pub on_parse_error: Option<Box<Fn(Box<Error>)>>,
    pub date_format: Option<String>,
    pub root_selector: Option<kuchiki::Selectors>,
}

impl fmt::Debug for ExtractorOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.debug_struct("ExtractorOptions")
            .field("on_parse_error",
                   &sdv_from_option!(self.on_parse_error, "fn"))
            .field("date_format", &self.date_format)
            .field("root_selector",
                   &sdv_from_option!(self.root_selector, "selector"))
            .finish()
    }
}

#[derive(Default)]
pub struct Extractor {
    selectors: Vec<Selector>,
    options: ExtractorOptions,
}

impl fmt::Debug for Extractor {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.debug_struct("Extractor")
            .field("selectors", &self.selectors)
            .field("options", &self.options)
            .finish()
    }
}

fn text(mut content: Vec<Part>) -> String {
    content.drain(..)
        .filter_map(|part| {
            match part {
                Part::Text(text) => Some(text),
                _ => None,
            }
        })
        .collect()
}

impl Extractor {
    pub fn new(options: ExtractorOptions) -> Extractor {
        Extractor {
            selectors: Vec::new(),
            options: options,
        }
    }

    fn new_part(&self,
                part_type: &PartType,
                node: &kuchiki::ElementData,
                children: Vec<Part>)
                -> Result<Part, Box<Error>> {
        macro_rules! get_attr {
            ($node:expr, $name:expr) => (
                if let Some(attr) = $node.attributes.borrow().get($name) {
                    attr.parse().ok() // TODO handle error, handle XXXpx format
                }
                else {
                    None
                }
            )
        }

        Ok(match *part_type {
            PartType::Date => {
                if let Some(ref format) = self.options.date_format {
                    Part::Date(parse_date(format, &text(children))?)
                }
                else {
                    Err("No date format")?
                }
            }
            PartType::Emphasis => Part::Emphasis(children),
            PartType::Header1 => Part::Header1(children),
            PartType::Header2 => Part::Header2(children),
            PartType::Header3 => Part::Header3(children),
            PartType::Image => {
                Part::Image {
                    url: node.attributes
                        .borrow()
                        .get("src")
                        .ok_or_else(|| "The image tag has no src attribute")?
                        .to_string(),
                    legend: node.attributes.borrow().get("title").map(|s| s.to_string()),
                    width: get_attr!(node, "width"),
                    height: get_attr!(node, "height"),
                }
            }
            PartType::Link => {
                Part::Link {
                    url: node.attributes
                        .borrow()
                        .get("href")
                        .ok_or_else(|| "The link has no href attribute")?
                        .to_string(),
                    content: children,
                }
            }
            PartType::List => Part::List(children),
            PartType::ListItem => Part::ListItem(children),
            PartType::Paragraph => Part::Paragraph(children),
            PartType::Title => Part::Title(children),
        })
    }

    pub fn add_selector(&mut self, selector: Selector) {
        if let Some(index) = self.selectors
            .iter()
            .enumerate()
            .skip_while(|&(_, s)| s.priority >= selector.priority)
            .map(|(i, _)| i)
            .next() {
            self.selectors.insert(index, selector);
        }
        else {
            self.selectors.push(selector);
        }
    }

    pub fn extract(&self, root: &kuchiki::NodeRef) -> Part {
        let mut children = Vec::new();
        self.extract_rec(root, &mut children);
        Part::Document(children)
    }

    fn extract_rec(&self, root: &kuchiki::NodeRef, mut content: &mut Vec<Part>) {
        let ignore_text = {
            if let Some(ref el) = root.as_element() {
                el.name.local.eq_str_ignore_ascii_case("script") ||
                el.name.local.eq_str_ignore_ascii_case("style")
            }
            else {
                false
            }
        };

        for child in root.children() {
            if let Some(ref child_element) = child.clone().into_element_ref() {
                let mut is_node = false;
                for selector in &self.selectors {
                    if selector.query.matches(child_element) {
                        let mut children = Vec::new();
                        self.extract_rec(&child, &mut children);

                        match self.new_part(&selector.part, child_element, children) {
                            Ok(part) => content.push(part),
                            Err(error) => {
                                if let Some(ref f) = self.options.on_parse_error {
                                    f(error);
                                }
                            }
                        }
                        is_node = true;
                        break;
                    }
                }

                if !is_node {
                    self.extract_rec(&child, &mut content);
                }
            }
            else if !ignore_text {
                let text = child.text_contents();
                if !text.is_empty() {
                    let mut appended = false;
                    if let Some(&mut Part::Text(ref mut last)) = content.last_mut() {
                        appended = true;
                        last.push_str(&text);
                    }

                    if !appended {
                        content.push(Part::Text(text));
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod extractor {
    use ::extractor::{Extractor, ExtractorOptions, Selector, PartType};
    use ::part::Part;
    use ::kuchiki;
    use ::chrono;
    use kuchiki::traits::TendrilSink;

    fn extract_markup(selectors: Vec<Selector>,
                      markup: &str,
                      mut options: ExtractorOptions)
                      -> Part {

        if options.on_parse_error.is_none() {
            options.on_parse_error = Some(Box::new(|error| panic!("{}", error)));
        }

        let mut extractor = Extractor::new(options);
        for selector in selectors {
            extractor.add_selector(selector);
        }
        let root = kuchiki::parse_html().one(markup);
        extractor.extract(&root)
    }

    #[test]
    fn test() {
        let markup = "\
<DOCTYPE html>
<html>
    <head><title>Hi!</title></head>
    <body>a<span>b </span> c</body>
</html>";
        let document = extract_markup(vec![Selector::new(PartType::Title,
                                                         "title".parse().unwrap())],
                                      markup,
                                      ExtractorOptions::default());

        assert_eq!(document.text(), "\n\n    Hi!\n    ab  c\n");
        assert_eq!(document.normalized_text(), "Hi! ab c");
        assert_eq!(document,
                   Part::Document(vec![Part::Text("\n\n    ".to_string()),
                                       Part::Title(vec![Part::Text("Hi!".to_string())]),
                                       Part::Text("\n    ab  c\n".to_string())]));
    }

    #[test]
    fn test_date_parsing() {
        let markup = r#"<DOCTYPE html>
<html>
    <head><title>Hi!</title></head>
    <body><span class="date">blah 2015-10-10 blah</span></body>
</html>"#;
        let document = extract_markup(vec![Selector::new(PartType::Date,
                                                         ".date".parse().unwrap())],
                                      markup,
                                      ExtractorOptions {
                                          date_format: Some("%Y-%m-%d".to_string()),
                                          ..ExtractorOptions::default()
                                      });

        assert_eq!(document.text(), "\n\n    Hi!\n    \n");
        assert_eq!(document.normalized_text(), "Hi!");
        assert_eq!(document,
                   Part::Document(vec![Part::Text("\n\n    Hi!\n    ".to_string()),
                                       Part::Date(chrono::NaiveDate::from_ymd(2015, 10, 10)),
                                       Part::Text("\n".to_string())]));
    }


    #[test]
    fn selectors_priority() {
        let mut extractor = Extractor::new(ExtractorOptions::default());
        assert_eq!(extractor.selectors.len(), 0);
        extractor.add_selector(
            Selector::new(PartType::Paragraph, "a".parse().unwrap()).priority(1));
        extractor.add_selector(Selector::new(PartType::Emphasis, "a".parse().unwrap()).priority(2));
        extractor.add_selector(Selector::new(PartType::Header1, "a".parse().unwrap()).priority(3));
        extractor.add_selector(Selector::new(PartType::Header2, "a".parse().unwrap()).priority(2));
        extractor.add_selector(Selector::new(PartType::Date, "a".parse().unwrap()).priority(0));
        extractor.add_selector(Selector::new(PartType::Header3, "a".parse().unwrap()).priority(1));

        assert_eq!(extractor.selectors.len(), 6);

        assert_eq!(extractor.selectors[0].part, PartType::Header1);
        assert_eq!(extractor.selectors[1].part, PartType::Emphasis);
        assert_eq!(extractor.selectors[2].part, PartType::Header2);
        assert_eq!(extractor.selectors[3].part, PartType::Paragraph);
        assert_eq!(extractor.selectors[4].part, PartType::Header3);
        assert_eq!(extractor.selectors[5].part, PartType::Date);
    }

    #[test]
    fn files() {
        let input = include_str!("../tests/rust-at-one-year.html");
        let expected = include_str!("../tests/rust-at-one-year.expected.html");

        let input_extracted =
            extract_markup(vec![Selector::new(PartType::Date, ".post-meta".parse().unwrap()),
                                Selector::new(PartType::Paragraph, "p".parse().unwrap())],
                           input,
                           ExtractorOptions {
                               date_format: Some("%B %d, %Y".to_string()),
                               ..ExtractorOptions::default()
                           });
        let expected_extracted =
            extract_markup(vec![Selector::new(PartType::Date, "time".parse().unwrap()),
                                Selector::new(PartType::Paragraph, "p".parse().unwrap())],
                           expected,
                           ExtractorOptions {
                               date_format: Some("%Y-%m-%d".to_string()),
                               ..ExtractorOptions::default()
                           });

        // print!("AAAA");
        // fn print_children(input: &Part) {
        //     for input_child in input.children() {
        //         match *input_child {
        //             Part::Paragraph(..) => {
        //                 print!("<p>");
        //                 print_children(&input_child);
        //                 print!("</p>");
        //             }
        //             Part::Date(ref date) => print!(r#"<time>{}</time>"#, date),
        //             Part::Text(..) => print!("<span>{}</span>", input_child.text()),
        //             _ => {}
        //         }
        //     }
        // }
        // print_children(&input_extracted);
        // print!("AAAA");
        // panic!("Print");

        for (input_child, expected_child) in input_extracted.children()
            .iter()
            .zip(expected_extracted.children().iter()) {
            assert_eq!(input_child, expected_child);
        }

        assert_eq!(input_extracted.children().len(),
                   expected_extracted.children().len());

    }
}
