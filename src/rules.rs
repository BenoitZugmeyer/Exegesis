
use ::serde;
use serde::de;
use std::error;

use super::rule::Rule;
use super::website::Website;
use super::part::Document;

#[derive(Debug, Default)]
pub struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    pub fn extract(&self, website: &Website) -> Result<Vec<Document>, Box<error::Error>> {
        let dom = website.dom.as_ref().ok_or("This website has no DOM")?;

        let rule = self.rules
            .iter()
            .find(|rule| rule.matchers.iter().any(|m| m.matches(website)))
            .ok_or("No rule matching this website")?;

        Ok(rule.extractor.extract(dom))
    }

    pub fn append(&mut self, mut other: Rules) {
        self.rules.append(&mut other.rules);
    }
}


// Deserialization

impl serde::Deserialize for Rules {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        deserializer.deserialize_map(RulesMapVisitor)
    }
}

struct RulesMapVisitor;

impl de::Visitor for RulesMapVisitor {
    type Value = Rules;
    fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
        where V: de::MapVisitor
    {
        let mut rules = Rules::default();

        while let Some(rule_name) = visitor.visit_key::<String>()? {
            let mut rule: Rule = visitor.visit_value()?;
            rule.name = rule_name;
            rules.rules.push(rule);
        }

        visitor.end()?;

        Ok(rules)
    }
}


#[cfg(test)]
mod tests {
    extern crate serde_json;
    extern crate serde;
    extern crate toml;

    use std::error::Error;
    use super::Rules;

    #[test]
    fn test_deserialization() {
        let json = r#"
{
    "rustlang_blog": {
        "include_url": ["*//blog.rust-lang.org/**"],
        "date_format": "%B %d, %Y",
        "root": ".foo",
        "list-item": ".blah"
    }
}
"#;

        let rules = serde_json::from_str::<Rules>(json).unwrap();
        assert_eq!(rules.rules.len(), 1);
        assert_eq!(rules.rules[0].name, "rustlang_blog");
    }

    fn parse_rules_from_str(source: &str) -> Result<Rules, toml::DecodeError> {
        let mut parser = toml::Parser::new(&source);
        let table = parser.parse().unwrap();
        let mut decoder = toml::Decoder::new(toml::Value::Table(table));
        serde::Deserialize::deserialize(&mut decoder)
    }

    fn parse_and_unwrap_error(source: &str) -> toml::DecodeError {
        parse_rules_from_str(source).unwrap_err()
    }

    #[test]
    fn fails_if_rules_is_not_a_table() {
        let error = parse_and_unwrap_error("[[rules]]\n");

        assert_eq!(format!("{}", error), "expected a value of type `table`, but found a value of type `array` for the key `rules`");
        assert_eq!(error.description(), "expected a type");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_rule_is_not_a_table() {
        let error = parse_and_unwrap_error("foo = false");

        assert_eq!(format!("{}", error), "expected a value of type `table`, but found a value of type `boolean` for the key `foo`");
        assert_eq!(error.description(), "expected a type");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_rule_include_url_is_not_a_string() {
        let error = parse_and_unwrap_error("[foo]
                                           include_url = false");

        assert_eq!(format!("{}", error), "invalid type: bool");
        assert_eq!(error.description(), "invalid type");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_date_format_is_not_a_string() {
        let error = parse_and_unwrap_error("[foo]
                                           date_format = false");

        assert_eq!(format!("{}", error), "expected a value of type `string`, but found a value of type `boolean` for the key `foo.date_format`");
        assert_eq!(error.description(), "expected a type");
        assert!(error.cause().is_none());
    }

    #[test]
    fn matches_an_url() {
        let rules = parse_rules_from_str(r#"
        [wordpress]
        include_url = "*//foo"
        "#)
            .expect("Failed to parse toml");

        assert_eq!(rules.rules.len(), 1);
        assert_eq!(rules.rules[0].name, "wordpress");
        // TODO
    }

    #[test]
    fn include_url_can_be_an_array() {
        let rules = parse_rules_from_str(r#"
        [wordpress]
        include_url = ["*//foo", "*//bar"]
        "#)
            .expect("Failed to parse toml");

        assert_eq!(rules.rules.len(), 1);
        assert_eq!(rules.rules[0].name, "wordpress");
        // TODO
    }

    #[test]
    fn fails_if_root_has_error() {
        let error = parse_and_unwrap_error(r#"[foo]
                                           root = "blih >""#);

        assert_eq!(format!("{}", error), "custom error: Failed to parse CSS selector");
        assert_eq!(error.description(), "custom error");
        assert!(error.cause().is_none());
    }
}
