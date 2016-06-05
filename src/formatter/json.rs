extern crate serde_json;
use self::serde_json::ser;
use std::io;
use std::error;
use part::Document;
use super::Formatter;

pub struct JsonFormatter;

impl JsonFormatter {}

impl Formatter for JsonFormatter {
    fn write_document<T: io::Write>(&self,
                                    document: &Document,
                                    output: &mut T)
                                    -> Result<(), Box<error::Error>> {
        Ok(ser::to_writer(output, document)?)
    }
}

#[cfg(test)]
mod tests {
    use ::chrono;
    use ::formatter::Formatter;
    use ::part::{Document, Part};
    use super::JsonFormatter;

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter {};
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
            {\
                \"title\":null,\
                \"publication_date\":\"2000-10-07\",\
                \"content\":[\
                    {\
                        \"Paragraph\":[\
                            {\"Text\":\"Oh hi! <>\\\"\"},\
                            {\"Link\":{\
                                \"url\":\"<>\\\"\",\
                                \"content\":[{\"Text\":\"link\"}]\
                            }}\
                        ]\
                    }\
                ]\
            }");
    }
}
