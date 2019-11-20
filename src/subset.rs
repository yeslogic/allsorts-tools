use std::fs::File;
use std::io::Write;
use std::str;

use itertools::Itertools;

use allsorts::binary::read::ReadScope;
use allsorts::font_data_impl::read_cmap_subtable;
use allsorts::fontfile::FontFile;
use allsorts::gsub::{GlyphOrigin, RawGlyph};
use allsorts::tables::cmap::Cmap;
use allsorts::tables::FontTableProvider;
use allsorts::{macroman, subset, tag};

use crate::cli::SubsetOpts;
use crate::{glyph, BoxError, ErrorMessage};

pub fn main(opts: SubsetOpts) -> Result<(), BoxError> {
    let buffer = std::fs::read(&opts.input)?;
    let font_file = ReadScope::new(&buffer).read::<FontFile>()?;
    let provider = font_file.table_provider(opts.index)?;

    subset(&provider, &opts.text, &opts.output)
}

fn subset<'a, F: FontTableProvider>(
    font_provider: &F,
    text: &str,
    output_path: &str,
) -> Result<(), BoxError> {
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

    let mut glyphs: Vec<RawGlyph<()>> = glyphs.into_iter().flatten().collect();
    glyphs.sort_by(|a, b| a.glyph_index.cmp(&b.glyph_index));
    let glyph_ids = glyphs
        .iter()
        .flat_map(|glyph| glyph.glyph_index)
        .dedup()
        .collect_vec();
    if glyph_ids.is_empty() {
        return Err(ErrorMessage("no glyphs left in font").into());
    }

    println!("Number of glyphs in new font: {}", glyph_ids.len());

    // Subset
    let cmap0 = if glyphs.iter().skip(1).all(is_macroman) {
        let mut cmap0 = [0; 256];
        glyphs
            .iter()
            .skip(1)
            .enumerate()
            .for_each(|(glyph_index, glyph)| {
                if let RawGlyph {
                    glyph_origin: GlyphOrigin::Char(chr),
                    ..
                } = glyph
                {
                    cmap0[usize::from(macroman::char_to_macroman(*chr).unwrap())] =
                        glyph_index as u8 + 1;
                }
            });
        Some(Box::new(cmap0))
    } else {
        return Err(ErrorMessage("not mac roman compatible").into());
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
) -> Result<Vec<Option<RawGlyph<()>>>, BoxError> {
    let cmap_data = font_provider.read_table_data(tag::CMAP)?;
    let cmap = ReadScope::new(&cmap_data).read::<Cmap>()?;
    let cmap_subtable =
        read_cmap_subtable(&cmap)?.ok_or(ErrorMessage("no suitable cmap sub-table found"))?;

    let glyphs = text
        .chars()
        .map(|ch| glyph::map(&cmap_subtable, ch))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(glyphs)
}

fn is_macroman(glyph: &RawGlyph<()>) -> bool {
    match glyph {
        RawGlyph {
            glyph_origin: GlyphOrigin::Char(chr),
            ..
        } => macroman::is_macroman(*chr),
        _ => false,
    }
}
