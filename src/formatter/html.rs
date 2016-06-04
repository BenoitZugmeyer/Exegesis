use ::chrono;
use std::fmt;
use chrono::TimeZone;
use part::{Document, Part};
use super::Formatter;

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
        self.write_parts(children, output)?;
        write!(output, "</{}>\n", tag)?;
        Ok(())
    }
}

impl Formatter for HtmlFormatter {
    fn write_document<T: fmt::Write>(&self,
                                     document: &Document,
                                     output: &mut T)
                                     -> Result<(), fmt::Error> {
        output.write_str("<article>")?;
        if document.title.is_some() {
            output.write_str("<header>")?;
            if let Some(ref title) = document.title {
                self.write_all_tag(title, "h2", output)?;
            }
            output.write_str("</header>\n")?;
        }
        self.write_parts(&document.content, output)?;
        output.write_str("</article>\n")?;
        Ok(())
    }

    fn write_part<T: fmt::Write>(&self, part: &Part, output: &mut T) -> Result<(), fmt::Error> {
        match *part {
            Part::Paragraph(ref children) => self.write_all_tag(children, "p", output)?,
            Part::PublicationDate(ref date) |
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
                self.write_parts(content, output)?;
                output.write_str("</a>")?;
            }
            Part::List(ref children) => self.write_all_tag(children, "ul", output)?,
            Part::ListItem(ref children) => self.write_all_tag(children, "li", output)?,
            Part::Text(ref text) => HtmlFormatter::write_escaped(text, false, output)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::HtmlFormatter;
    use ::formatter::Formatter;
    use ::part::{Document, Part};

    #[test]
    fn test_html_formatter() {
        let formatter = HtmlFormatter {};
        let document = Document {
            content: vec![Part::Paragraph(vec![Part::Text("Oh hi!".to_string())])],
            ..Document::default()
        };
        assert_eq!(&formatter.format(&document).unwrap(),
                   "<article><p>Oh hi!</p>\n</article>\n");
    }
}
