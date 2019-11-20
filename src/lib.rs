pub mod cli;
pub mod dump;
mod glyph;
pub mod shape;
pub mod subset;

use std::error::Error;
use std::fmt;

type BoxError = Box<dyn Error>;

#[derive(Debug)]
struct ErrorMessage(pub &'static str);

impl fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Error for ErrorMessage {}
