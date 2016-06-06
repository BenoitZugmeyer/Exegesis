#![feature(question_mark)]
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde;

#[macro_use]
extern crate mime;
extern crate hyper;
extern crate regex;
extern crate kuchiki;
extern crate html5ever;
extern crate chrono;

#[cfg(test)]mod mock;
mod rule;
mod rules;
mod matcher;
mod website;
mod extractor;
mod date;
mod part;
mod formatter;

pub use website::Website;
pub use rules::Rules;
pub use formatter::Formatter;
pub use formatter::html::HtmlFormatter;
pub use formatter::json::JsonFormatter;
