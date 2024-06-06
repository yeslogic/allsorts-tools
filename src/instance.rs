use std::fs::File;
use std::io::Write;

use allsorts::binary::read::ReadScope;
use allsorts::font_data::FontData;

use crate::cli::InstanceOpts;
use crate::{parse_tuple, BoxError};

pub fn main(opts: InstanceOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let provider = font_file.table_provider(opts.index)?;

    let user_instance = parse_tuple(&opts.tuple)?;
    let (new_font, _tuple) = allsorts::variations::instance(&provider, &user_instance)?;

    // Write out the new font
    let mut output = File::create(&opts.output)?;
    output.write_all(&new_font)?;

    Ok(0)
}
