use allsorts::context::Glyph;
use allsorts::error::ParseError;
use allsorts::gpos::{Info, MarkPlacement, Placement};
use allsorts::gsub::{GlyphOrigin, RawGlyph};
use allsorts::tables::cmap::CmapSubtable;
use allsorts::tables::FontTableProvider;
use allsorts::tinyvec::tiny_vec;
use allsorts::unicode::VariationSelector;
use allsorts::Font;

use crate::unicode::is_upright_char;
use crate::BoxError;

pub(crate) fn map(
    cmap_subtable: &CmapSubtable,
    ch: char,
    variation: Option<VariationSelector>,
) -> Result<Option<RawGlyph<()>>, ParseError> {
    if let Some(glyph_index) = cmap_subtable.map_glyph(ch as u32)? {
        let glyph = make(ch, glyph_index, variation);
        Ok(Some(glyph))
    } else {
        Ok(None)
    }
}

pub(crate) fn make(
    ch: char,
    glyph_index: u16,
    variation: Option<VariationSelector>,
) -> RawGlyph<()> {
    RawGlyph {
        unicodes: tiny_vec![[char; 1] => ch],
        glyph_index,
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Char(ch),
        small_caps: false,
        multi_subst_dup: false,
        is_vert_alt: false,
        fake_bold: false,
        fake_italic: false,
        extra_data: (),
        variation,
    }
}

pub fn calculate_glyph_positions<T: FontTableProvider>(
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
