use atty::Stream;
use encoding_rs::{Encoding, MACINTOSH, UTF_16BE};
use getopts::Options;

use fontcode::error::ParseError;
use fontcode::fontfile::FontFile;
use fontcode::read::ReadScope;
use fontcode::tables::{HeadTable, MaxpTable, NameTable, OffsetTable, OpenTypeFont, TTCHeader};
use fontcode::tag;
use fontcode::woff::WoffFile;
use fontcode::woff2::{Woff2File, Woff2GlyfTable, Woff2LocaTable};

use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str;
use std::{env, process};

type Tag = u32;

fn main() -> Result<(), ParseError> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("t", "table", "dump the content of this table", "TABLE");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    let table = matches.opt_str("t").map(|table| {
        let mut tag = [0u8; 4];
        for (i, byte) in table.bytes().take(4).enumerate() {
            tag[i] = byte;
        }
        tag::tag(tag)
    });

    if table.is_some() && atty::is(Stream::Stdout) {
        eprintln!("Not printing binary data to tty.");
        process::exit(1);
    }

    let filename = if !matches.free.is_empty() {
        matches.free[0].as_str()
    } else {
        print_usage(&program, opts);
        return Ok(());
    };

    let buffer = read_file(filename)?;

    match ReadScope::new(&buffer).read::<FontFile>()? {
        FontFile::OpenType(font_file) => match font_file.font {
            OpenTypeFont::Single(ttf) => dump_ttf(font_file.scope, ttf, table)?,
            OpenTypeFont::Collection(ttc) => dump_ttc(font_file.scope, ttc, table)?,
        },
        FontFile::Woff(woff_file) => dump_woff(woff_file.scope, woff_file, table)?,
        FontFile::Woff2(woff_file) => {
            dump_woff2(woff_file.table_data_block_scope(), &woff_file, table)?
        }
    }

    Ok(())
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] FONTFILE ", program);
    eprint!("{}", opts.usage(&brief));
}

fn read_file(path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn dump_ttc<'a>(
    scope: ReadScope<'a>,
    ttc: TTCHeader<'a>,
    tag: Option<Tag>,
) -> Result<(), ParseError> {
    println!("TTC");
    println!(" - version: {}.{}", ttc.major_version, ttc.minor_version);
    println!(" - num_fonts: {}", ttc.offset_tables.len());
    println!();
    for offset_table_offset in &ttc.offset_tables {
        let offset_table_offset = offset_table_offset as usize; // FIXME range
        let offset_table = scope.offset(offset_table_offset).read::<OffsetTable>()?;
        dump_ttf(scope, offset_table, tag)?;
    }
    println!();
    Ok(())
}

fn dump_ttf<'a>(
    scope: ReadScope<'a>,
    ttf: OffsetTable<'a>,
    tag: Option<Tag>,
) -> Result<(), ParseError> {
    if let Some(tag) = tag {
        return dump_raw_table(tag, ttf.read_table(scope, tag)?).map_err(ParseError::from);
    }

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
    if let Some(name_table_data) = ttf.read_table(scope, tag::NAME)? {
        let name_table = name_table_data.read::<NameTable>()?;
        dump_name_table(&name_table)?;
    }
    Ok(())
}

fn dump_woff<'a>(
    scope: ReadScope<'a>,
    woff: WoffFile<'a>,
    tag: Option<Tag>,
) -> Result<(), ParseError> {
    if let Some(tag) = tag {
        if let Some(entry) = woff.table_directory.iter().find(|entry| entry.tag == tag) {
            let table = entry.read_table(woff.scope)?;

            return dump_raw_table(tag, Some(table.scope())).map_err(ParseError::from);
        } else {
            eprintln!("Table {} not found", DisplayTag(tag));
        }

        return Ok(());
    }

    println!("TTF in WOFF");
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
        .find(|entry| entry.tag == tag::NAME)
    {
        let table = entry.read_table(woff.scope)?;
        let name_table = table.scope().read::<NameTable>()?;
        dump_name_table(&name_table)?;
    }

    Ok(())
}

fn dump_woff2<'a>(
    scope: ReadScope<'a>,
    woff: &Woff2File<'a>,
    tag: Option<Tag>,
) -> Result<(), ParseError> {
    if let Some(tag) = tag {
        let table = woff.read_table(tag::NAME)?;
        return dump_raw_table(tag, table.as_ref().map(|buf| buf.scope())).map_err(ParseError::from);
    }

    println!("TTF in WOFF2");
    println!(" - num_tables: {}", woff.table_directory.len());
    println!(
        " - sizeof font data: {} compressed {} uncompressed\n",
        woff.woff_header.total_compressed_size,
        woff.table_data_block.len()
    );

    for entry in &woff.table_directory {
        println!("{} {:?}", DisplayTag(entry.tag), entry,);
    }

    let metadata = woff.extended_metadata()?;
    if let Some(metadata) = metadata {
        println!("\nExtended Metadata:\n{}", metadata);
    }

    if let Some(entry) = woff.find_table_entry(tag::GLYF) {
        println!();
        let table = entry.read_table(scope)?;
        let head = woff
            .read_table(tag::HEAD)?
            .ok_or(ParseError::BadValue)?
            .scope()
            .read::<HeadTable>()?;
        let maxp = woff
            .read_table(tag::MAXP)?
            .ok_or(ParseError::BadValue)?
            .scope()
            .read::<MaxpTable>()?;
        let loca_entry = woff
            .find_table_entry(tag::LOCA)
            .ok_or(ParseError::BadValue)?;
        let loca = loca_entry.read_table(woff.table_data_block_scope())?;
        let loca = loca.scope().read_dep::<Woff2LocaTable>((
            &loca_entry,
            usize::from(maxp.num_glyphs),
            head.index_to_loc_format,
        ))?;
        let glyf = table.scope().read_dep::<Woff2GlyfTable>((&entry, loca))?;

        println!("Read glyf table with {} glyphs:", glyf.records.len());
        for glyph in glyf.records {
            println!("- {:?}", glyph);
        }
    }

    if let Some(table) = woff.read_table(tag::NAME)? {
        println!();
        let name_table = table.scope().read::<NameTable>()?;
        dump_name_table(&name_table)?;
    }

    Ok(())
}

fn dump_name_table(name_table: &NameTable) -> Result<(), ParseError> {
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

    Ok(())
}

fn dump_raw_table(tag: Tag, scope: Option<ReadScope>) -> Result<(), io::Error> {
    if let Some(scope) = scope {
        io::stdout().write_all(scope.data())?;
    } else {
        eprintln!("Table {} not found", DisplayTag(tag));
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
