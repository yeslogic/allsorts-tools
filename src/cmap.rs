use allsorts::binary::read::ReadScope;
use allsorts::error::ParseError;
use allsorts::font::Encoding;
use allsorts::font_data::FontData;
use allsorts::tables::cmap::CmapSubtable;
use allsorts::tables::FontTableProvider;
use allsorts::Font;

use crate::cli::CmapOpts;
use crate::BoxError;

pub fn main(opts: CmapOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let table_provider = font_file.table_provider(opts.index)?;
    let mut font = match Font::new(Box::new(table_provider))? {
        Some(font) => font,
        None => {
            eprintln!("unable to find suitable cmap subtable");
            return Ok(1);
        }
    };
    let failed = dump_cmap(&mut font)?;
    if failed {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn dump_cmap<T: FontTableProvider>(font: &mut Font<T>) -> Result<bool, ParseError> {
    let cmap_subtable = ReadScope::new(font.cmap_subtable_data()).read::<CmapSubtable<'_>>()?;
    let encoding = font.cmap_subtable_encoding;

    println!("cmap sub-table encoding: {:?}", encoding);
    cmap_subtable.mappings_fn(|ch, gid| match encoding {
        Encoding::Unicode => {
            let chr = std::char::from_u32(ch).and_then(|chr| {
                if chr.is_ascii_control() {
                    std::char::from_u32(ch + 0x2400)
                } else {
                    Some(chr)
                }
            });
            match chr {
                Some(code) if code.is_control() => println!("    U+{:04X} -> {}", ch, gid),
                Some(code) => println!("'{}' U+{:04X} -> {}", code, ch, gid),
                None => println!("{} -> {}", ch, gid),
            }
        }
        Encoding::Symbol | Encoding::AppleRoman | Encoding::Big5 => println!("{} -> {}", ch, gid),
    })?;

    Ok(true)
}
