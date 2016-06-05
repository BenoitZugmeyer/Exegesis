pub mod html;
pub mod json;

use std::io;
use std::error;
use part::Document;

pub trait Formatter {
    fn write_document<T: io::Write>(&self, &Document, &mut T) -> Result<(), Box<error::Error>>;

    fn format(&self, document: &Document) -> Result<String, Box<error::Error>> {
        let mut result = Vec::new();
        self.write_document(document, &mut result)?;
        Ok(String::from_utf8(result).unwrap())
    }
}
