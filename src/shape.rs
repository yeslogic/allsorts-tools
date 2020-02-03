use std::convert::TryFrom;

use allsorts::binary::read::ReadScope;
use allsorts::error::ShapingError;
use allsorts::font_data_impl::read_cmap_subtable;
use allsorts::gpos::{gpos_apply, Info};
use allsorts::gsub::{gsub_apply_default, RawGlyph};
use allsorts::layout::{new_layout_cache, GDEFTable, LayoutTable, GPOS, GSUB};
use allsorts::tables::cmap::{Cmap, CmapSubtable};
use allsorts::tables::{MaxpTable, OffsetTable, OpenTypeFile, OpenTypeFont, TTCHeader};
use allsorts::tag;

use crate::cli::ShapeOpts;
use crate::glyph;
use crate::BoxError;

pub fn main(opts: ShapeOpts) -> Result<i32, BoxError> {
    let script = tag::from_string(&opts.script)?;
    let lang = tag::from_string(&opts.lang)?;
    let buffer = std::fs::read(&opts.font)?;
    let fontfile = ReadScope::new(&buffer).read::<OpenTypeFile>()?;

    match fontfile.font {
        OpenTypeFont::Single(ttf) => shape_ttf(&fontfile.scope, ttf, script, lang, &opts.text)?,
        OpenTypeFont::Collection(ttc) => shape_ttc(&fontfile.scope, ttc, script, lang, &opts.text)?,
    }

    Ok(0)
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
    let maxp = if let Some(maxp_scope) = ttf.read_table(&scope, tag::MAXP)? {
        maxp_scope.read::<MaxpTable>()?
    } else {
        println!("no maxp table");
        return Ok(());
    };
    let opt_glyphs_res: Result<Vec<_>, _> = text
        .chars()
        .map(|ch| glyph::map(&cmap_subtable, ch))
        .collect();
    let opt_glyphs = opt_glyphs_res?;
    let mut glyphs = opt_glyphs.into_iter().flatten().collect();
    println!("glyphs before: {:#?}", glyphs);
    if let Some(gsub_record) = ttf.find_table_record(tag::GSUB) {
        let gsub_table = gsub_record.read_table(scope)?.read::<LayoutTable<GSUB>>()?;
        let gsub_cache = new_layout_cache::<GSUB>(gsub_table);
        let opt_gdef_table = match ttf.find_table_record(tag::GDEF) {
            Some(gdef_record) => Some(gdef_record.read_table(scope)?.read::<GDEFTable>()?),
            None => None,
        };
        let opt_gpos_cache = match ttf.find_table_record(tag::GPOS) {
            Some(gpos_record) => {
                let gpos_table = gpos_record.read_table(scope)?.read::<LayoutTable<GPOS>>()?;
                let gpos_cache = new_layout_cache::<GPOS>(gpos_table);
                Some(gpos_cache)
            }
            None => None,
        };
        let vertical = false;
        gsub_apply_default(
            &|| make_dotted_circle(&cmap_subtable),
            &gsub_cache,
            opt_gdef_table.as_ref(),
            script,
            lang,
            vertical,
            maxp.num_glyphs,
            &mut glyphs,
        )?;
        println!("glyphs after: {:#?}", glyphs);
        match opt_gpos_cache {
            Some(gpos_cache) => {
                let kerning = true;
                let mut infos = Info::init_from_glyphs(opt_gdef_table.as_ref(), glyphs)?;
                gpos_apply(
                    &gpos_cache,
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
    match glyph::map(cmap_subtable, '\u{25cc}') {
        Ok(Some(raw_glyph)) => vec![raw_glyph],
        _ => Vec::new(),
    }
}
