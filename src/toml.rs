extern crate toml;

use std::error;
use std::fmt;
use std::io::Write;
use extractor::{Extractor, ExtractorOptions, SelectorKind, Selector, ALL_SELECTOR_KINDS};
use kuchiki::Selectors;
use rule;
use matcher;

#[derive(Debug)]
pub struct ParserError {
    description: String,
    line: usize,
    column: usize,
}

#[derive(Debug)]
pub enum Error {
    ParserErrors(Vec<ParserError>),
    FormatError(String),
    SelectorParserError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ParserErrors(ref errors) => {
                for error in errors {
                    write!(f,
                           "{}:{}  {}\n",
                           error.line + 1,
                           error.column + 1,
                           error.description)
                        ?;
                }
                Ok(())
            }
            Error::FormatError(ref reason) => write!(f, "{}", reason),
            Error::SelectorParserError => write!(f, "{}", "Failed to parse the selector"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ParserErrors(..) => "TOML parser errors",
            Error::FormatError(..) => "TOML format error",
            Error::SelectorParserError => "Selector parser error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

fn parse_selector(selector_kind: SelectorKind,
                  table: &toml::Table,
                  extractor: &mut Extractor)
                  -> Result<(), Box<error::Error>> {
    let name = selector_kind.to_string();

    if let Some(value) = table.get(&name) {
        let query = value.as_str()
            .ok_or_else(|| Error::FormatError(format!("{} should be a string", name)))?;

        let selector = Selector::new(selector_kind, parse_kuchiki_selectors(query)?);
        extractor.add_selector(selector);
    }
    Ok(())
}

fn splat<F>(opt_value: Option<&toml::Value>, name: &str, f: &mut F) -> Result<(), Box<error::Error>>
    where F: FnMut(&str) -> Result<(), Box<error::Error>>
{
    match opt_value {
        None => {}
        Some(&toml::Value::String(ref s)) => f(s)?,
        Some(&toml::Value::Array(ref array)) => {
            for v in array {
                splat(Some(v), name, f)?
            }
        }
        _ => Err(Error::FormatError(format!("'{}' should be a string", name)))?,
    }

    Ok(())
}

fn get_string_opt<'l>(table: &'l toml::Table, name: &str) -> Result<Option<&'l str>, Error> {
    match table.get(name) {
        None => Ok(None),
        Some(&toml::Value::String(ref s)) => Ok(Some(s)),
        _ => Err(Error::FormatError(format!("'{}' should be a string", name)))?,
    }
}

fn parse_kuchiki_selectors(s: &str) -> Result<Selectors, Box<error::Error>> {
    Ok(s.parse().map_err(|_| Error::SelectorParserError)?)
}

trait MapThen<T> {
    fn map_then<F, U, E>(self, f: F) -> Result<Option<U>, E> where F: FnOnce(T) -> Result<U, E>;
}

impl<T> MapThen<T> for Option<T> {
    fn map_then<F, U, E>(self, f: F) -> Result<Option<U>, E>
        where F: FnOnce(T) -> Result<U, E>
    {
        match self {
            Some(v) => {
                match f(v) {
                    Ok(result) => Ok(Some(result)),
                    Err(error) => Err(error),
                }
            }
            None => Ok(None),
        }
    }
}

fn parse_rule_from_toml_value(name: &str,
                              value: &toml::Value)
                              -> Result<rule::Rule, Box<error::Error>> {


    let table = value.as_table()
        .ok_or_else(|| Error::FormatError(format!("The rule '{}' should be a table", name)))?;

    let extractor_options = ExtractorOptions {
        date_format: get_string_opt(table, "date_format")?.map(String::from),
        root_selector: get_string_opt(table, "root")?.map_then(parse_kuchiki_selectors)?,
        on_parse_error: Some(Box::new(|error| {
            writeln!(::std::io::stderr(), "Warning: while parsing: {}", error)
                .expect("Failed to write on stderr"); // TODO move this elsewhere
        })),
        ..ExtractorOptions::default()
    };

    let mut extractor = Extractor::new(extractor_options);

    for part in &ALL_SELECTOR_KINDS {
        parse_selector(*part, table, &mut extractor)?;
    }

    let mut result = rule::Rule::new(name.to_owned(), extractor);

    splat(table.get("include_url"),
          "include_url",
          &mut |s| {
              result.add_matcher(Box::new(matcher::URLMatcher::new(s)?));
              Ok(())
          })
        ?;

    Ok(result)
}

pub fn parse_rules(toml: &toml::Table) -> Result<Vec<rule::Rule>, Box<error::Error>> {
    let rule_declarations = toml.get("rules")
        .ok_or_else(|| Error::FormatError("No 'rules' key".to_string()))?
        .as_table()
        .ok_or_else(|| Error::FormatError("The 'rules' value should be a table".to_string()))?;

    let mut rules = Vec::new();

    for (name, value) in rule_declarations.iter() {
        rules.push(parse_rule_from_toml_value(name, value)?);
    }

    Ok(rules)
}

pub fn parse_rules_from_str(toml: &str) -> Result<Vec<rule::Rule>, Box<error::Error>> {
    let mut parser = toml::Parser::new(toml);

    let parsed = parser.parse()
        .ok_or_else(move || {
            let errors: Vec<_> = parser.errors
                .iter()
                .map(|error| {
                    let linecol = parser.to_linecol(error.lo);
                    ParserError {
                        description: error.desc.clone(),
                        line: linecol.0,
                        column: linecol.1,
                    }
                })
                .collect();
            Error::ParserErrors(errors)
        })?;

    Ok(parse_rules(&parsed)?)
}

#[cfg(test)]
mod parse_rule {
    use std::error::Error as StdError;
    use super::{Error, parse_rules_from_str};

    fn parse_and_unwrap_error(input: &str) -> Error {
        *parse_rules_from_str(input).unwrap_err().downcast::<Error>().unwrap()
    }

    #[test]
    fn fails_if_parse_error() {
        let error = parse_and_unwrap_error("[coucou");

        assert_eq!(format!("{}", error), "1:8  expected `.`, but found eof\n");
        assert_eq!(error.description(), "TOML parser errors");
        assert!(error.cause().is_none());

        if let Error::ParserErrors(errors) = error {
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].line, 0);
            assert_eq!(errors[0].column, 7);
            assert_eq!(errors[0].description, "expected `.`, but found eof");
        }
        else {
            panic!("parse_rules_from_str did not fail with ParserErrors");
        }
    }

    #[test]
    fn fails_if_no_rules_entry() {
        let error = parse_and_unwrap_error("[coucou]\n");

        assert_eq!(format!("{}", error), "No 'rules' key");
        assert_eq!(error.description(), "TOML format error");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_rules_is_not_a_table() {
        let error = parse_and_unwrap_error("[[rules]]\n");

        assert_eq!(format!("{}", error), "The 'rules' value should be a table");
        assert_eq!(error.description(), "TOML format error");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_rule_is_not_a_table() {
        let error = parse_and_unwrap_error("[rules]
                                           foo = false");

        assert_eq!(format!("{}", error), "The rule 'foo' should be a table");
        assert_eq!(error.description(), "TOML format error");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_rule_include_url_is_not_a_string() {
        let error = parse_and_unwrap_error("[rules.foo]
                                           include_url = false");

        assert_eq!(format!("{}", error), "'include_url' should be a string");
        assert_eq!(error.description(), "TOML format error");
        assert!(error.cause().is_none());
    }

    #[test]
    fn fails_if_date_format_is_not_a_string() {
        let error = parse_and_unwrap_error("[rules.foo]
                                           date_format = false");

        assert_eq!(format!("{}", error), "'date_format' should be a string");
        assert_eq!(error.description(), "TOML format error");
        assert!(error.cause().is_none());
    }

    #[test]
    fn matches_an_url() {
        let rules = parse_rules_from_str(r#"
        [rules.wordpress]
        include_url = "*//foo"
        "#)
            .expect("Failed to parse toml");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name(), "wordpress");
        // TODO
    }

    #[test]
    fn include_url_can_be_an_array() {
        let rules = parse_rules_from_str(r#"
        [rules.wordpress]
        include_url = ["*//foo", "*//bar"]
        "#)
            .expect("Failed to parse toml");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name(), "wordpress");
        // TODO
    }

    #[test]
    fn fails_if_root_has_error() {
        let error = parse_and_unwrap_error(r#"[rules.foo]
                                           root = "blih >""#);

        assert_eq!(format!("{}", error), "Failed to parse the selector");
        assert_eq!(error.description(), "Selector parser error");
        assert!(error.cause().is_none());
    }

}
