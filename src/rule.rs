
use std::error;

use super::extractor::Extractor;
use super::website::Website;
use super::part::Part;
use super::matcher;

#[derive(Debug)]
pub struct Rule {
    name: String,
    matchers: Vec<Box<matcher::Matcher>>,
    extractor: Extractor,
}

impl Rule {
    pub fn new(name: String, extractor: Extractor) -> Self {
        Rule {
            name: name,
            matchers: Vec::new(),
            extractor: extractor,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_matcher(&mut self, matcher: Box<matcher::Matcher>) {
        self.matchers.push(matcher);
    }
}

pub fn extract(rules: &[Rule], website: &Website) -> Result<Part, Box<error::Error>> {
    let dom = website.dom.as_ref().ok_or("This website has no DOM")?;

    let rule = rules.iter()
        .find(|rule| rule.matchers.iter().any(|m| m.matches(website)))
        .ok_or("No rule matching this website")?;

    Ok(rule.extractor.extract(dom))
}
