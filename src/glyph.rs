use allsorts::error::ParseError;
use allsorts::gsub::{GlyphOrigin, RawGlyph};
use allsorts::tables::cmap::CmapSubtable;
use allsorts::tinyvec::tiny_vec;
use allsorts::unicode::VariationSelector;

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
        unicodes: tiny_vec![[char; 1], ch],
        glyph_index: Some(glyph_index),
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
