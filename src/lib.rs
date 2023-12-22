pub mod bitmaps;
pub mod cli;
pub mod cmap;
pub mod dump;
mod glyph;
pub mod has_table;
pub mod instance;
pub mod layout_features;
mod script;
pub mod shape;
pub mod subset;
pub mod svg;
pub mod validate;
pub mod variations;
pub mod view;
mod writer;

use std::error::Error;
use std::fmt;
use std::num::ParseFloatError;

use allsorts::binary::read::ReadScope;
use allsorts::error::ParseError;
use allsorts::tables::variable_fonts::avar::AvarTable;
use allsorts::tables::variable_fonts::fvar::FvarTable;
use allsorts::tables::variable_fonts::OwnedTuple;
use allsorts::tables::{Fixed, FontTableProvider};
use allsorts::tag;
use encoding_rs::Encoding;

pub type BoxError = Box<dyn Error>;

#[derive(Debug)]
struct ErrorMessage(pub &'static str);

impl fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Error for ErrorMessage {}

/// Decode a non-utf-8 string to a UTF-8 Rust string.
pub(crate) fn decode(encoding: &'static Encoding, data: &[u8]) -> String {
    let mut decoder = encoding.new_decoder();
    if let Some(size) = decoder.max_utf8_buffer_length(data.len()) {
        let mut s = String::with_capacity(size);
        let (_res, _read, _repl) = decoder.decode_to_string(data, &mut s, true);
        s
    } else {
        String::new() // can only happen if buffer is enormous
    }
}

fn parse_tuple(tuple: &str) -> Result<Vec<Fixed>, ParseFloatError> {
    tuple
        .split(',')
        .map(|s| s.trim().parse::<f32>().map(Fixed::from))
        .collect()
}

fn normalise_tuple(
    provider: &impl FontTableProvider,
    tuple: &[Fixed],
) -> Result<OwnedTuple, ParseError> {
    let fvar_data = provider
        .table_data(tag::FVAR)?
        .ok_or(ParseError::MissingValue)?;
    let fvar = ReadScope::new(&fvar_data).read::<FvarTable>()?;
    let avar_data = provider.table_data(tag::AVAR)?;
    let avar = avar_data
        .as_ref()
        .map(|data| ReadScope::new(data).read::<AvarTable>())
        .transpose()?;

    fvar.normalize(tuple.iter().copied(), avar.as_ref())
}
