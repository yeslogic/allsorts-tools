use std::convert::TryFrom;
use std::rc::Rc;

use allsorts::binary::read::ReadScope;
use allsorts::error::ShapingError;
use allsorts::font_data_impl::FontDataImpl;
use allsorts::fontfile::FontFile;
use allsorts::gpos::{gpos_apply, Info};
use allsorts::gsub::{gsub_apply_default, GsubFeatureMask, RawGlyph};
use allsorts::tables::FontTableProvider;
use allsorts::tag;
use allsorts::unicode::VariationSelector;

use crate::cli::ShapeOpts;
use crate::glyph;
use crate::BoxError;

const DOTTED_CIRCLE: char = '\u{25cc}';

pub fn main(opts: ShapeOpts) -> Result<i32, BoxError> {
    let script = tag::from_string(&opts.script)?;
    let lang = tag::from_string(&opts.lang)?;
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontFile<'_>>()?;
    let provider = font_file.table_provider(opts.index)?;
    let font_data_impl = match FontDataImpl::new(Box::new(provider))? {
        Some(font_data_impl) => font_data_impl,
        None => {
            eprintln!("unable to find suitable cmap subtable");
            return Ok(1);
        }
    };

    shape(font_data_impl, script, lang, &opts.text)?;
    Ok(0)
}

fn shape<'a, P: FontTableProvider>(
    mut font: FontDataImpl<P>,
    script: u32,
    lang: u32,
    text: &str,
) -> Result<(), ShapingError> {
    let opt_gsub_cache = font.gsub_cache()?;
    let opt_gpos_cache = font.gpos_cache()?;
    let opt_gdef_table = font.gdef_table()?;
    let opt_gdef_table = opt_gdef_table.as_ref().map(Rc::as_ref);

    // Map glyphs
    //
    // We look ahead in the char stream for variation selectors. If one is found it is used for
    // mapping the current glyph. When a variation selector is reached in the stream it is skipped
    // as it was handled as part of the preceding character.
    let mut chars_iter = text.chars().peekable();
    let mut glyphs = Vec::new();
    while let Some(ch) = chars_iter.next() {
        match VariationSelector::try_from(ch) {
            Ok(_) => {} // filter out variation selectors
            Err(()) => {
                let vs = chars_iter
                    .peek()
                    .and_then(|&next| VariationSelector::try_from(next).ok());
                // TODO: Remove cast when lookup_glyph_index returns u16
                let glyph_index = font.lookup_glyph_index(ch as u32) as u16;
                let glyph = glyph::make(ch, glyph_index, vs);
                glyphs.push(glyph);
            }
        }
    }

    // Apply gsub if table is present
    println!("glyphs before: {:#?}", glyphs);
    if let Some(gsub_cache) = opt_gsub_cache {
        gsub_apply_default(
            &|| make_dotted_circle(&font),
            &gsub_cache,
            opt_gdef_table,
            script,
            lang,
            GsubFeatureMask::default(),
            font.num_glyphs(),
            &mut glyphs,
        )?;
        println!("glyphs after: {:#?}", glyphs);

        // Apply gpos if table is present
        if let Some(gpos_cache) = opt_gpos_cache {
            let kerning = true;
            let mut infos = Info::init_from_glyphs(opt_gdef_table, glyphs)?;
            gpos_apply(
                &gpos_cache,
                opt_gdef_table,
                kerning,
                script,
                lang,
                &mut infos,
            )?;
        }
    } else {
        eprintln!("no GSUB table");
    }
    Ok(())
}

fn make_dotted_circle<P: FontTableProvider>(font_data_impl: &FontDataImpl<P>) -> Vec<RawGlyph<()>> {
    // TODO: Remove cast when lookup_glyph_index returns u16
    let glyph_index = font_data_impl.lookup_glyph_index(DOTTED_CIRCLE as u32) as u16;
    vec![glyph::make(DOTTED_CIRCLE, glyph_index, None)]
}
