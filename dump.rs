use fontcode::error::ParseError;
use fontcode::read::ReadScope;
use fontcode::tables::{OffsetTable, OpenTypeFile, OpenTypeFont, TTCHeader};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};

fn main() -> Result<(), ParseError> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: dump FILE");
        return Ok(());
    }

    let filename = &args[1];
    let buffer = read_file(filename)?;

    let fontfile = ReadScope::new(&buffer).read::<OpenTypeFile>()?;

    match fontfile.font {
        OpenTypeFont::Single(ttf) => dump_ttf(fontfile.scope, ttf)?,
        OpenTypeFont::Collection(ttc) => dump_ttc(fontfile.scope, ttc)?,
    }

    Ok(())
}

fn read_file(path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn dump_ttc<'a>(scope: ReadScope<'a>, ttc: TTCHeader<'a>) -> Result<(), ParseError> {
    println!("TTC");
    println!(" - version: {}.{}", ttc.major_version, ttc.minor_version);
    println!(" - num_fonts: {}", ttc.offset_tables.len());
    println!();
    for offset_table_offset in &ttc.offset_tables {
        let offset_table_offset = offset_table_offset as usize; // FIXME range
        let offset_table = scope.offset(offset_table_offset).read::<OffsetTable>()?;
        dump_ttf(scope, offset_table)?;
    }
    println!();
    Ok(())
}

fn dump_ttf<'a>(scope: ReadScope<'a>, ttf: OffsetTable<'a>) -> Result<(), ParseError> {
    println!("TTF");
    println!(" - version: 0x{:08x}", ttf.sfnt_version);
    println!(" - num_tables: {}", ttf.table_records.len());
    println!();
    for table_record in &ttf.table_records {
        println!(
            "{} (checksum: 0x{:08x}, length: {})",
            DisplayTag(table_record.table_tag),
            table_record.checksum,
            table_record.length
        );
        let _table = table_record.read_table(scope)?;
    }
    println!();
    Ok(())
}

struct DisplayTag(u32);

impl fmt::Display for DisplayTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tag = self.0;
        let mut s = String::with_capacity(4);
        s.push(char::from((tag >> 24) as u8));
        s.push(char::from(((tag >> 16) & 255) as u8));
        s.push(char::from(((tag >> 8) & 255) as u8));
        s.push(char::from((tag & 255) as u8));
        if s.chars().any(|c| !c.is_ascii() || c.is_ascii_control()) {
            write!(f, "0x{:08x}", tag)
        } else {
            s.fmt(f)
        }
    }
}
