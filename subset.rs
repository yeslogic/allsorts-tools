use encoding_rs::{Encoding, MACINTOSH, UTF_16BE};
use getopts::Options;

use fontcode::error::{ParseError, ReadWriteError};
use fontcode::fontfile::FontFile;
use fontcode::read::ReadScope;
use fontcode::tables::{HeadTable, MaxpTable, NameTable, OffsetTable, OpenTypeFile, OpenTypeFont};
use fontcode::woff::WoffFile;
use fontcode::woff2::{Woff2File, Woff2GlyfTable, Woff2LocaTable};
use fontcode::{macroman, tag};

use fontcode::glyph_index::read_cmap_subtable;
use fontcode::gsub::{GlyphOrigin, RawGlyph};
use fontcode::tables::cmap::{Cmap, CmapSubtable};
use itertools::Itertools;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str;

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Parse(ParseError),
    ReadWrite(ReadWriteError),
    Message(&'static str),
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt(
        "t",
        "text",
        "subset the font to include glyphs from text",
        "TEXT",
    );
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    let text = matches
        .opt_str("t")
        .ok_or(Error::Message("-t TEXT is required"))?;

    if matches.free.len() < 2 {
        print_usage(&program, opts);
        return Ok(());
    }

    let input = matches.free[0].as_str();
    let output = matches.free[1].as_str();

    // Work out the glyphs we want to keep from the text
    let buffer = read_file(input)?;

    match ReadScope::new(&buffer).read::<FontFile>()? {
        FontFile::OpenType(font_file) => subset_ttf(&font_file, &text, output)?,
        FontFile::Woff(woff_file) => subset_woff(woff_file.scope, woff_file)?,
        FontFile::Woff2(woff_file) => subset_woff2(woff_file.table_data_block_scope(), &woff_file)?,
    }

    Ok(())
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] INPUT OUTPUT ", program);
    eprint!("{}", opts.usage(&brief));
}

fn read_file(path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn subset_ttf<'a>(
    font_file: &'a OpenTypeFile<'a>,
    text: &str,
    output_path: &str,
) -> Result<(), Error> {
    let script = tag::DFLT; // FIXME
    let lang = tag::LATN; // FIXME
    let ttf = match &font_file.font {
        OpenTypeFont::Single(ttf) => ttf,
        OpenTypeFont::Collection(_ttc) => unimplemented!(),
    };

    let glyphs = chars_to_glyphs(font_file.scope, &ttf, script, lang, text)?;
    let mut glyph_ids = glyphs
        .iter()
        .flat_map(|glyph| glyph.as_ref().and_then(|raw_glyph| raw_glyph.glyph_index))
        .collect::<Vec<_>>();
    glyph_ids.sort();
    let glyph_ids = glyph_ids.into_iter().dedup().collect::<Vec<_>>();
    if glyph_ids.is_empty() {
        return Err(Error::Message("no glyphs left in font"));
    }

    println!("Number of glyphs in new font: {}", glyph_ids.len());

    // Subset
    let cmap0 = if glyphs.iter().all(is_macroman) {
        let mut cmap0 = [0; 256];
        glyphs
            .iter()
            .enumerate()
            .for_each(|(glyph_index, glyph)| match glyph {
                Some(RawGlyph {
                    glyph_origin: GlyphOrigin::Char(chr),
                    ..
                }) => {
                    cmap0[usize::from(macroman::char_to_macroman(*chr).unwrap())] =
                        glyph_index as u8
                }
                _ => unreachable!(),
            });
        Some(Box::new(cmap0))
    } else {
        // TODO: Handle this
        return Err(Error::Message("not mac roman compatible"));
    };

    let new_font = font_file.subset(&glyph_ids, cmap0)?;

    // Write out the new font
    let mut output = File::create(output_path)?;
    output.write_all(new_font.bytes())?;

    Ok(())
}

fn subset_woff<'a>(scope: ReadScope<'a>, woff: WoffFile<'a>) -> Result<(), Error> {
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

fn subset_woff2<'a>(scope: ReadScope<'a>, woff: &Woff2File<'a>) -> Result<(), Error> {
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

fn chars_to_glyphs<'a>(
    scope: ReadScope<'a>,
    ttf: &OffsetTable<'a>,
    _script: u32,
    _lang: u32,
    text: &str,
) -> Result<Vec<Option<RawGlyph<()>>>, Error> {
    let cmap = ttf
        .read_table(scope, tag::CMAP)?
        .ok_or(Error::Message("no cmap table"))?
        .read::<Cmap>()?;
    let cmap_subtable =
        read_cmap_subtable(&cmap)?.ok_or(Error::Message("no suitable cmap sub-table found"))?;

    let glyphs = text
        .chars()
        .map(|ch| map_glyph(&cmap_subtable, ch))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(glyphs)
}

fn map_glyph(cmap_subtable: &CmapSubtable, ch: char) -> Result<Option<RawGlyph<()>>, ParseError> {
    if let Some(glyph_index) = cmap_subtable.map_glyph(ch as u32)? {
        let glyph = make_glyph(ch, glyph_index);
        Ok(Some(glyph))
    } else {
        Ok(None)
    }
}

fn make_glyph(ch: char, glyph_index: u16) -> RawGlyph<()> {
    RawGlyph {
        unicodes: vec![ch],
        glyph_index: Some(glyph_index),
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Char(ch),
        small_caps: false,
        multi_subst_dup: false,
        is_vert_alt: false,
        fake_bold: false,
        fake_italic: false,
        extra_data: (),
    }
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

fn is_macroman(glyph: &Option<RawGlyph<()>>) -> bool {
    match glyph {
        Some(RawGlyph {
            glyph_origin: GlyphOrigin::Char(chr),
            ..
        }) => macroman::is_macroman(*chr),
        _ => false,
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

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Parse(err)
    }
}

impl From<ReadWriteError> for Error {
    fn from(err: ReadWriteError) -> Self {
        Error::ReadWrite(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}
