use ::kuchiki;
use ::chrono;
use std::mem;
use std::error;
use std::fmt;
use std::str;
use date::parse_date;
use part::{Part, Document};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SelectorKind {
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
    PublicationDate,
    Title,
}

impl fmt::Display for SelectorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(match *self {
            SelectorKind::Date => "date",
            SelectorKind::Emphasis => "emphasis",
            SelectorKind::Header1 => "header1",
            SelectorKind::Header2 => "header2",
            SelectorKind::Header3 => "header3",
            SelectorKind::Image => "image",
            SelectorKind::Link => "link",
            SelectorKind::List => "list",
            SelectorKind::ListItem => "list-item",
            SelectorKind::Paragraph => "paragraph",
            SelectorKind::PublicationDate => "publicaton-date",
            SelectorKind::Title => "title",
        })
    }
}

pub const ALL_SELECTOR_KINDS: [SelectorKind; 12] = [SelectorKind::Date,
                                                    SelectorKind::Emphasis,
                                                    SelectorKind::Header1,
                                                    SelectorKind::Header2,
                                                    SelectorKind::Header3,
                                                    SelectorKind::Image,
                                                    SelectorKind::Link,
                                                    SelectorKind::List,
                                                    SelectorKind::ListItem,
                                                    SelectorKind::Paragraph,
                                                    SelectorKind::PublicationDate,
                                                    SelectorKind::Title];

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
    kind: SelectorKind,
    query: kuchiki::Selectors, // absolute: bool,
    priority: u16,
}

impl fmt::Debug for Selector {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.debug_struct("Selector")
            .field("kind", &self.kind)
            .field("query", &SimpleDebugValue("..."))
            .field("priority", &self.priority)
            .finish()
    }
}

impl Selector {
    pub fn new(kind: SelectorKind, query: kuchiki::Selectors) -> Selector {
        Selector {
            kind: kind,
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
    pub on_parse_error: Option<Box<Fn(Box<error::Error>)>>,
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

struct ExtractorResult<'a> {
    document: &'a mut Document,
    parent_children: Vec<Part>,
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
}

impl Extractor {
    pub fn extract(&self, root: &kuchiki::NodeRef) -> Vec<Document> {
        let mut documents = Vec::new();
        self.extract_rec(root, &mut documents);
        documents
    }

    fn extract_rec(&self, root: &kuchiki::NodeRef, documents: &mut Vec<Document>) {
        if let Some(ref root_selector) = self.options.root_selector {

            if let Some(root_element) = root.clone().into_element_ref() {
                if root_selector.matches(&root_element) {
                    documents.push(self.extract_document(root));
                    return;
                }
            }

            for child in root.children() {
                self.extract_rec(&child, documents);
            }

        }
        else {
            documents.push(self.extract_document(root));
        }
    }

    fn extract_document(&self, root: &kuchiki::NodeRef) -> Document {
        let mut document = Document::default();
        {
            let mut result = ExtractorResult {
                document: &mut document,
                parent_children: Vec::new(),
            };
            self.extract_document_rec(root, &mut result);
            result.document.content.append(&mut result.parent_children);
        }
        document
    }

    fn extract_document_rec(&self, root: &kuchiki::NodeRef, mut result: &mut ExtractorResult) {
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

                        mem::swap(&mut result.parent_children, &mut children);
                        self.extract_document_rec(&child, &mut result);
                        mem::swap(&mut result.parent_children, &mut children);

                        if let Err(error) = self.handle_part(&selector.kind,
                                                             child_element,
                                                             children,
                                                             &mut result) {

                            if let Some(ref f) = self.options.on_parse_error {
                                f(error);
                            }
                        }
                        is_node = true;
                        break;
                    }
                }

                if !is_node {
                    self.extract_document_rec(&child, result);
                }
            }
            else if !ignore_text {
                let text = child.text_contents();
                if !text.is_empty() {
                    let mut appended = false;
                    if let Some(&mut Part::Text(ref mut last)) = result.parent_children.last_mut() {
                        appended = true;
                        last.push_str(&text);
                    }

                    if !appended {
                        result.parent_children.push(Part::Text(text));
                    }
                }
            }
        }

    }

    fn parse_date(&self, content: Vec<Part>) -> Result<chrono::NaiveDate, Box<error::Error>> {
        if let Some(ref format) = self.options.date_format {
            Ok(parse_date(format, &text(content))?)
        }
        else {
            Err("No date format")?
        }
    }

    fn handle_part(&self,
                   selector_kind: &SelectorKind,
                   node: &kuchiki::ElementData,
                   children: Vec<Part>,
                   mut result: &mut ExtractorResult)
                   -> Result<(), Box<error::Error>> {

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

        match *selector_kind {
            SelectorKind::Date => result.parent_children.push(Part::Date(self.parse_date(children)?)),
            SelectorKind::Emphasis => result.parent_children.push(Part::Emphasis(children)),
            SelectorKind::Header1 => result.parent_children.push(Part::Header1(children)),
            SelectorKind::Header2 => result.parent_children.push(Part::Header2(children)),
            SelectorKind::Header3 => result.parent_children.push(Part::Header3(children)),
            SelectorKind::Image => {
                result.parent_children.push(Part::Image {
                    url: node.attributes
                        .borrow()
                        .get("src")
                        .ok_or_else(|| "The image tag has no src attribute")?
                        .to_string(),
                    legend: node.attributes.borrow().get("title").map(|s| s.to_string()),
                    width: get_attr!(node, "width"),
                    height: get_attr!(node, "height"),
                })
            }
            SelectorKind::Link => {
                result.parent_children.push(Part::Link {
                    url: node.attributes
                        .borrow()
                        .get("href")
                        .ok_or_else(|| "The link has no href attribute")?
                        .to_string(),
                    content: children,
                })
            }
            SelectorKind::List => result.parent_children.push(Part::List(children)),
            SelectorKind::ListItem => result.parent_children.push(Part::ListItem(children)),
            SelectorKind::Paragraph => result.parent_children.push(Part::Paragraph(children)),
            SelectorKind::PublicationDate => {
                result.document.publication_date = Some(self.parse_date(children)?)
            }
            SelectorKind::Title => result.document.title = Some(children),
        }

        Ok(())
    }
}

#[cfg(test)]
mod extractor {
    use ::extractor::{Extractor, ExtractorOptions, Selector, SelectorKind};
    use ::part::{Part, Document};
    use ::kuchiki;
    use ::chrono;
    use kuchiki::traits::TendrilSink;

    fn extract_markup(selectors: Vec<Selector>,
                      markup: &str,
                      mut options: ExtractorOptions)
                      -> Document {

        if options.on_parse_error.is_none() {
            options.on_parse_error = Some(Box::new(|error| panic!("{}", error)));
        }

        let mut extractor = Extractor::new(options);
        for selector in selectors {
            extractor.add_selector(selector);
        }
        let root = kuchiki::parse_html().one(markup);
        extractor.extract(&root).pop().unwrap()
    }

    #[test]
    fn test_simple() {
        let markup = "\
<DOCTYPE html>
<html>
    <head><title>Hi!</title></head>
    <body>a<span>b </span> c</body>
</html>";
        let document = extract_markup(vec![Selector::new(SelectorKind::Title,
                                                         "title".parse().unwrap())],
                                      markup,
                                      ExtractorOptions::default());

        assert_eq!(document,
                   Document {
                       title: Some(vec![Part::Text("Hi!".to_string())]),
                       publication_date: None,
                       content: vec![Part::Text("\n\n    \n    ab  c\n".to_string())],
                   });
    }

    #[test]
    fn test_date_parsing() {
        let markup = r#"<DOCTYPE html>
<html>
    <head><title>Hi!</title></head>
    <body><span class="date">blah 2015-10-10 blah</span></body>
</html>"#;
        let document = extract_markup(vec![Selector::new(SelectorKind::Date,
                                                         ".date".parse().unwrap())],
                                      markup,
                                      ExtractorOptions {
                                          date_format: Some("%Y-%m-%d".to_string()),
                                          ..ExtractorOptions::default()
                                      });

        assert_eq!(document,
                   Document {
                       title: None,
                       publication_date: None,
                       content: vec![Part::Text("\n\n    Hi!\n    ".to_string()),
                                     Part::Date(chrono::NaiveDate::from_ymd(2015, 10, 10)),
                                     Part::Text("\n".to_string())],
                   });
    }

    #[test]
    fn test_publication_date() {
        let markup = r#"<DOCTYPE html>
<html>
    <head><title>Hi!</title></head>
    <body><span class="date">blah 2015-10-10 blah</span></body>
</html>"#;
        let document = extract_markup(vec![Selector::new(SelectorKind::PublicationDate,
                                                         ".date".parse().unwrap())],
                                      markup,
                                      ExtractorOptions {
                                          date_format: Some("%Y-%m-%d".to_string()),
                                          ..ExtractorOptions::default()
                                      });

        assert_eq!(document,
                   Document {
                       title: None,
                       publication_date: Some(chrono::NaiveDate::from_ymd(2015, 10, 10)),
                       content: vec![Part::Text("\n\n    Hi!\n    \n".to_string())],
                   });
    }

    #[test]
    fn selectors_priority() {
        let mut extractor = Extractor::new(ExtractorOptions::default());
        assert_eq!(extractor.selectors.len(), 0);
        extractor.add_selector(
            Selector::new(SelectorKind::Paragraph, "a".parse().unwrap()).priority(1));
        extractor.add_selector(Selector::new(SelectorKind::Emphasis, "a".parse().unwrap()).priority(2));
        extractor.add_selector(Selector::new(SelectorKind::Header1, "a".parse().unwrap()).priority(3));
        extractor.add_selector(Selector::new(SelectorKind::Header2, "a".parse().unwrap()).priority(2));
        extractor.add_selector(Selector::new(SelectorKind::Date, "a".parse().unwrap()).priority(0));
        extractor.add_selector(Selector::new(SelectorKind::Header3, "a".parse().unwrap()).priority(1));

        assert_eq!(extractor.selectors.len(), 6);

        assert_eq!(extractor.selectors[0].kind, SelectorKind::Header1);
        assert_eq!(extractor.selectors[1].kind, SelectorKind::Emphasis);
        assert_eq!(extractor.selectors[2].kind, SelectorKind::Header2);
        assert_eq!(extractor.selectors[3].kind, SelectorKind::Paragraph);
        assert_eq!(extractor.selectors[4].kind, SelectorKind::Header3);
        assert_eq!(extractor.selectors[5].kind, SelectorKind::Date);
    }

    #[test]
    fn files() {
        let input = include_str!("../tests/rust-at-one-year.html");
        let expected = include_str!("../tests/rust-at-one-year.expected.html");

        let input_extracted =
            extract_markup(vec![Selector::new(SelectorKind::Date, ".post-meta".parse().unwrap()),
                                Selector::new(SelectorKind::Paragraph, "p".parse().unwrap())],
                           input,
                           ExtractorOptions {
                               date_format: Some("%B %d, %Y".to_string()),
                               root_selector: Some(".post".parse().unwrap()),
                               ..ExtractorOptions::default()
                           });
        let expected_extracted =
            extract_markup(vec![Selector::new(SelectorKind::Date, "time".parse().unwrap()),
                                Selector::new(SelectorKind::Paragraph, "p".parse().unwrap())],
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

        for (input_child, expected_child) in input_extracted.content
            .iter()
            .zip(expected_extracted.content.iter()) {
            assert_eq!(input_child, expected_child);
        }

        assert_eq!(input_extracted.content.len(),
                   expected_extracted.content.len());

    }
}
