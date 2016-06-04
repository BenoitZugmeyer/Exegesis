pub mod html;

use std::fmt;
use part::{Document, Part};

pub trait Formatter {
    fn write_document<T: fmt::Write>(&self, &Document, &mut T) -> Result<(), fmt::Error>;
    fn write_part<T: fmt::Write>(&self, &Part, &mut T) -> Result<(), fmt::Error>;

    fn format(&self, document: &Document) -> Result<String, fmt::Error> {
        let mut result = String::new();
        self.write_document(document, &mut result)?;
        Ok(result)
    }

    fn write_parts<T: fmt::Write>(&self, children: &[Part], output: &mut T) -> fmt::Result {
        for child in children {
            self.write_part(child, output)?;
        }
        Ok(())
    }
}
