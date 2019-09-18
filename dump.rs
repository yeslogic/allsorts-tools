use atty::Stream;
use encoding_rs::{Encoding, MACINTOSH, UTF_16BE};
use getopts::Options;

use allsorts::binary::read::ReadScope;
use allsorts::cff::{self, CFFVariant, Charset, FontDict, Operator, CFF};
use allsorts::error::ParseError;
use allsorts::fontfile::FontFile;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::{HeadTable, MaxpTable, NameTable, OffsetTable, OpenTypeFont, TTCHeader};
use allsorts::tag::{self, DisplayTag};
use allsorts::woff::WoffFile;
use allsorts::woff2::{Woff2File, Woff2GlyfTable, Woff2LocaTable};
use fontcode::font_tables;

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str::{self, FromStr};

type Tag = u32;

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Parse(ParseError),
    Message(&'static str),
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("c", "cff", "treat the file as a CFF font/table");
    opts.optopt("t", "table", "dump the content of this table", "TABLE");
    opts.optopt("i", "index", "index of the font to dump (for TTC)", "INDEX");
    opts.optopt("g", "glyph", "dump the specified glyph", "INDEX");
    opts.optflag("l", "loca", "print the loca table");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    let table = matches
        .opt_str("t")
        .map(|table| tag::from_string(&table))
        .transpose()?;
    if table.is_some() && atty::is(Stream::Stdout) {
        return Err(Error::Message("Not printing binary data to tty."));
    }

    let index = matches
        .opt_str("i")
        .map(|index| usize::from_str(&index))
        .transpose()
        .map_err(|_err| ParseError::BadValue)?
        .unwrap_or_default();

    let filename = if !matches.free.is_empty() {
        matches.free[0].as_str()
    } else {
        print_usage(&program, opts);
        return Ok(());
    };

    let buffer = read_file(filename)?;

    if matches.opt_present("l") {
        dump_loca_table(&buffer, index)?;
    } else if let Ok(Some(glyph_id)) = matches.opt_get::<u16>("g") {
        dump_glyph(&buffer, index, glyph_id)?;
    } else if matches.opt_present("c") {
        dump_cff_table(ReadScope::new(&buffer))?
    } else {
        match ReadScope::new(&buffer).read::<FontFile>()? {
            FontFile::OpenType(font_file) => match font_file.font {
                OpenTypeFont::Single(ttf) => dump_ttf(&font_file.scope, ttf, table)?,
                OpenTypeFont::Collection(ttc) => dump_ttc(font_file.scope, ttc, table)?,
            },
            FontFile::Woff(woff_file) => dump_woff(woff_file, table)?,
            FontFile::Woff2(woff_file) => {
                dump_woff2(woff_file.table_data_block_scope(), &woff_file, table, index)?
            }
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

fn dump_ttc<'a>(scope: ReadScope<'a>, ttc: TTCHeader<'a>, tag: Option<Tag>) -> Result<(), Error> {
    println!("TTC");
    println!(" - version: {}.{}", ttc.major_version, ttc.minor_version);
    println!(" - num_fonts: {}", ttc.offset_tables.len());
    println!();
    for offset_table_offset in &ttc.offset_tables {
        let offset_table_offset = usize::try_from(offset_table_offset).map_err(ParseError::from)?;
        let offset_table = scope.offset(offset_table_offset).read::<OffsetTable>()?;
        dump_ttf(&scope, offset_table, tag)?;
    }
    println!();
    Ok(())
}

fn dump_ttf<'a>(
    scope: &ReadScope<'a>,
    ttf: OffsetTable<'a>,
    tag: Option<Tag>,
) -> Result<(), Error> {
    if let Some(tag) = tag {
        return dump_raw_table(ttf.read_table(&scope, tag)?);
    }

    println!("TTF");
    println!(" - version: 0x{:08x}", ttf.sfnt_version);
    println!(" - num_tables: {}", ttf.table_records.len());
    println!();
    for table_record in &ttf.table_records {
        println!(
            "{} (checksum: 0x{:08x}, offset: {}, length: {})",
            DisplayTag(table_record.table_tag),
            table_record.checksum,
            table_record.offset,
            table_record.length
        );
        let _table = table_record.read_table(&scope)?;
    }
    if let Some(cff_table_data) = ttf.read_table(&scope, tag::CFF)? {
        println!();
        dump_cff_table(cff_table_data)?;
    }
    println!();
    if let Some(name_table_data) = ttf.read_table(&scope, tag::NAME)? {
        let name_table = name_table_data.read::<NameTable>()?;
        dump_name_table(&name_table)?;
    }
    Ok(())
}

fn dump_woff<'a>(woff: WoffFile<'a>, tag: Option<Tag>) -> Result<(), Error> {
    let scope = &woff.scope;
    if let Some(tag) = tag {
        if let Some(entry) = woff.table_directory.iter().find(|entry| entry.tag == tag) {
            let table = entry.read_table(&woff.scope)?;

            return dump_raw_table(Some(table.scope().clone()));
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
        let _table = entry.read_table(&scope)?;
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
        let table = entry.read_table(&woff.scope)?;
        let name_table = table.scope().read::<NameTable>()?;
        dump_name_table(&name_table)?;
    }

    Ok(())
}

fn dump_woff2<'a>(
    scope: ReadScope<'a>,
    woff: &Woff2File<'a>,
    tag: Option<Tag>,
    index: usize,
) -> Result<(), Error> {
    if let Some(tag) = tag {
        let table = woff.read_table(tag, index)?;
        return dump_raw_table(table.as_ref().map(|buf| buf.scope().clone()));
    }

    println!("TTF in WOFF2");
    println!(" - num tables: {}", woff.table_directory.len());
    if let Some(collection_directory) = &woff.collection_directory {
        println!(" - num fonts: {}", collection_directory.fonts().count());
    }
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

    if let Some(entry) = woff.find_table_entry(tag::GLYF, index) {
        println!();
        let table = entry.read_table(&scope)?;
        let head = woff
            .read_table(tag::HEAD, index)?
            .ok_or(ParseError::BadValue)?
            .scope()
            .read::<HeadTable>()?;
        let maxp = woff
            .read_table(tag::MAXP, index)?
            .ok_or(ParseError::BadValue)?
            .scope()
            .read::<MaxpTable>()?;
        let loca_entry = woff
            .find_table_entry(tag::LOCA, index)
            .ok_or(ParseError::BadValue)?;
        let loca = loca_entry.read_table(&woff.table_data_block_scope())?;
        let loca = loca.scope().read_dep::<Woff2LocaTable>((
            &loca_entry,
            usize::from(maxp.num_glyphs),
            head.index_to_loc_format,
        ))?;
        let glyf = table.scope().read_dep::<Woff2GlyfTable>((&entry, &loca))?;

        println!("Read glyf table with {} glyphs:", glyf.records.len());
        for glyph in glyf.records {
            println!("- {:?}", glyph);
        }
    }

    if let Some(table) = woff.read_table(tag::NAME, index)? {
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

fn dump_loca_table(buffer: &[u8], index: usize) -> Result<(), ParseError> {
    let font = font_tables::FontImpl::new(&buffer, index).unwrap();
    let provider = font_tables::FontTablesImpl::FontImpl(font);

    let table = provider.get_table(tag::HEAD).expect("no head table");
    let scope = ReadScope::new(table.borrow());
    let head = scope.read::<HeadTable>()?;

    let table = provider.get_table(tag::MAXP).expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let table = provider.get_table(tag::LOCA).expect("no loca table");
    let scope = ReadScope::new(table.borrow());
    let loca =
        scope.read_dep::<LocaTable>((usize::from(maxp.num_glyphs), head.index_to_loc_format))?;

    println!("loca:");
    for (glyph_id, offset) in loca.offsets.iter().enumerate() {
        println!("{}: {}", glyph_id, offset);
    }

    Ok(())
}

fn dump_cff_table<'a>(scope: ReadScope<'a>) -> Result<(), ParseError> {
    let cff = scope.read::<CFF>()?;

    println!("- CFF:");
    println!(" - version: {}.{}", cff.header.major, cff.header.minor);
    for obj in cff.name_index.iter() {
        let name = String::from_utf8_lossy(obj);
        println!(" - name: {}", name);
    }

    if cff.name_index.count != 1 {
        return Err(ParseError::BadIndex);
    }
    let font = cff.fonts.get(0).ok_or(ParseError::MissingValue)?;
    let char_strings_offset = font
        .top_dict
        .get_i32(Operator::CharStrings)
        .ok_or(ParseError::MissingValue)??;
    let char_strings_index = scope
        .offset(usize::try_from(char_strings_offset)?)
        .read::<cff::Index<'_>>()?;
    println!(" - num glyphs: {}", char_strings_index.count);
    println!(
        " - charset: {}",
        match font.charset {
            Charset::ISOAdobe => "ISO Adobe",
            Charset::Expert => "Expert",
            Charset::ExpertSubset => "Expert Subset",
            Charset::Custom(_) => "Custom",
        }
    );
    println!(
        " - variant: {}",
        match font.data {
            CFFVariant::CID(_) => "CID",
            CFFVariant::Type1(_) => "Type 1",
        }
    );
    println!();
    println!(" - Top DICT");
    for (op, operands) in font.top_dict.iter() {
        println!("  - {:?}: {:?}", op, operands);
    }
    match &font.data {
        CFFVariant::Type1(ref type1) => {
            println!();
            println!(
                " - encoding: {}",
                match type1.encoding {
                    cff::Encoding::Standard => "Standard",
                    cff::Encoding::Expert => "Expert",
                    cff::Encoding::Custom(_) => "Custom",
                }
            );
            println!();
            println!(" - Private DICT");
            for (op, operands) in type1.private_dict.iter() {
                println!("  - {:?}: {:?}", op, operands);
            }
            let (subrs_count, subrs_size) = match type1.local_subr_index {
                Some(ref index) => (index.count, index.data_len()),
                None => (0, 0),
            };
            println!(" - Local subrs: {} ({} bytes)", subrs_count, subrs_size);
        }
        CFFVariant::CID(cid) => {
            for (i, object) in cid.font_dict_index.iter().enumerate() {
                println!();
                println!(" - Font DICT {}", i);
                let font_dict = ReadScope::new(object).read::<FontDict>()?;
                for (op, operands) in font_dict.iter() {
                    println!("  - {:?}: {:?}", op, operands);
                }

                println!();
                println!("  - Private DICT");
                let (private_dict, _private_dict_offset) = font_dict.read_private_dict(&scope)?;
                for (op, operands) in private_dict.iter() {
                    println!("   - {:?}: {:?}", op, operands);
                }
            }
            let (subrs_count, subrs_size) =
                cid.local_subr_indices
                    .iter()
                    .fold((0, 0), |(mut count, mut size), index| {
                        if let Some(index) = index {
                            count += index.count;
                            size += index.data_len();
                        }
                        (count, size)
                    });
            println!();
            println!(
                " - Local subrs: {} ({} bytes) in {} indices",
                subrs_count,
                subrs_size,
                cid.local_subr_indices.len()
            );
        }
    }
    println!(
        " - Global subrs: {} ({} bytes)",
        cff.global_subr_index.count,
        cff.global_subr_index.data_len()
    );

    Ok(())
}

fn dump_glyph(buffer: &[u8], index: usize, glyph_id: u16) -> Result<(), ParseError> {
    let font = font_tables::FontImpl::new(&buffer, index).unwrap();
    let provider = font_tables::FontTablesImpl::FontImpl(font);

    let table = provider.get_table(tag::HEAD).expect("no head table");
    let scope = ReadScope::new(table.borrow());
    let head = scope.read::<HeadTable>()?;

    let table = provider.get_table(tag::MAXP).expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let table = provider.get_table(tag::LOCA).expect("no loca table");
    let scope = ReadScope::new(table.borrow());
    let loca =
        scope.read_dep::<LocaTable>((usize::from(maxp.num_glyphs), head.index_to_loc_format))?;

    let table = provider.get_table(tag::GLYF).expect("no glyf table");
    let scope = ReadScope::new(table.borrow());
    let glyf = scope.read_dep::<GlyfTable>(&loca)?;

    let glyph = glyf
        .records
        .get(usize::from(glyph_id))
        .ok_or(ParseError::BadValue)?;
    println!("{:#?}", glyph.clone().parse()?);

    Ok(())
}

fn dump_raw_table(scope: Option<ReadScope>) -> Result<(), Error> {
    if let Some(scope) = scope {
        io::stdout().write_all(scope.data()).map_err(Error::from)
    } else {
        Err(Error::Message("Table not found"))
    }
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

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Parse(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}
