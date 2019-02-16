use fontcode::error::ParseError;
use fontcode::opentype::tag;
use fontcode::read::ReadScope;
use fontcode::tables::{NameTable, OffsetTable, OpenTypeFile, OpenTypeFont, TTCHeader};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::str;

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
    if let Some(name_table_data) = ttf.read_table(scope, tag('n', 'a', 'm', 'e'))? {
        let name_table = name_table_data.read::<NameTable>()?;
        for name_record in &name_table.name_records {
            let platform = name_record.platform_id;
            let encoding = name_record.encoding_id;
            let language = name_record.language_id;
            let offset = usize::from(name_record.offset);
            let length = usize::from(name_record.length);
            let name_data = name_table
                .string_storage
                .offset_length(offset, length)?
                .data();
            let name = match (platform, encoding, language) {
                (0, _, _) => get_utf16be(name_data),
                (1, 0, _) => get_utf8(name_data), // FIXME MacRoman
                (3, 0, _) => get_utf16be(name_data),
                (3, 1, _) => get_utf16be(name_data),
                (3, 10, _) => get_utf16be(name_data),
                _ => format!(
                    "(unknown platform={} encoding={} language={})",
                    platform, encoding, language
                ),
            };
            match get_name_meaning(name_record.name_id) {
                Some(meaning) => println!("{}", meaning,),
                None => println!("name {}", name_record.name_id,),
            }
            println!("{:?}", name);
            println!();
        }
    }
    Ok(())
}

fn get_utf8(data: &[u8]) -> String {
    match str::from_utf8(data) {
        Ok(s) => String::from(s),
        Err(_) => format!("(not UTF-8: {:?})", data),
    }
}

fn get_utf16be(data: &[u8]) -> String {
    if data.len() & 1 == 0 {
        let u16buf: Vec<u16> = data
            .chunks(2)
            .map(|x| (u16::from(x[0]) << 8) | u16::from(x[1]))
            .collect();
        match String::from_utf16(u16buf.as_slice()) {
            Ok(s) => s,
            Err(_) => String::from("(not UTF-16BE)"),
        }
    } else {
        String::from("(not UTF-16BE)")
    }
}

fn get_name_meaning(name_id: u16) -> Option<&'static str> {
    match name_id {
        0 => Some("Copyright"),
        1 => Some("Font Family"),
        2 => Some("Font Subfamily"),
        3 => Some("Unique Identifier"),
        4 => Some("Full Font Name"),
        5 => Some("Version"),
        6 => Some("PostScript Name"),
        7 => Some("Trademark"),
        8 => Some("Manufacturer"),
        9 => Some("Designer"),
        10 => Some("Description"),
        11 => Some("URL Vendor"),
        12 => Some("URL Designer"),
        13 => Some("License Description"),
        14 => Some("License Info URL"),
        15 => None, // Reserved
        16 => Some("Typographic Family"),
        17 => Some("Typographic Subfamily"),
        18 => Some("Compatible Full"),
        19 => Some("Sample Text"),
        20 => Some("PostScript CID findfont"),
        21 => Some("WWS Family Name"),
        22 => Some("WWS Subfamily Name"),
        23 => Some("Light Background Palette"),
        24 => Some("Dark Background Palette"),
        25 => Some("Variations PostScript Name Prefix"),
        _ => None,
    }
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
