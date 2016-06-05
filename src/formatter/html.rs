use ::chrono;
use std::io;
use std::error;
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
            $output.write_all(br#"""#)?;
        )*

        $output.write_all(b"/>")?;
    }};

    ($output: expr, $tag:tt { $( $attr_name:tt = $attr_value:expr )* } => $inner:expr) => {{
        write!($output, "<{}", $tag)?;

        $(
            write!($output, r#" {}=""#, $attr_name)?;
            HtmlFormatter::write_escaped($attr_value, true, $output)?;
            $output.write_all(br#"""#)?;
        )*

        $output.write_all(b">")?;
        $inner;
        write!($output, "</{}>\n", $tag)?;
    }}
}

pub struct HtmlFormatter;

impl HtmlFormatter {
    fn write_escaped<T: io::Write>(text: &str,
                                   attr_mode: bool,
                                   output: &mut T)
                                   -> Result<(), io::Error> {
        for c in text.bytes() {
            match c {
                    b'&' => output.write_all(b"&amp;"),
                    0x00A0 => output.write_all(b"&nbsp;"),
                    b'"' if attr_mode => output.write_all(b"&quot;"),
                    b'<' if !attr_mode => output.write_all(b"&lt;"),
                    b'>' if !attr_mode => output.write_all(b"&gt;"),
                    c => output.write_all(&[c]),
                }
                ?;
        }
        Ok(())
    }

    fn write_date<T: io::Write>(&self,
                                date: &chrono::NaiveDate,
                                output: &mut T)
                                -> Result<(), io::Error> {
        write_el!(output, "time" {
            "datetime" = &chrono::UTC.from_utc_datetime(&date.and_hms(0, 0, 0)).to_rfc3339()
        } => output.write_all(&date.format("%Y-%m-%d").to_string().as_bytes())?);
        Ok(())
    }

    fn write_parts<T: io::Write>(&self, children: &[Part], output: &mut T) -> io::Result<()> {
        for child in children {
            self.write_part(child, output)?;
        }
        Ok(())
    }

    fn write_part<T: io::Write>(&self, part: &Part, output: &mut T) -> Result<(), io::Error> {
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

impl Formatter for HtmlFormatter {
    fn write_document<T: io::Write>(&self,
                                    document: &Document,
                                    output: &mut T)
                                    -> Result<(), Box<error::Error>> {
        write_el!(output, "article" => {
            if document.title.is_some() || document.publication_date.is_some() {
                write_el!(output, "header" => {
                    if let Some(ref title) = document.title {
                        write_el!(output, "h2" => self.write_parts(title, output)?);
                    }
                    if let Some(ref date) = document.publication_date {
                        write_el!(output, "p" => {
                            output.write_all(b"On ")?;
                            self.write_date(date, output)?
                        });
                    }

                });
            }
            self.write_parts(&document.content, output)?;
        });
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
