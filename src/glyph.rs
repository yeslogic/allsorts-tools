use std::convert::TryFrom;

use allsorts::context::Glyph as _;
use allsorts::error::ParseError;
use allsorts::gpos::{Attachment, Info, Placement};
use allsorts::gsub::{GlyphOrigin, RawGlyph};
use allsorts::tables::cmap::CmapSubtable;
use allsorts::tables::FontTableProvider;
use allsorts::tinyvec::tiny_vec;
use allsorts::unicode::VariationSelector;
use allsorts::Font;

use crate::unicode::is_upright_char;
use crate::BoxError;

#[derive(Debug, Copy, Clone)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

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

#[derive(Debug, Default, Copy, Clone)]
pub struct GlyphPosition {
    pub hori_advance: i32,
    pub vert_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    cursive_attachment: Option<u16>,
}

impl GlyphPosition {
    pub const fn new(hori_advance: i32, vert_advance: i32, x_offset: i32, y_offset: i32) -> Self {
        GlyphPosition {
            hori_advance,
            vert_advance,
            x_offset,
            y_offset,
            cursive_attachment: None,
        }
    }
}

pub fn calculate_glyph_positions<T: FontTableProvider>(
    font: &mut Font<T>,
    infos: &[Info],
    direction: TextDirection,
    vertical: bool,
) -> Result<Vec<GlyphPosition>, BoxError> {
    let mut has_marks = false;
    let mut has_cursive_connection = false;
    let mut positions = vec![GlyphPosition::default(); infos.len()];
    for (i, info) in infos.iter().enumerate() {
        let (hori_advance, vert_advance) = glyph_advance(font, info, vertical)?;
        match info.attachment {
            Attachment::None => match info.placement {
                Placement::None => {
                    positions[i] = GlyphPosition::new(hori_advance, vert_advance, 0, 0)
                }
                Placement::Distance(dx, dy) => {
                    positions[i] = GlyphPosition::new(hori_advance, vert_advance, dx, dy)
                }
            },
            Attachment::MarkAnchor(base_index, base_anchor, mark_anchor) => {
                has_marks = true;
                match infos.get(base_index) {
                    Some(base_info) => {
                        // TODO: Do this later?
                        let (dx, dy) = match base_info.placement {
                            Placement::None => (0, 0),
                            Placement::Distance(dx, dy) => (dx, dy),
                        };
                        let offset_x = i32::from(base_anchor.x) - i32::from(mark_anchor.x) + dx;
                        let offset_y = i32::from(base_anchor.y) - i32::from(mark_anchor.y) + dy;
                        positions[i] =
                            GlyphPosition::new(hori_advance, vert_advance, offset_x, offset_y);
                    }
                    None => {
                        return Err(ParseError::BadIndex.into());
                    }
                }
            }
            Attachment::MarkOverprint(base_index) => {
                has_marks = true;
                // FIXME: Should there be zero advance in this case?
                infos.get(base_index).ok_or(ParseError::BadIndex)?;
            }
            Attachment::CursiveAnchor(exit_glyph_index, _, _, _) => {
                has_cursive_connection = true;
                // Validate index
                infos.get(exit_glyph_index).ok_or(ParseError::BadIndex)?;

                // Link to exit glyph
                positions[exit_glyph_index].cursive_attachment = Some(u16::try_from(i)?);
                positions[i] = GlyphPosition {
                    hori_advance,
                    vert_advance,
                    ..positions[i]
                };
            }
        };
    }

    if has_cursive_connection {
        // Now that we know all base glyphs are positioned we do a second pass to apply
        // cursive attachment adjustments
        for (i, info) in infos.iter().enumerate() {
            match info.attachment {
                Attachment::None
                | Attachment::MarkAnchor(_, _, _)
                | Attachment::MarkOverprint(_) => {}
                Attachment::CursiveAnchor(
                    exit_glyph_index,
                    rtl_flag,
                    exit_glyph_anchor,
                    entry_glyph_anchor,
                ) => {
                    // Anchor alignment can result in horizontal or vertical positioning adjustments,
                    // or both. Note that the positioning effects in the text-layout direction
                    // (horizontal, for horizontal layout) work differently than for the cross-stream
                    // direction (vertical, in horizontal layout):
                    //
                    // * For adjustments in the line-layout direction, the layout engine adjusts the
                    //   advance of the first glyph (in logical order). This effectively moves the
                    //   second glyph relative to the first so that the anchors are aligned in that
                    //   direction.
                    // * For the cross-stream direction, placement of one glyph is adjusted to make
                    //   the anchors align. Which glyph is adjusted is determined by the RIGHT_TO_LEFT
                    //   flag in the parent lookup table: if the RIGHT_TO_LEFT flag is clear, the
                    //   second glyph is adjusted to align anchors with the first glyph; if the
                    //   RIGHT_TO_LEFT flag is set, the first glyph is adjusted to align anchors with
                    //   the second glyph.
                    //
                    // https://docs.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-3-cursive-attachment-positioning-subtable

                    // First glyph in logical order is the one with the lower index
                    let (first_glyph_index, second_glyph_index) = if i < exit_glyph_index {
                        (i, exit_glyph_index)
                    } else {
                        (exit_glyph_index, i)
                    };

                    // Line-layout direction
                    // FIXME: Handle vertical text
                    match direction {
                        TextDirection::LeftToRight => {
                            positions[first_glyph_index].hori_advance =
                                i32::from(entry_glyph_anchor.x)
                        }
                        TextDirection::RightToLeft => {
                            //  TODO: Find an example to test this further
                            positions[first_glyph_index].hori_advance +=
                                i32::from(entry_glyph_anchor.x)
                        }
                    }

                    // Cross-stream direction
                    let dy = i32::from(exit_glyph_anchor.y) - i32::from(entry_glyph_anchor.y);
                    if rtl_flag == true {
                        positions[first_glyph_index].y_offset +=
                            dy + positions[second_glyph_index].y_offset;
                        if let Some(linked_index) = positions[first_glyph_index].cursive_attachment
                        {
                            adjust_cursive_chain(
                                dy,
                                direction,
                                usize::from(linked_index),
                                infos,
                                &mut positions,
                            );
                        }
                    } else {
                        positions[second_glyph_index].y_offset +=
                            dy + positions[first_glyph_index].y_offset;
                        if let Some(linked_index) = positions[second_glyph_index].cursive_attachment
                        {
                            adjust_cursive_chain(
                                dy,
                                direction,
                                usize::from(linked_index),
                                infos,
                                &mut positions,
                            );
                        }
                    }
                }
            }
        }
    }

    if has_marks {
        // Now that cursive connected glyphs are positioned, ensure marks are positioned on their
        // base properly.
        for (i, info) in infos.iter().enumerate() {
            match info.attachment {
                Attachment::None | Attachment::CursiveAnchor(_, _, _, _) => {}
                Attachment::MarkAnchor(base_index, _, _) => {
                    let base_pos = positions[base_index];
                    let (hori_advance_offset, vert_advance_offset) = match direction {
                        TextDirection::LeftToRight => sum_advance(positions.get(base_index..i)),
                        TextDirection::RightToLeft => sum_advance(positions.get(i..base_index)),
                    };

                    // Add the x & y offset of the base glyph to the mark
                    let position = &mut positions[i];
                    position.x_offset += base_pos.x_offset;
                    position.y_offset += base_pos.y_offset;

                    // Shift the mark back the advance of the base glyph and glyphs leading to it
                    // so that it is positioned above it
                    match direction {
                        TextDirection::LeftToRight => {
                            position.x_offset -= hori_advance_offset;
                            position.y_offset -= vert_advance_offset;
                        }
                        TextDirection::RightToLeft => {
                            position.x_offset += hori_advance_offset;
                            position.y_offset += vert_advance_offset;
                        }
                    }
                }
                Attachment::MarkOverprint(base_index) => {
                    let base_pos = positions[base_index];
                    let position = &mut positions[i];
                    position.x_offset = base_pos.x_offset;
                    position.y_offset = base_pos.y_offset;
                }
            }
        }
    }
    Ok(positions)
}

fn adjust_cursive_chain(
    delta: i32,
    direction: TextDirection,
    index: usize,
    infos: &[Info],
    positions: &mut [GlyphPosition],
) {
    let position = &mut positions[index];
    position.y_offset += delta;
    if let Some(next_index) = position.cursive_attachment {
        // TODO: prevent cycles
        adjust_cursive_chain(delta, direction, usize::from(next_index), infos, positions)
    }
}

fn sum_advance(positions: Option<&[GlyphPosition]>) -> (i32, i32) {
    positions.map_or((0, 0), |p| {
        p.iter().fold((0, 0), |(hori, vert), &pos| {
            (hori + pos.hori_advance, vert + pos.vert_advance)
        })
    })
}

fn glyph_advance<T: FontTableProvider>(
    font: &mut Font<T>,
    info: &Info,
    vertical: bool,
) -> Result<(i32, i32), BoxError> {
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
    Ok(if vertical { (0, advance) } else { (advance, 0) })
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
