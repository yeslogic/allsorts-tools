use allsorts::binary::read::ReadScope;
use allsorts::cff::CFF;
use allsorts::error::ParseError;
use allsorts::font::{Font, GlyphTableFlags, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::gsub::{FeatureInfo, FeatureMask, Features, GlyphOrigin, RawGlyph};
use allsorts::pathfinder_geometry::transform2d::Matrix2x2F;
use allsorts::pathfinder_geometry::vector::vec2f;
use allsorts::post::PostTable;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::{FontTableProvider, SfntVersion};
use allsorts::tag;
use allsorts::tinyvec::tiny_vec;

use crate::cli::ViewOpts;
use crate::script;
use crate::writer::{GlyfPost, SVGMode, SVGWriter};
use crate::BoxError;

const FONT_SIZE: f32 = 1000.0;

pub fn main(opts: ViewOpts) -> Result<i32, BoxError> {
    let script = tag::from_string(&opts.script)?;
    let lang = opts
        .lang
        .as_deref()
        .map(|s| tag::from_string(&s).expect("invalid language tag"));

    match (&opts.text, &opts.codepoints, &opts.indices) {
        (Some(_), None, None) | (None, Some(_), None) | (None, None, Some(_)) => {}
        (_, _, _) => {
            eprintln!("required option: --text OR --codepoints OR --indices");
            return Ok(1);
        }
    }

    let features = match opts.features {
        Some(ref features) => parse_features(&features),
        None => Features::Mask(FeatureMask::default()),
    };

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

    let glyphs = if let Some(ref text) = opts.text {
        font.map_glyphs(&text, script, MatchingPresentation::NotRequired)
    } else if let Some(ref codepoints) = opts.codepoints {
        let text = parse_codepoints(&codepoints);
        font.map_glyphs(&text, script, MatchingPresentation::NotRequired)
    } else if let Some(ref indices) = opts.indices {
        parse_glyph_indices(&indices)
    } else {
        panic!("expected --text OR --codepoints OR --indices");
    };

    let infos = font
        .shape(glyphs, script, lang, &features, true)
        .map_err(|(err, _infos)| err)?;
    let direction = script::direction(script);

    // TODO: Can we avoid creating a new table provider?
    let provider = font_file.table_provider(0)?;

    // Turn each glyph into an SVG...
    let head = font.head_table()?.ok_or(ParseError::MissingValue)?;
    let scale = FONT_SIZE / f32::from(head.units_per_em);
    let transform = Matrix2x2F::from_scale(vec2f(scale, -scale));
    let mode = SVGMode::from(&opts);
    let svg = if font.glyph_table_flags.contains(GlyphTableFlags::CFF)
        && provider.sfnt_version() == tag::OTTO
    {
        let cff_data = provider.read_table_data(tag::CFF)?;
        let mut cff = ReadScope::new(&cff_data).read::<CFF<'_>>()?;
        let writer = SVGWriter::new(mode, transform);
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
        let writer = SVGWriter::new(mode, transform);
        writer.glyphs_to_svg(&mut glyf_post, &mut font, &infos, direction)?
    } else {
        eprintln!("no glyf or CFF table");
        return Ok(1);
    };

    println!("{}", svg);

    Ok(0)
}

fn parse_codepoints(codepoints: &str) -> String {
    codepoints
        .split(',')
        .map(str::trim)
        .map(hex_string_to_char)
        .collect::<String>()
}

fn hex_string_to_char(hex: &str) -> char {
    let i = u32::from_str_radix(hex, 16)
        .expect(format!("failed to parse hex string '{}'", hex).as_str());
    std::char::from_u32(i).unwrap_or('\u{FFFD}')
}

fn parse_glyph_indices(glyph_indices: &str) -> Vec<RawGlyph<()>> {
    glyph_indices
        .split(',')
        .map(str::trim)
        .map(string_to_u16)
        .map(make_raw_glyph)
        .collect()
}

fn string_to_u16(s: &str) -> u16 {
    u16::from_str_radix(s, 10).expect(format!("failed to parse u16 string '{}'", s).as_str())
}

fn make_raw_glyph(glyph_index: u16) -> RawGlyph<()> {
    RawGlyph {
        unicodes: tiny_vec![],
        glyph_index,
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Char('x'),
        small_caps: false,
        multi_subst_dup: false,
        is_vert_alt: false,
        fake_bold: false,
        fake_italic: false,
        variation: None,
        extra_data: (),
    }
}

fn parse_features(features: &str) -> Features {
    let feature_infos = features
        .split(',')
        .map(str::trim)
        .map(|s| tag::from_string(s).expect(format!("invalid feature '{}'", s).as_str()))
        .map(|f| FeatureInfo {
            feature_tag: f,
            alternate: None,
        })
        .collect();
    Features::Custom(feature_infos)
}

impl From<&ViewOpts> for SVGMode {
    fn from(opts: &ViewOpts) -> Self {
        SVGMode::View {
            mark_origin: opts.mark_origin,
        }
    }
}
