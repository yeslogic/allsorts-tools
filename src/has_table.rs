use allsorts::binary::read::ReadScope;

use allsorts::font_data::FontData;

use allsorts::tables::FontTableProvider;
use allsorts::tag::{self};

use crate::cli::HasTableOpts;
use crate::BoxError;

pub fn main(opts: HasTableOpts) -> Result<i32, BoxError> {
    let table = tag::from_string(&opts.table)?;
    let mut found = false;
    for path in opts.fonts {
        let buffer = std::fs::read(&path)?;
        let scope = ReadScope::new(&buffer);
        let font_file = scope.read::<FontData>()?;
        let table_provider = font_file.table_provider(opts.index)?;
        let has_table = if opts.invert_match {
            !table_provider.has_table(table)
        } else {
            table_provider.has_table(table)
        };
        found |= has_table;
        if has_table && opts.print_file {
            println!("{}", path.to_string_lossy());
        }
    }
    Ok(if found { 0 } else { 1 })
}
