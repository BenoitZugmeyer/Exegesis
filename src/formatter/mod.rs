pub mod html;

use std::io;
use part::{Document, Part};

pub trait Formatter {
    fn write_document<T: io::Write>(&self, &Document, &mut T) -> Result<(), io::Error>;
    fn write_part<T: io::Write>(&self, &Part, &mut T) -> Result<(), io::Error>;

    fn format(&self, document: &Document) -> Result<String, io::Error> {
        let mut result = Vec::new();
        self.write_document(document, &mut result)?;
        Ok(String::from_utf8(result).unwrap())
    }

    fn write_parts<T: io::Write>(&self, children: &[Part], output: &mut T) -> io::Result<()> {
        for child in children {
            self.write_part(child, output)?;
        }
        Ok(())
    }
}
