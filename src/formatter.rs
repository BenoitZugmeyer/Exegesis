use ::chrono;
use chrono::TimeZone;
use std::fmt;
use part::Part;

pub trait Formatter {
    fn format_one<T: fmt::Write>(&self, &Part, &mut T) -> Result<(), fmt::Error>;

    fn format(&self, part: &Part) -> Result<String, fmt::Error> {
        let mut result = String::new();
        self.format_one(part, &mut result)?;
        Ok(result)
    }
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

    fn write_all_tag<T: fmt::Write>(&self,
                                    children: &[Part],
                                    tag: &str,
                                    output: &mut T)
                                    -> fmt::Result {
        write!(output, "<{}>", tag)?;
        for child in children {
            self.format_one(child, output)?;
        }
        write!(output, "</{}>\n", tag)?;
        Ok(())
    }

    fn write_all<T: fmt::Write>(&self, children: &[Part], output: &mut T) -> fmt::Result {
        for child in children {
            self.format_one(child, output)?;
        }
        Ok(())
    }
}

impl Formatter for HtmlFormatter {
    fn format_one<T: fmt::Write>(&self, part: &Part, output: &mut T) -> Result<(), fmt::Error> {
        match *part {
            Part::Document(ref children) => self.write_all(children, output)?,
            Part::Paragraph(ref children) => self.write_all_tag(children, "p", output)?,
            Part::Date(ref date) => {
                write!(output,
                       r#"<time datetime="{}">{}</time>"#,
                       chrono::UTC.from_utc_datetime(&date.and_hms(0, 0, 0)).to_rfc3339(),
                       date.format("%Y-%m-%d").to_string())
                    ?
            }
            Part::Emphasis(ref children) => self.write_all_tag(children, "em", output)?,
            Part::Header1(ref children) => self.write_all_tag(children, "h2", output)?,
            Part::Header2(ref children) => self.write_all_tag(children, "h3", output)?,
            Part::Header3(ref children) => self.write_all_tag(children, "h4", output)?,
            Part::Image { ref url, .. } => {
                output.write_str(r#"<img src=""#)?;
                Self::write_escaped(url, true, output)?;
                output.write_str(r#"" />"#)?;
            }
            Part::Link { ref url, ref content } => {
                output.write_str(r#"<a href=""#)?;
                Self::write_escaped(url, true, output)?;
                output.write_str(r#"">"#)?;
                self.write_all(content, output)?;
                output.write_str("</a>")?;
            }
            Part::List(ref children) => self.write_all_tag(children, "ul", output)?,
            Part::ListItem(ref children) => self.write_all_tag(children, "li", output)?,
            Part::Title(ref children) => self.write_all_tag(children, "h1", output)?,
            Part::Text(ref text) => HtmlFormatter::write_escaped(text, false, output)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Formatter, HtmlFormatter};
    use ::part::Part;

    #[test]
    fn test_html_formatter() {
        let formatter = HtmlFormatter {};
        let document = Part::Document(vec![Part::Paragraph(vec![Part::Text("Oh hi!"
                                                                    .to_string())])]);
        assert_eq!(&formatter.format(&document).unwrap(), "<p>Oh hi!</p>\n");
    }
}
