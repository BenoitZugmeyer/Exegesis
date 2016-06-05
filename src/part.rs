use ::chrono;
use ::regex;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Part {
    Date(chrono::NaiveDate),
    Emphasis(Vec<Part>),
    Header1(Vec<Part>),
    Header2(Vec<Part>),
    Header3(Vec<Part>),
    Image {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
        legend: Option<String>,
    },
    Link {
        url: String,
        content: Vec<Part>,
    },
    List(Vec<Part>),
    ListItem(Vec<Part>),
    Paragraph(Vec<Part>),

    // Special parts
    Text(String),
}

impl Part {
    pub fn children(&self) -> &[Part] {
        match *self {
            Part::Emphasis(ref children) |
            Part::Header1(ref children) |
            Part::Header2(ref children) |
            Part::Header3(ref children) |
            Part::Link { content: ref children, .. } |
            Part::List(ref children) |
            Part::ListItem(ref children) |
            Part::Paragraph(ref children) => children,

            Part::Date(..) |
            Part::Image { .. } |
            Part::Text(..) => &[],
        }
    }

    pub fn text(&self) -> String {
        match *self {
            Part::Text(ref text) => text.to_string(),
            ref other => other.children().iter().map(|c| c.text()).collect(),
        }
    }

    pub fn normalized_text(&self) -> String {
        let re = regex::Regex::new(r"(^\s+|\s+$)|\s+").unwrap();
        re.replace_all(&self.text(), |captures: &regex::Captures| {
            if captures.at(1).is_some() { String::new() } else { " ".to_string() }
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub title: Option<Vec<Part>>,
    pub publication_date: Option<chrono::NaiveDate>,
    pub content: Vec<Part>,
}

#[cfg(test)]
mod parts {
    extern crate serde_json;
    use ::chrono;
    use super::Part;

    #[test]
    fn test_serialization() {
        assert_eq!(serde_json::to_string(&Part::Date(chrono::NaiveDate::from_ymd(2000, 10, 8)))
                       .unwrap(),
                   r#"{"Date":"2000-10-08"}"#);
    }

    #[test]
    fn test_deserialization() {
        assert_eq!(serde_json::from_str::<Part>(r#"{"Date":"2000-10-08"}"#).unwrap(),
                   Part::Date(chrono::NaiveDate::from_ymd(2000, 10, 8)));
    }
}
