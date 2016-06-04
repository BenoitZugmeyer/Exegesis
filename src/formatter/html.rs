use ::chrono;
use std::fmt;
use chrono::TimeZone;
use part::{Document, Part};
use super::Formatter;

macro_rules! write_el{
    ($output: expr, $name:tt) => {
        write_el!($output, $name { })
    };

    ($output: expr, $name:tt => $inner:expr) => {
        write_el!($output, $name { } => $inner)
    };

    ($output: expr, $tag:tt { $( $attr_name:tt = $attr_value:expr )* }) => {{
        write!($output, "<{}", $tag)?;

        $(
            write!($output, r#" {}=""#, $attr_name)?;
            HtmlFormatter::write_escaped($attr_value, true, $output)?;
            $output.write_str(r#"""#)?;
        )*

        $output.write_str("/>")?;
    }};

    ($output: expr, $tag:tt { $( $attr_name:tt = $attr_value:expr )* } => $inner:expr) => {{
        write!($output, "<{}", $tag)?;

        $(
            write!($output, r#" {}=""#, $attr_name)?;
            HtmlFormatter::write_escaped($attr_value, true, $output)?;
            $output.write_str(r#"""#)?;
        )*

        $output.write_str(">")?;
        $inner;
        write!($output, "</{}>\n", $tag)?;
    }}
}

pub struct HtmlFormatter;

impl HtmlFormatter {
    fn write_escaped<T: fmt::Write>(text: &str,
                                    attr_mode: bool,
                                    output: &mut T)
                                    -> Result<(), fmt::Error> {
        for c in text.chars() {
            match c {
                    '&' => output.write_str("&amp;"),
                    '\u{00A0}' => output.write_str("&nbsp;"),
                    '"' if attr_mode => output.write_str("&quot;"),
                    '<' if !attr_mode => output.write_str("&lt;"),
                    '>' if !attr_mode => output.write_str("&gt;"),
                    c => output.write_char(c),
                }
                ?;
        }
        Ok(())
    }

    fn write_date<T: fmt::Write>(&self,
                                 date: &chrono::NaiveDate,
                                 output: &mut T)
                                 -> Result<(), fmt::Error> {
        write_el!(output, "time" {
            "datetime" = &chrono::UTC.from_utc_datetime(&date.and_hms(0, 0, 0)).to_rfc3339()
        } => output.write_str(&date.format("%Y-%m-%d").to_string())?);
        Ok(())
    }
}

impl Formatter for HtmlFormatter {
    fn write_document<T: fmt::Write>(&self,
                                     document: &Document,
                                     output: &mut T)
                                     -> Result<(), fmt::Error> {
        write_el!(output, "article" => {
            if document.title.is_some() || document.publication_date.is_some() {
                write_el!(output, "header" => {
                    if let Some(ref title) = document.title {
                        write_el!(output, "h2" => self.write_parts(title, output)?);
                    }
                    if let Some(ref date) = document.publication_date {
                        write_el!(output, "p" => {
                            output.write_str("On ")?;
                            self.write_date(date, output)?
                        });
                    }

                });
            }
            self.write_parts(&document.content, output)?;
        });
        Ok(())
    }

    fn write_part<T: fmt::Write>(&self, part: &Part, output: &mut T) -> Result<(), fmt::Error> {
        match *part {
            Part::Paragraph(ref children) => {
                write_el!(output, "p" => self.write_parts(children, output)?)
            }

            Part::Date(ref date) => {
                self.write_date(date, output)?;
            }

            Part::Emphasis(ref children) => {
                write_el!(output, "em" => self.write_parts(children, output)?)
            }
            Part::Header1(ref children) => {
                write_el!(output, "h2" => self.write_parts(children, output)?)
            }
            Part::Header2(ref children) => {
                write_el!(output, "h3" => self.write_parts(children, output)?)
            }
            Part::Header3(ref children) => {
                write_el!(output, "h4" => self.write_parts(children, output)?)
            }
            Part::Image { ref url, .. } => {
                write_el!(output, "img" {
                    "src" = url
                });
            }
            Part::Link { ref url, ref content } => {
                write_el!(output, "a" {
                    "href" = url
                } => self.write_parts(content, output)?);
            }
            Part::List(ref children) => {
                write_el!(output, "ul" => self.write_parts(children, output)?)
            }
            Part::ListItem(ref children) => {
                write_el!(output, "li" => self.write_parts(children, output)?)
            }
            Part::Text(ref text) => HtmlFormatter::write_escaped(text, false, output)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ::chrono;
    use ::formatter::Formatter;
    use ::part::{Document, Part};
    use super::HtmlFormatter;

    #[test]
    fn test_html_formatter() {
        let formatter = HtmlFormatter {};
        let document = Document {
            content: vec![Part::Paragraph(vec![Part::Text(r#"Oh hi! <>""#.to_string()),
                                               Part::Link {
                                                   url: r#"<>""#.to_string(),
                                                   content: vec![Part::Text("link".to_string())],
                                               }])],
            publication_date: Some(chrono::NaiveDate::from_ymd(2000, 10, 7)),
            ..Document::default()
        };
        assert_eq!(&formatter.format(&document).unwrap(),
                   "\
            <article>\
                <header>\
                    <p>\
                        On <time datetime=\"2000-10-07T00:00:00+00:00\">2000-10-07</time>\n\
                    </p>\n\
                </header>\n\
                <p>\
                    Oh hi! &lt;&gt;\"\
                    <a href=\"<>&quot;\">link</a>\n\
                </p>\n\
            </article>\n");
    }
}
