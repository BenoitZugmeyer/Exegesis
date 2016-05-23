#![feature(question_mark)]

extern crate hyper;
extern crate exegesis;

use exegesis::{Website, toml, extract, Formatter, HtmlFormatter};

use hyper::{Client, Error as HyperError};
use hyper::header;

fn download(url: &str) -> Result<Website, HyperError> {
    let client = Client::new();

    let request = client.get(url)
        .header(header::Connection::close());

    let response = request.send()?;

    Ok(Website::from_response(url.to_string(), response))
}

fn main() {
    let rules = toml::parse_rules_from_str(r#"
[rules.rustlang_blog]
include_url = "*//blog.rust-lang.org/**"
date_format = "%B %d, %Y"

title = ".post-title"
date = ".post-meta"
paragraph = ".post-content p"
header1 = ".post-content h3"
list = ".post-content ul"
list-item = ".post-content li"
link = ".post-content a"
emphasis = "strong"
image = "img"
    "#)
        .unwrap();

    let website = download("http://blog.rust-lang.org/2016/05/16/rust-at-one-year.html").unwrap();
    println!("{}",
             HtmlFormatter {}.format(&extract(&rules, &website).unwrap()).unwrap());
}
