mod writer;

use allsorts::binary::read::ReadScope;
use allsorts::cff::CFF;
use allsorts::error::ParseError;
use allsorts::font::{GlyphTableFlags, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::gsub::{Features, GsubFeatureMask};
use allsorts::post::PostTable;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::FontTableProvider;
use allsorts::{tag, Font};

use crate::cli::SvgOpts;
use crate::svg::writer::SVGWriter;
use crate::BoxError;

pub fn main(opts: SvgOpts) -> Result<i32, BoxError> {
    // Read and parse the font
    let script = tag::LATN;
    let lang = tag::from_string("ENG ")?;
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData<'_>>()?;
    let provider = font_file.table_provider(0)?;
    let mut font = match Font::new(provider)? {
        Some(font) => font,
        None => {
            eprintln!("unable to find suitable cmap subtable");
            return Ok(1);
        }
    };

    // Map text to glyphs and then apply font shaping
    let glyphs = font.map_glyphs(&opts.render, MatchingPresentation::NotRequired);
    let infos = font.shape(
        glyphs,
        script,
        Some(lang),
        &Features::Mask(GsubFeatureMask::default()),
        true,
    )?;

    // TODO: Can we avoid creating a new table provider?
    let provider = font_file.table_provider(0)?;
    let post_data = provider.table_data(tag::POST)?;
    let post = post_data
        .as_ref()
        .map(|data| ReadScope::new(data).read::<PostTable<'_>>())
        .transpose()?;

    // Turn each glyph into an SVG...
    let svg = if font.glyph_table_flags.contains(GlyphTableFlags::CFF) {
        let cff_data = provider.read_table_data(tag::CFF)?;
        let mut cff = ReadScope::new(&cff_data).read::<CFF<'_>>()?;
        let writer = SVGWriter::new(opts.testcase, opts.flip);
        writer.glyphs_to_svg(&mut cff, &mut font, &infos, post.as_ref())?
    } else if font.glyph_table_flags.contains(GlyphTableFlags::GLYF) {
        let head = font.head_table()?.ok_or(ParseError::MissingValue)?;
        let loca_data = provider.read_table_data(tag::LOCA)?;
        let loca = ReadScope::new(&loca_data).read_dep::<LocaTable<'_>>((
            usize::from(font.maxp_table.num_glyphs),
            head.index_to_loc_format,
        ))?;
        let glyf_data = provider.read_table_data(tag::GLYF)?;
        let mut glyf = ReadScope::new(&glyf_data).read_dep::<GlyfTable<'_>>(&loca)?;
        let writer = SVGWriter::new(opts.testcase, opts.flip);
        writer.glyphs_to_svg(&mut glyf, &mut font, &infos, post.as_ref())?
    } else {
        eprintln!("no glyf or CFF table");
        return Ok(1);
    };

    println!("{}", svg);

    Ok(0)
}
