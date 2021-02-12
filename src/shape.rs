use allsorts::binary::read::ReadScope;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::gsub::{Features, GsubFeatureMask};
use allsorts::tag;

use crate::cli::ShapeOpts;
use crate::glyph::{calculate_glyph_positions, TextDirection};
use crate::BoxError;

pub fn main(opts: ShapeOpts) -> Result<i32, BoxError> {
    let script = tag::from_string(&opts.script)?;
    let lang = tag::from_string(&opts.lang)?;
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData<'_>>()?;
    let provider = font_file.table_provider(opts.index)?;
    let mut font = match Font::new(Box::new(provider))? {
        Some(font) => font,
        None => {
            eprintln!("unable to find suitable cmap subtable");
            return Ok(1);
        }
    };
    let glyphs = font.map_glyphs(&opts.text, script, MatchingPresentation::NotRequired);
    let infos = font.shape(
        glyphs,
        script,
        Some(lang),
        &Features::Mask(GsubFeatureMask::default()),
        true,
    )?;
    let positions =
        calculate_glyph_positions(&mut font, &infos, TextDirection::LeftToRight, opts.vertical)?;

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
