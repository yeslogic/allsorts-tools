use getopts::Options;

use fontcode::error::{ParseError, ReadWriteError};
use fontcode::font_tables::{FontImpl, FontTablesImpl};
use fontcode::glyph_index::read_cmap_subtable;
use fontcode::gsub::{GlyphOrigin, RawGlyph};
use fontcode::read::ReadScope;
use fontcode::tables::cmap::{Cmap, CmapSubtable};
use fontcode::tables::FontTableProvider;
use fontcode::tag;
use fontcode::{macroman, subset};
use itertools::Itertools;

use std::borrow::Cow;
use std::env;
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
    let buffer = read_file(input)?;

    let font = FontImpl::new(&buffer, 0).unwrap();
    let provider = FontTablesImpl::FontImpl(font);
    subset(&provider, &text, output)?;

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

fn subset<'a, F: FontTableProvider>(
    font_provider: &F,
    text: &str,
    output_path: &str,
) -> Result<(), Error> {
    // Work out the glyphs we want to keep from the text
    let mut glyphs = chars_to_glyphs(font_provider, text)?;
    let notdef = RawGlyph {
        unicodes: vec![],
        glyph_index: Some(0),
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Direct,
        small_caps: false,
        multi_subst_dup: false,
        is_vert_alt: false,
        fake_bold: false,
        fake_italic: false,
        extra_data: (),
    };
    glyphs.insert(0, Some(notdef));

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
    let cmap0 = if glyphs.iter().skip(1).all(is_macroman) {
        let mut cmap0 = [0; 256];
        glyphs
            .iter()
            .skip(1)
            .enumerate()
            .for_each(|(glyph_index, glyph)| match glyph {
                Some(RawGlyph {
                    glyph_origin: GlyphOrigin::Char(chr),
                    ..
                }) => {
                    cmap0[usize::from(macroman::char_to_macroman(*chr).unwrap())] =
                        glyph_index as u8 + 1
                }
                _ => unreachable!(),
            });
        Some(Box::new(cmap0))
    } else {
        return Err(Error::Message("not mac roman compatible"));
    };

    let new_font = subset::subset(font_provider, &glyph_ids, cmap0)?;

    // Write out the new font
    let mut output = File::create(output_path)?;
    output.write_all(&new_font)?;

    Ok(())
}

fn chars_to_glyphs<'a, F: FontTableProvider>(
    font_provider: &F,
    text: &str,
) -> Result<Vec<Option<RawGlyph<()>>>, Error> {
    let cmap_data = read_table_data(font_provider, tag::CMAP)?;
    let cmap = ReadScope::new(&cmap_data).read::<Cmap>()?;
    let cmap_subtable =
        read_cmap_subtable(&cmap)?.ok_or(Error::Message("no suitable cmap sub-table found"))?;

    let glyphs = text
        .chars()
        .map(|ch| map_glyph(&cmap_subtable, ch))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(glyphs)
}

fn read_table_data<'a, F: FontTableProvider>(
    provider: &'a F,
    tag: u32,
) -> Result<Cow<'a, [u8]>, ParseError> {
    provider.table_data(tag)?.ok_or(ParseError::MissingValue)
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

fn is_macroman(glyph: &Option<RawGlyph<()>>) -> bool {
    match glyph {
        Some(RawGlyph {
            glyph_origin: GlyphOrigin::Char(chr),
            ..
        }) => macroman::is_macroman(*chr),
        _ => false,
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
