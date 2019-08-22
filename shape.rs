use fontcode::error::{ParseError, ShapingError};
use fontcode::font_data_impl::read_cmap_subtable;
use fontcode::gpos::{gpos_apply, Info};
use fontcode::gsub::{gsub_apply_default, GlyphOrigin, RawGlyph};
use fontcode::layout::{GDEFTable, LayoutTable, GPOS, GSUB};
use fontcode::read::ReadScope;
use fontcode::tables::cmap::{Cmap, CmapSubtable};
use fontcode::tables::{OffsetTable, OpenTypeFile, OpenTypeFont, TTCHeader};
use fontcode::tag;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{self, Read};

fn main() -> Result<(), ShapingError> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        println!("Usage: shape FONTFILE SCRIPT LANG TEXT");
        return Ok(());
    }

    let filename = &args[1];
    let script = tag::from_string(&args[2])?;
    let lang = tag::from_string(&args[3])?;
    let text = &args[4];
    let buffer = read_file(filename)?;

    let fontfile = ReadScope::new(&buffer).read::<OpenTypeFile>()?;

    match fontfile.font {
        OpenTypeFont::Single(ttf) => shape_ttf(&fontfile.scope, ttf, script, lang, text)?,
        OpenTypeFont::Collection(ttc) => shape_ttc(&fontfile.scope, ttc, script, lang, text)?,
    }

    Ok(())
}

fn read_file(path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn shape_ttc<'a>(
    scope: &ReadScope<'a>,
    ttc: TTCHeader<'a>,
    script: u32,
    lang: u32,
    text: &str,
) -> Result<(), ShapingError> {
    for offset_table_offset in &ttc.offset_tables {
        let offset_table_offset = usize::try_from(offset_table_offset)?;
        let offset_table = scope.offset(offset_table_offset).read::<OffsetTable>()?;
        shape_ttf(scope, offset_table, script, lang, text)?;
    }
    Ok(())
}

fn shape_ttf<'a>(
    scope: &ReadScope<'a>,
    ttf: OffsetTable<'a>,
    script: u32,
    lang: u32,
    text: &str,
) -> Result<(), ShapingError> {
    let cmap = if let Some(cmap_scope) = ttf.read_table(&scope, tag::CMAP)? {
        cmap_scope.read::<Cmap>()?
    } else {
        println!("no cmap table");
        return Ok(());
    };
    let cmap_subtable = if let Some(cmap_subtable) = read_cmap_subtable(&cmap)? {
        cmap_subtable
    } else {
        println!("no suitable cmap subtable");
        return Ok(());
    };
    let opt_glyphs_res: Result<Vec<_>, _> = text
        .chars()
        .map(|ch| map_glyph(&cmap_subtable, ch))
        .collect();
    let opt_glyphs = opt_glyphs_res?;
    let mut glyphs = opt_glyphs.into_iter().flatten().collect();
    println!("glyphs before: {:?}", glyphs);
    if let Some(gsub_record) = ttf.find_table_record(tag::GSUB) {
        let gsub_table = gsub_record
            .read_table(&scope)?
            .read::<LayoutTable<GSUB>>()?;
        let opt_gdef_table = match ttf.find_table_record(tag::GDEF) {
            Some(gdef_record) => Some(gdef_record.read_table(&scope)?.read::<GDEFTable>()?),
            None => None,
        };
        let opt_gpos_table = match ttf.find_table_record(tag::GPOS) {
            Some(gpos_record) => Some(
                gpos_record
                    .read_table(&scope)?
                    .read::<LayoutTable<GPOS>>()?,
            ),
            None => None,
        };
        let vertical = false;
        gsub_apply_default(
            &|| make_dotted_circle(&cmap_subtable),
            &gsub_table,
            opt_gdef_table.as_ref(),
            script,
            lang,
            vertical,
            &mut glyphs,
        )?;
        println!("glyphs after: {:?}", glyphs);
        match opt_gpos_table {
            Some(gpos_table) => {
                let kerning = true;
                let mut infos = Info::init_from_glyphs(opt_gdef_table.as_ref(), glyphs)?;
                gpos_apply(
                    &gpos_table,
                    opt_gdef_table.as_ref(),
                    kerning,
                    script,
                    lang,
                    &mut infos,
                )?;
            }
            None => {}
        }
    } else {
        println!("no GSUB table");
    }
    Ok(())
}

fn make_dotted_circle(cmap_subtable: &CmapSubtable) -> Vec<RawGlyph<()>> {
    match map_glyph(cmap_subtable, '\u{25cc}') {
        Ok(Some(raw_glyph)) => vec![raw_glyph],
        _ => Vec::new(),
    }
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
