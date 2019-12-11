use allsorts::binary::read::{ReadScope, ReadScopeOwned};
use allsorts::font_data_impl::read_cmap_subtable;
use allsorts::gpos::{gpos_apply, Info, Placement};
use allsorts::gsub::{gsub_apply_default, GlyphOrigin, RawGlyph};
use allsorts::layout::{new_layout_cache, GDEFTable, LayoutCache, LayoutTable, GPOS, GSUB};
use allsorts::tables::cmap::{Cmap, CmapSubtable};
use allsorts::tables::{HheaTable, HmtxTable, MaxpTable, OpenTypeFile, OpenTypeFont};
use allsorts::tag;
use anyhow::{anyhow, Result};
use std::mem::transmute;

#[derive(Debug)]
enum ResolvedGlyph {
    Resolved(Info),
    Unresolved(RawGlyph<()>),
}

#[derive(Debug)]
pub struct Shaped {
    info: ResolvedGlyph,
    font_index: usize,
    x_advance: i32,
    y_advance: i32,
}

fn main() -> Result<()> {
    let fonts = [
        "/home/wez/.dotfiles/fonts/OperatorMonoLig-Medium.otf",
        "/home/wez/.dotfiles/fonts/NotoColorEmoji.ttf",
    ];

    let mut loaded_fonts = vec![];
    for f in &fonts {
        loaded_fonts.push(LoadedFont::load_font(f)?);
    }

    let pos = shape(
        "A->B||❤.",
        tag::from_string("DFLT")?,
        tag::from_string("dflt")?,
        &loaded_fonts,
    )?;
    println!("pos: {:#?}", pos);
    Ok(())
}

fn shape<T: AsRef<str>>(
    text: T,
    script: u32,
    lang: u32,
    loaded_fonts: &[LoadedFont],
) -> Result<Vec<Shaped>> {
    let mut results = vec![];
    shape_into(0, text.as_ref(), script, lang, loaded_fonts, &mut results)?;
    Ok(results)
}

fn shape_into(
    font_index: usize,
    text: &str,
    script: u32,
    lang: u32,
    loaded_fonts: &[LoadedFont],
    results: &mut Vec<Shaped>,
) -> Result<()> {
    let first_pass = loaded_fonts[font_index].shape_text(text, font_index, script, lang)?;

    let mut item_iter = first_pass.into_iter();
    let mut fallback_run = String::new();
    while let Some(item) = item_iter.next() {
        match &item.info {
            ResolvedGlyph::Resolved(_) => results.push(item),
            ResolvedGlyph::Unresolved(raw) => {
                // There was no glyph in that font, so we'll need to shape
                // using a fallback.  Let's collect together any potential
                // run of unresolved entries first
                for &c in &raw.unicodes {
                    fallback_run.push(c);
                }

                while let Some(item) = item_iter.next() {
                    match &item.info {
                        ResolvedGlyph::Unresolved(raw) => {
                            for &c in &raw.unicodes {
                                fallback_run.push(c);
                            }
                        }
                        ResolvedGlyph::Resolved(_) => {
                            shape_into(
                                font_index + 1,
                                &fallback_run,
                                script,
                                lang,
                                loaded_fonts,
                                results,
                            )?;
                            fallback_run.clear();
                            results.push(item);
                        }
                    }
                }
            }
        }
    }

    if !fallback_run.is_empty() {
        shape_into(
            font_index + 1,
            &fallback_run,
            script,
            lang,
            loaded_fonts,
            results,
        )?;
    }

    Ok(())
}

struct LoadedFont {
    cmap_subtable: CmapSubtable<'static>,
    gpos_cache: Option<LayoutCache<GPOS>>,
    gsub_cache: LayoutCache<GSUB>,
    gdef_table: Option<GDEFTable>,
    hmtx: HmtxTable<'static>,
    hhea: HheaTable,
    num_glyphs: u16,

    // Must be last: this keeps the 'static items alive
    _scope: ReadScopeOwned,
}

impl LoadedFont {
    fn load_font<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path.as_ref())?;
        let owned_scope = ReadScopeOwned::new(ReadScope::new(&data));

        // This unsafe block and transmute are present so that we can
        // extend the lifetime of the OpenTypeFile that we produce here.
        // That in turn allows us to store all of these derived items
        // into a struct and manage their lifetimes together.
        let file: OpenTypeFile<'static> =
            unsafe { transmute(owned_scope.scope().read::<OpenTypeFile>()?) };

        let otf = match &file.font {
            OpenTypeFont::Single(v) => v,
            _ => panic!(),
        };

        let cmap = otf
            .read_table(&file.scope, tag::CMAP)?
            .ok_or_else(|| anyhow!("CMAP table missing or broken"))?
            .read::<Cmap>()?;
        let cmap_subtable: CmapSubtable<'static> =
            read_cmap_subtable(&cmap)?.ok_or_else(|| anyhow!("CMAP subtable not found"))?;

        let maxp = otf
            .read_table(&file.scope, tag::MAXP)?
            .ok_or_else(|| anyhow!("MAXP table not found"))?
            .read::<MaxpTable>()?;
        let num_glyphs = maxp.num_glyphs;

        let hhea = otf
            .read_table(&file.scope, tag::HHEA)?
            .ok_or_else(|| anyhow!("HHEA table not found"))?
            .read::<HheaTable>()?;
        let hmtx = otf
            .read_table(&file.scope, tag::HMTX)?
            .ok_or_else(|| anyhow!("HMTX table not found"))?
            .read_dep::<HmtxTable>((
                usize::from(maxp.num_glyphs),
                usize::from(hhea.num_h_metrics),
            ))?;

        let gsub_table = otf
            .find_table_record(tag::GSUB)
            .ok_or_else(|| anyhow!("GSUB table record not found"))?
            .read_table(&file.scope)?
            .read::<LayoutTable<GSUB>>()?;
        let gdef_table: Option<GDEFTable> = otf
            .find_table_record(tag::GDEF)
            .map(|gdef_record| -> Result<GDEFTable> {
                Ok(gdef_record.read_table(&file.scope)?.read::<GDEFTable>()?)
            })
            .transpose()?;
        let opt_gpos_table = otf
            .find_table_record(tag::GPOS)
            .map(|gpos_record| -> Result<LayoutTable<GPOS>> {
                Ok(gpos_record
                    .read_table(&file.scope)?
                    .read::<LayoutTable<GPOS>>()?)
            })
            .transpose()?;
        let gsub_cache = new_layout_cache(gsub_table);
        let gpos_cache = opt_gpos_table.map(new_layout_cache);

        Ok(Self {
            cmap_subtable,
            hmtx,
            hhea,
            gpos_cache,
            gsub_cache,
            gdef_table,
            num_glyphs,
            _scope: owned_scope,
        })
    }

    fn glyph_index_for_char(&self, c: char) -> Result<Option<u16>> {
        self.cmap_subtable
            .map_glyph(c as u32)
            .map_err(|e| anyhow!("Error while looking up glyph {}: {}", c, e))
    }

    pub fn shape_text<T: AsRef<str>>(
        &self,
        text: T,
        font_index: usize,
        script: u32,
        lang: u32,
    ) -> Result<Vec<Shaped>> {
        let mut glyphs = vec![];
        for c in text.as_ref().chars() {
            glyphs.push(RawGlyph {
                unicodes: vec![c],
                glyph_index: self.glyph_index_for_char(c)?,
                liga_component_pos: 0,
                glyph_origin: GlyphOrigin::Char(c),
                small_caps: false,
                multi_subst_dup: false,
                is_vert_alt: false,
                fake_bold: false,
                fake_italic: false,
                extra_data: (),
            });
        }

        let vertical = false;

        gsub_apply_default(
            &|| vec![], //map_char('\u{25cc}')],
            &self.gsub_cache,
            self.gdef_table.as_ref(),
            script,
            lang,
            vertical,
            self.num_glyphs,
            &mut glyphs,
        )?;

        // Note: init_from_glyphs silently elides entries that
        // have no glyph in the current font!  we need to deal
        // with this so that we can perform font fallback, so
        // we pass a copy of the glyphs here and detect this
        // during glyph_positions().
        let mut infos = Info::init_from_glyphs(self.gdef_table.as_ref(), glyphs.clone())?;
        if let Some(gpos_cache) = self.gpos_cache.as_ref() {
            let kerning = true;

            gpos_apply(
                gpos_cache,
                self.gdef_table.as_ref(),
                kerning,
                script,
                lang,
                &mut infos,
            )?;
        }

        self.glyph_positions(font_index, infos, glyphs)
    }

    fn glyph_positions(
        &self,
        font_index: usize,
        infos: Vec<Info>,
        glyphs: Vec<RawGlyph<()>>,
    ) -> Result<Vec<Shaped>> {
        let mut pos: Vec<Shaped> = Vec::new();

        let mut glyph_iter = glyphs.into_iter();

        for glyph_info in infos.into_iter() {
            let mut input_glyph = glyph_iter
                .next()
                .ok_or_else(|| anyhow!("more output infos than input glyphs!"))?;

            while input_glyph.unicodes != glyph_info.glyph.unicodes {
                // Info::init_from_glyphs skipped the input glyph, so let's be
                // sure to emit a placeholder for it
                pos.push(Shaped {
                    info: ResolvedGlyph::Unresolved(input_glyph),
                    font_index,
                    x_advance: 0,
                    y_advance: 0,
                });

                input_glyph = glyph_iter
                    .next()
                    .ok_or_else(|| anyhow!("more output infos than input glyphs! (loop bottom)"))?;
            }

            let horizontal_advance = i32::from(
                self.hmtx.horizontal_advance(
                    glyph_info
                        .glyph
                        .glyph_index
                        .ok_or_else(|| anyhow!("no mapped glyph_index for {:?}", glyph_info))?,
                    self.hhea.num_h_metrics,
                )?,
            );

            /*
            let width = if glyph_info.kerning != 0 {
                horizontal_advance + i32::from(glyph_info.kerning)
            } else {
                horizontal_advance
            };
            */

            // Adjust for distance placement
            match glyph_info.placement {
                Placement::Distance(dx, dy) => {
                    pos.push(Shaped {
                        info: ResolvedGlyph::Resolved(glyph_info),
                        font_index,
                        x_advance: horizontal_advance + dx,
                        y_advance: dy,
                    });
                }
                Placement::Anchor(_, _) | Placement::None => {
                    pos.push(Shaped {
                        info: ResolvedGlyph::Resolved(glyph_info),
                        font_index,
                        x_advance: horizontal_advance,
                        y_advance: 0,
                    });
                }
            }
        }

        Ok(pos)
    }
}
