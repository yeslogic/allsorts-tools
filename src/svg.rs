mod writer;

use allsorts::binary::read::ReadScope;
use allsorts::cff::CFF;
use allsorts::error::ParseError;
use allsorts::font::{GlyphTableFlags, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::gsub::{Features, GsubFeatureMask};
use allsorts::outline::{OutlineBuilder, OutlineSink};
use allsorts::post::PostTable;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::{FontTableProvider, SfntVersion};
use allsorts::{tag, Font};

use crate::cli::SvgOpts;
use crate::svg::writer::SVGWriter;
use crate::BoxError;

pub trait GlyphName {
    fn gid_to_glyph_name(&self, gid: u16) -> Option<String>;
}

struct GlyfPost<'a> {
    glyf: GlyfTable<'a>,
    post: Option<PostTable<'a>>,
}

pub fn main(opts: SvgOpts) -> Result<i32, BoxError> {
    // Read and parse the font
    let (script, lang) = script_and_lang_from_testcase(&opts.testcase);
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
    let glyphs = font.map_glyphs(&opts.render, script, MatchingPresentation::NotRequired);
    let infos = font.shape(
        glyphs,
        script,
        Some(lang),
        &Features::Mask(GsubFeatureMask::default()),
        true,
    )?;

    // TODO: Can we avoid creating a new table provider?
    let provider = font_file.table_provider(0)?;

    // Turn each glyph into an SVG...
    let head = font.head_table()?.ok_or(ParseError::MissingValue)?;
    let svg = if font.glyph_table_flags.contains(GlyphTableFlags::CFF)
        && provider.sfnt_version() == tag::OTTO
    {
        let cff_data = provider.read_table_data(tag::CFF)?;
        let mut cff = ReadScope::new(&cff_data).read::<CFF<'_>>()?;
        let writer = SVGWriter::new(opts.testcase, opts.flip, scale);
        writer.glyphs_to_svg(&mut cff, &mut font, &infos)?
    } else if font.glyph_table_flags.contains(GlyphTableFlags::GLYF) {
        let loca_data = provider.read_table_data(tag::LOCA)?;
        let loca = ReadScope::new(&loca_data).read_dep::<LocaTable<'_>>((
            usize::from(font.maxp_table.num_glyphs),
            head.index_to_loc_format,
        ))?;
        let glyf_data = provider.read_table_data(tag::GLYF)?;
        let glyf = ReadScope::new(&glyf_data).read_dep::<GlyfTable<'_>>(&loca)?;
        let post_data = provider.table_data(tag::POST)?;
        let post = post_data
            .as_ref()
            .map(|data| ReadScope::new(data).read::<PostTable<'_>>())
            .transpose()?;
        let mut glyf_post = GlyfPost { glyf, post };
        let writer = SVGWriter::new(opts.testcase, opts.flip);
        writer.glyphs_to_svg(&mut glyf_post, &mut font, &infos)?
    } else {
        eprintln!("no glyf or CFF table");
        return Ok(1);
    };

    println!("{}", svg);

    Ok(0)
}

fn script_and_lang_from_testcase(testcase: &str) -> (u32, u32) {
    if testcase.starts_with("SHARAN") {
        (tag::ARAB, tag::from_string("URD ").unwrap())
    } else if testcase.starts_with("SHBALI") {
        (
            tag::from_string("bali").unwrap(),
            tag::from_string("BAN ").unwrap(),
        )
    } else if testcase.starts_with("SHKNDA") {
        (tag::KNDA, tag::from_string("KAN ").unwrap())
    } else if testcase.starts_with("SHLANA") {
        (
            tag::from_string("THA ").unwrap(),
            tag::from_string("lana").unwrap(),
        )
    } else {
        (tag::LATN, tag::from_string("ENG ").unwrap())
    }
}

impl<'a> GlyphName for CFF<'a> {
    fn gid_to_glyph_name(&self, glyph_id: u16) -> Option<String> {
        let font = self.fonts.first()?;
        if font.is_cid_keyed() {
            return None;
        }
        let sid = font.charset.id_for_glyph(glyph_id)?;
        self.read_string(sid).ok()
    }
}

impl<'a> GlyphName for GlyfPost<'a> {
    fn gid_to_glyph_name(&self, glyph_id: u16) -> Option<String> {
        self.post
            .as_ref()
            .and_then(|post| post.glyph_name(glyph_id).ok().flatten())
            .map(|s| s.to_string())
    }
}

impl<'a> OutlineBuilder for GlyfPost<'a> {
    type Error = ParseError;

    fn visit<V: OutlineSink>(
        &mut self,
        glyph_index: u16,
        visitor: &mut V,
    ) -> Result<(), Self::Error> {
        self.glyf.visit(glyph_index, visitor)
    }
}
