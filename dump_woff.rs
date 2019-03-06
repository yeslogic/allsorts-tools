use encoding_rs::{Encoding, MACINTOSH, UTF_16BE};
use fontcode::error::ParseError;
use fontcode::opentype::tag;
use fontcode::read::ReadScope;
use fontcode::tables::NameTable;
use fontcode::woff::WoffFile;
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

    let fontfile = ReadScope::new(&buffer).read::<WoffFile>()?;

    match fontfile.flavor() {
        0x00010000 | 0x4F54544F /* OTTO */ => dump_ttf(fontfile.scope, fontfile),
        _ => Err(ParseError::BadVersion)
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn dump_ttf<'a>(scope: ReadScope<'a>, woff: WoffFile<'a>) -> Result<(), ParseError> {
    println!("TTF");
    println!(" - num_tables: {}\n", woff.table_directory.len());

    for entry in &woff.table_directory {
        println!(
            "{} (original checksum: 0x{:08x}, compressed length: {} original length: {})",
            DisplayTag(entry.tag),
            entry.orig_checksum,
            entry.comp_length,
            entry.orig_length
        );
        let _table = entry.read_table(scope)?;
    }

    let metadata = woff.extended_metadata()?;
    if let Some(metadata) = metadata {
        println!("\nExtended Metadata:\n{}", metadata);
    }

    println!();
    if let Some(entry) = woff
        .table_directory
        .iter()
        .find(|entry| entry.tag == tag('n', 'a', 'm', 'e'))
    {
        let table = entry.read_table(woff.scope)?;
        let name_table = table.scope().read::<NameTable>()?;

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
                (0, _, _) => decode(&UTF_16BE, name_data),
                (1, 0, _) => decode(&MACINTOSH, name_data),
                (3, 0, _) => decode(&UTF_16BE, name_data),
                (3, 1, _) => decode(&UTF_16BE, name_data),
                (3, 10, _) => decode(&UTF_16BE, name_data),
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

fn decode(encoding: &'static Encoding, data: &[u8]) -> String {
    let mut decoder = encoding.new_decoder();
    if let Some(size) = decoder.max_utf8_buffer_length(data.len()) {
        let mut s = String::with_capacity(size);
        let (_res, _read, _repl) = decoder.decode_to_string(data, &mut s, true);
        s
    } else {
        String::new() // can only happen if buffer is enormous
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
