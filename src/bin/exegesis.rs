#![feature(question_mark)]
extern crate clap;
extern crate toml;
extern crate exegesis;
extern crate hyper;
extern crate serde;

use clap::{App, Arg};
use std::fs;
use std::io;
use std::io::{Read, Write};
use std::fmt::Write as FmtWrite;
use std::path::Path;
use std::process;
use exegesis::{Website, HtmlFormatter, Rules};

macro_rules! error(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut io::stderr(), $($arg)*);
        r.expect("Failed printing to stderr");
        process::exit(1);
    } }
);

fn read_file(path: &Path) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut result = String::new();
    file.read_to_string(&mut result)?;
    Ok(result)
}

fn download(url: &str) -> Result<Website, hyper::Error> {
    let client = hyper::Client::new();

    let request = client.get(url)
        .header(hyper::header::Connection::close());

    let response = request.send()?;

    Ok(Website::from_response(url.to_string(), response))
}

fn main() {
    let matches = App::new("Exegesis")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Benoît Zugmeyer <bzugmeyer@gmail.com>")
        .about("Extract and format the content of Web pages")
        .arg(Arg::with_name("rules")
            .short("r")
            .long("rules")
            .value_name("FILE")
            .help("Rule files")
            .multiple(true)
            .number_of_values(1)
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("URL")
            .help("Sets the input file to use")
            .required(true))
        .get_matches();

    let mut rules = Rules::default();

    for value in matches.values_of_os("rules").unwrap() {
        let source = match read_file(Path::new(value)) {
            Err(error) => error!("Error while reading '{}': {}", value.to_string_lossy(), error),
            Ok(s) => s,
        };

        let mut parser = toml::Parser::new(&source);
        let table = match parser.parse() {
            None => {
                let mut errors = String::new();
                for error in &parser.errors {
                    let (line, column) = parser.to_linecol(error.lo);
                    write!(errors, "{}:{}  {}", line + 1, column + 1, error.desc)
                        .expect("Failed to write in a string buffer");
                }

                error!("Error while parsing '{}' as TOML:\n{}",
                       value.to_string_lossy(), errors);
            }
            Some(t) => t,
        };

        let mut decoder = toml::Decoder::new(toml::Value::Table(table));
        let new_rules = match serde::Deserialize::deserialize(&mut decoder) {
            Err(error) => {
                error!("Error while decoding '{}' rules: {}", value.to_string_lossy(), error)
            }
            Ok(r) => r,
        };

        rules.append(new_rules);
    }

    let url = matches.value_of("URL").unwrap();
    let website = match download(url) {
        Err(error) => error!("Error while loading '{}': {}", url, error),
        Ok(w) => w,
    };

    let docs = match rules.extract(&website) {
        Err(error) => error!("Error while extracting '{}': {}", url, error),
        Ok(d) => d,
    };

    HtmlFormatter::default()
        .write_full(&docs, &mut io::stdout())
        .expect("Failed printing to stdout");
}
