use allsorts::binary::read::ReadScope;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::glyph_position::{GlyphLayout, TextDirection};
use allsorts::gsub::{FeatureMask, Features};
use allsorts::tables::variable_fonts::OwnedTuple;
use allsorts::tag;

use crate::cli::ShapeOpts;
use crate::{normalise_tuple, parse_tuple, BoxError};

pub fn main(opts: ShapeOpts) -> Result<i32, BoxError> {
    let script = tag::from_string(&opts.script)?;
    let lang = tag::from_string(&opts.lang)?;
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData<'_>>()?;
    let provider = font_file.table_provider(opts.index)?;

    let user_tuple = opts.tuple.as_deref().map(parse_tuple).transpose()?;
    let tuple = match user_tuple {
        Some(user_tuple) => match normalise_tuple(&provider, &user_tuple) {
            Ok(tuple) => Some(tuple),
            Err(err) => {
                eprintln!("unable to normalise variation tuple: {err}");
                return Ok(1);
            }
        },
        None => None,
    };

    let mut font = Font::new(Box::new(provider))?;
    let glyphs = font.map_glyphs(&opts.text, script, MatchingPresentation::NotRequired);
    let infos = font
        .shape(
            glyphs,
            script,
            Some(lang),
            &Features::Mask(FeatureMask::default()),
            tuple.as_ref().map(OwnedTuple::as_tuple),
            true,
        )
        .map_err(|(err, _infos)| err)?;
    let mut layout = GlyphLayout::new(&mut font, &infos, TextDirection::LeftToRight, opts.vertical);
    let positions = layout.glyph_positions()?;

    for (glyph, position) in infos.iter().zip(&positions) {
        println!(
            "{},{} ({}, {}) {:#?}",
            position.hori_advance,
            position.vert_advance,
            position.x_offset,
            position.y_offset,
            glyph
        );
    }

    Ok(0)
}
