pub mod cli;
pub mod dump;
mod glyph;
pub mod shape;
pub mod subset;

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};

type BoxError = Box<dyn Error>;

#[derive(Debug)]
struct ErrorMessage(pub &'static str);

impl fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Error for ErrorMessage {}

pub(crate) fn read_file(path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
