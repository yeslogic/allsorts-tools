use allsorts::binary::read::ReadScope;

use allsorts::fontfile::FontFile;

use allsorts::tables::FontTableProvider;
use allsorts::tag::{self};

use crate::cli::HasTableOpts;
use crate::BoxError;

pub fn main(opts: HasTableOpts) -> Result<i32, BoxError> {
    let table = tag::from_string(&opts.table)?;
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontFile>()?;
    let table_provider = font_file.table_provider(opts.index)?;
    if table_provider.has_table(table) {
        if opts.print_file {
            println!("{}", opts.font);
        }
        Ok(0)
    } else {
        Ok(1)
    }
}
