use allsorts::binary::read::ReadScope;
use allsorts::context::Glyph;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::gpos::{Info, MarkPlacement, Placement};
use allsorts::gsub::{Features, GsubFeatureMask};
use allsorts::tables::FontTableProvider;
use allsorts::tag;

use crate::cli::ShapeOpts;
use crate::unicode::is_upright_char;
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
    let glyphs = font.map_glyphs(&opts.text, MatchingPresentation::NotRequired);
    let infos = font.shape(
        glyphs,
        script,
        Some(lang),
        &Features::Mask(GsubFeatureMask::default()),
        true,
    )?;
    let positions = calculate_glyph_positions(&mut font, &infos, opts.vertical)?;

    for (glyph, (advance, x, y)) in infos.iter().zip(&positions) {
        println!("{} ({}, {}) {:#?}", advance, x, y, glyph);
    }

    Ok(0)
}

fn calculate_glyph_positions<T: FontTableProvider>(
    font: &mut Font<T>,
    infos: &[Info],
    vertical: bool,
) -> Result<Vec<(i32, i32, i32)>, BoxError> {
    let mut x = 0;
    let mut y = 0;
    let mut positions: Vec<(i32, i32, i32)> = Vec::with_capacity(infos.len());

    for info in infos.iter() {
        let advance = glyph_advance(font, info, vertical)?;
        let position = match info.mark_placement {
            MarkPlacement::None => {
                match info.placement {
                    Placement::None => (advance, x, y),
                    Placement::Distance(dx, dy) => (advance, x + dx, y + dy),
                    Placement::Anchor(_, _) => (advance, x, y), // TODO: position anchors for cursive
                }
            }
            MarkPlacement::MarkAnchor(base_index, base_anchor, mark_anchor) => {
                match (positions.get(base_index), infos.get(base_index)) {
                    (Some((_, base_x, base_y)), Some(base_info)) => {
                        let (dx, dy) = match base_info.placement {
                            Placement::None | Placement::Anchor(_, _) => (0, 0),
                            Placement::Distance(dx, dy) => (dx, dy),
                        };
                        (
                            advance,
                            base_x + i32::from(base_anchor.x) - i32::from(mark_anchor.x) + dx,
                            base_y + i32::from(base_anchor.y) - i32::from(mark_anchor.y) + dy,
                        )
                    }
                    // If you were rasterising you might drop the mark here instead
                    _ => (advance, x, y),
                }
            }
            MarkPlacement::MarkOverprint(base_index) => positions[base_index],
        };
        positions.push(position);

        if vertical {
            y += advance;
        } else {
            x += advance;
        }
    }

    Ok(positions)
}

fn glyph_advance<T: FontTableProvider>(
    font: &mut Font<T>,
    info: &Info,
    vertical: bool,
) -> Result<i32, BoxError> {
    let advance = if vertical && is_upright_glyph(&info) {
        font.vertical_advance(info.get_glyph_index())
            .map(i32::from)
            .unwrap_or_else(|| {
                i32::from(font.hhea_table.ascender) - i32::from(font.hhea_table.descender)
            })
            + i32::from(info.kerning)
    } else {
        font.horizontal_advance(info.get_glyph_index())
            .map(i32::from)
            .ok_or_else(|| {
                // `hmtx` is a required table so this error is unlikely in practice.
                format!("no horizontal advance for glyph {}", info.glyph.glyph_index)
            })?
            + i32::from(info.kerning)
    };
    Ok(advance)
}

fn is_upright_glyph(info: &Info) -> bool {
    info.glyph.is_vert_alt
        || info
            .glyph
            .unicodes
            .first()
            .map(|&ch| is_upright_char(ch))
            .unwrap_or(false)
}
