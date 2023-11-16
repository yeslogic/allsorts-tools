use std::collections::HashMap;
use std::str::FromStr;

use allsorts::binary::read::ReadScope;
use allsorts::cff::CFF;
use allsorts::error::ParseError;
use allsorts::font::{GlyphTableFlags, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::gsub::{FeatureMask, Features};
use allsorts::pathfinder_geometry::transform2d::Matrix2x2F;
use allsorts::pathfinder_geometry::vector::vec2f;
use allsorts::post::PostTable;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::variable_fonts::fvar::FvarTable;
use allsorts::tables::{Fixed, FontTableProvider, SfntVersion};
use allsorts::{tag, Font};

use crate::cli::SvgOpts;
use crate::script;
use crate::writer::{GlyfPost, SVGMode, SVGWriter};
use crate::BoxError;

const FONT_SIZE: f32 = 1000.0;

pub fn main(opts: SvgOpts) -> Result<i32, BoxError> {
    // Read and parse the font
    let buffer = load_font_maybe_instance(&opts)?;
    let (script, lang) = script_and_lang_from_testcase(&opts.testcase);
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
    let infos = font
        .shape(
            glyphs,
            script,
            Some(lang),
            &Features::Mask(FeatureMask::default()),
            true,
        )
        .map_err(|(err, _infos)| err)?;
    let direction = script::direction(script);

    // TODO: Can we avoid creating a new table provider?
    let provider = font_file.table_provider(0)?;

    // Turn each glyph into an SVG...
    let head = font.head_table()?.ok_or(ParseError::MissingValue)?;
    let scale = FONT_SIZE / f32::from(head.units_per_em);
    let transform = if opts.flip {
        Matrix2x2F::from_scale(vec2f(scale, -scale))
    } else {
        Matrix2x2F::from_scale(scale)
    };
    let svg = if font.glyph_table_flags.contains(GlyphTableFlags::CFF)
        && provider.sfnt_version() == tag::OTTO
    {
        let cff_data = provider.read_table_data(tag::CFF)?;
        let mut cff = ReadScope::new(&cff_data).read::<CFF<'_>>()?;
        let writer = SVGWriter::new(SVGMode::TextRenderingTests(opts.testcase), transform);
        writer.glyphs_to_svg(&mut cff, &mut font, &infos, direction)?
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
        let writer = SVGWriter::new(SVGMode::TextRenderingTests(opts.testcase), transform);
        writer.glyphs_to_svg(&mut glyf_post, &mut font, &infos, direction)?
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

fn load_font_maybe_instance(opts: &SvgOpts) -> Result<Vec<u8>, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData<'_>>()?;
    let provider = font_file.table_provider(0)?;

    if provider.has_table(tag::FVAR) && provider.has_table(tag::GVAR) {
        // Construct the user tuple
        // wght:28;wdth:100;opsz:72
        let test_variations = dbg!(opts.variation.as_deref().unwrap_or(""))
            .split(';')
            .map(|pair| {
                let (axis, value) = pair.split_once(':').expect("variation does no contain ':'");
                let axis = tag::from_string(axis).expect("invalid axis tag");
                let value = f64::from_str(value)
                    .map(Fixed::from)
                    .expect("invalid axis value");
                (axis, value)
            })
            .collect::<HashMap<_, _>>();

        let table = provider.read_table_data(tag::FVAR)?;
        let fvar = ReadScope::new(&table).read::<FvarTable<'_>>()?;
        let user_tuple = fvar
            .axes()
            .map(|axis| {
                test_variations
                    .get(&axis.axis_tag)
                    .copied()
                    .unwrap_or(axis.default_value)
            })
            .collect::<Vec<_>>();

        allsorts::variations::instance(&provider, &user_tuple).map_err(BoxError::from)
    } else {
        drop(provider);
        Ok(buffer)
    }
}
