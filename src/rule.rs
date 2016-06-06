
use ::serde;
use ::kuchiki;
use serde::de;

use super::extractor::Extractor;
use super::matcher;
use super::extractor;

#[derive(Debug, Default)]
pub struct Rule {
    pub name: String,
    pub matchers: Vec<Box<matcher::Matcher>>,
    pub extractor: Extractor,
}


// Deserialization

#[derive(Debug, Default)]
struct Splat {
    values: Vec<String>,
}

impl serde::Deserialize for Splat {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        deserializer.deserialize(SplatVisitor)
    }
}

struct SplatVisitor;

impl de::Visitor for SplatVisitor {
    type Value = Splat;
    fn visit_string<E>(&mut self, v: String) -> Result<Self::Value, E>
        where E: de::Error
    {
        Ok(Splat { values: vec![v] })
    }

    fn visit_seq<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
        where V: de::SeqVisitor
    {
        let mut result = Splat::default();
        while let Some(s) = visitor.visit()? {
            result.values.push(s);
        }
        visitor.end()?;

        Ok(result)
    }
}


impl serde::Deserialize for Rule {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        deserializer.deserialize_map(RuleMapVisitor)
    }
}

struct RuleMapVisitor;

impl RuleMapVisitor {
    fn visit_include_url<V>(&self, rule: &mut Rule, visitor: &mut V) -> Result<(), V::Error>
        where V: de::MapVisitor
    {
        let splat: Splat = visitor.visit_value()?;
        for s in splat.values {
            let matcher = Box::new(matcher::URLMatcher::new(&s)
                .map_err(|e| de::Error::custom(e.to_string()))?);
            rule.matchers.push(matcher);
        }
        Ok(())
    }

    fn visit_kuchiki_selectors<V>(&self, visitor: &mut V) -> Result<kuchiki::Selectors, V::Error>
        where V: de::MapVisitor
    {
        let value = visitor.visit_value::<String>()?;
        Ok(value.parse().map_err(|_| de::Error::custom("Failed to parse CSS selector"))?)
    }

    fn visit_field<V>(&self,
                      name: &str,
                      mut rule: &mut Rule,
                      mut visitor: &mut V)
                      -> Result<(), V::Error>
        where V: de::MapVisitor
    {
        match name {
            "include_url" => self.visit_include_url(&mut rule, &mut visitor)?,
            "date_format" => rule.extractor.options.date_format = Some(visitor.visit_value()?),
            "root" => {
                rule.extractor.options.root_selector =
                    Some(self.visit_kuchiki_selectors(&mut visitor)?)
            }
            selector_kind => {
                match extractor::SelectorKind::from_str(selector_kind) {
                    Some(kind) => {
                        let selectors = self.visit_kuchiki_selectors(&mut visitor)?;
                        rule.extractor
                            .add_selector(extractor::Selector::new(kind, selectors));
                    }
                    None => return Err(de::Error::unknown_field(name)),
                }
            }
        }
        Ok(())
    }
}

impl de::Visitor for RuleMapVisitor {
    type Value = Rule;

    fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
        where V: de::MapVisitor
    {
        let mut result = Rule::default();

        while let Some(v) = visitor.visit_key::<String>()? {
            self.visit_field(v.as_ref(), &mut result, &mut visitor)?;
        }

        visitor.end()?;

        Ok(result)
    }
}
