use std::collections::HashMap;

use allsorts::cff::CFF;
use allsorts::context::Glyph;
use allsorts::error::ParseError;
use allsorts::glyph_position::{GlyphLayout, GlyphPosition, TextDirection};
use allsorts::gpos::Info;
use allsorts::outline::{OutlineBuilder, OutlineSink};
use allsorts::pathfinder_geometry::line_segment::LineSegment2F;
use allsorts::pathfinder_geometry::transform2d::Matrix2x2F;
use allsorts::pathfinder_geometry::vector::{vec2f, Vector2F};
use allsorts::post::PostTable;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::FontTableProvider;
use allsorts::Font;
use xmlwriter::XmlWriter;

use crate::BoxError;

struct Symbol<'info> {
    glyph_name: String,
    path: String,
    info: &'info Info,
}

pub trait GlyphName {
    fn gid_to_glyph_name(&self, gid: u16) -> Option<String>;
}

pub struct GlyfPost<'a> {
    pub glyf: GlyfTable<'a>,
    pub post: Option<PostTable<'a>>,
}

impl<'a> GlyphName for CFF<'a> {
    fn gid_to_glyph_name(&self, glyph_id: u16) -> Option<String> {
        let font = self.fonts.first()?;
        if font.is_cid_keyed() {
            return None;
        }
        let sid = font.charset.id_for_glyph(glyph_id)?;
        self.read_string(sid).ok()
    }
}

impl<'a> GlyphName for GlyfPost<'a> {
    fn gid_to_glyph_name(&self, glyph_id: u16) -> Option<String> {
        self.post
            .as_ref()
            .and_then(|post| post.glyph_name(glyph_id).ok().flatten())
            .map(|s| s.to_string())
    }
}

impl<'a> OutlineBuilder for GlyfPost<'a> {
    type Error = ParseError;

    fn visit<V: OutlineSink>(
        &mut self,
        glyph_index: u16,
        visitor: &mut V,
    ) -> Result<(), Self::Error> {
        self.glyf.visit(glyph_index, visitor)
    }
}

pub struct SVGWriter {
    id_prefix: Option<String>,
    transform: Matrix2x2F,
    usage: Vec<(usize, Vector2F)>,
}

struct Symbols<'info> {
    transform: Matrix2x2F,
    symbols: Vec<Symbol<'info>>,
}

impl SVGWriter {
    pub fn new(id_prefix: Option<String>, transform: Matrix2x2F) -> Self {
        SVGWriter {
            id_prefix,
            transform,
            usage: Vec::new(),
        }
    }

    pub fn glyphs_to_svg<F, T>(
        self,
        builder: &mut T,
        font: &mut Font<F>,
        infos: &[Info],
        direction: TextDirection,
    ) -> Result<String, BoxError>
    where
        T: OutlineBuilder + GlyphName,
        F: FontTableProvider,
    {
        let mut layout = GlyphLayout::new(font, infos, direction, false);
        let glyph_positions = layout.glyph_positions()?;
        let iter = infos.iter().zip(glyph_positions.iter().copied());
        let svg = match direction {
            TextDirection::LeftToRight => self.glyphs_to_svg_impl(builder, font, iter),
            TextDirection::RightToLeft => self.glyphs_to_svg_impl(builder, font, iter.rev()),
        }
        .map_err(|err| format!("error building SVG: {}", err))?;
        Ok(svg)
    }

    fn glyphs_to_svg_impl<'infos, F, T, I>(
        mut self,
        builder: &mut T,
        font: &mut Font<F>,
        iter: I,
    ) -> Result<String, T::Error>
    where
        T: OutlineBuilder + GlyphName,
        F: FontTableProvider,
        I: Iterator<Item = (&'infos Info, GlyphPosition)>,
    {
        // Turn each glyph into an SVG...
        let mut x = 0.;
        let mut y = 0.;
        let mut symbols = Symbols {
            transform: self.transform,
            symbols: Vec::new(),
        };
        let mut symbol_map = HashMap::new();
        for (info, pos) in iter {
            let glyph_index = info.get_glyph_index();
            if let Some(&symbol_index) = symbol_map.get(&glyph_index) {
                self.use_glyph(
                    symbol_index,
                    x + pos.x_offset as f32,
                    y + pos.y_offset as f32,
                )
            } else {
                let glyph_name = builder
                    .gid_to_glyph_name(glyph_index)
                    .unwrap_or_else(|| format!("gid{}", glyph_index));
                let symbol_index = symbols.new_glyph(glyph_name, info);
                symbol_map.insert(glyph_index, symbol_index);
                builder.visit(glyph_index, &mut symbols)?;
                self.use_glyph(
                    symbol_index,
                    x + pos.x_offset as f32,
                    y + pos.y_offset as f32,
                );
            }
            x += pos.hori_advance as f32;
            y += pos.vert_advance as f32;
        }

        Ok(self.end(
            x,
            font.hhea_table.ascender,
            font.hhea_table.descender,
            symbols,
        ))
    }

    fn use_glyph(&mut self, symbol_index: usize, x: f32, y: f32) {
        self.usage
            .push((symbol_index, self.transform * vec2f(x, y)));
    }

    fn end(self, x_max: f32, ascender: i16, descender: i16, symbols: Symbols) -> String {
        let mut w = XmlWriter::new(xmlwriter::Options::default());
        w.write_declaration();
        w.start_element("svg");
        w.write_attribute("version", "1.1");
        w.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        w.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        let width = self.transform.extract_scale().x() * x_max;
        let ascender = self.transform.extract_scale().y() * f32::from(ascender);
        let descender = self.transform.extract_scale().y() * f32::from(descender);
        let height = ascender - descender;
        let is_flipped = self.transform.m22() < 0.0;
        let min_y = if is_flipped { -ascender } else { descender };
        let view_box = format!(
            "{} {} {} {}",
            0,
            min_y.round(),
            width.round(),
            height.round()
        );
        w.write_attribute("viewBox", &view_box);

        // Write symbols
        for symbol in &symbols.symbols {
            w.start_element("symbol");
            let id = SVGWriter::format_id(&self.id_prefix, &symbol.glyph_name);
            // let class = SVGWriter::class(&symbol.glyph_name);
            w.write_attribute("id", &id);
            w.write_attribute("overflow", "visible");
            w.start_element("path");
            w.write_attribute("d", &symbol.path);
            w.end_element();
            w.end_element();
        }

        // Write use statements
        for (symbol_index, point) in self.usage {
            w.start_element("use");
            let symbol = &symbols.symbols[symbol_index];
            let id = SVGWriter::format_id(&self.id_prefix, &symbol.glyph_name);
            let href = format!("#{}", id);
            w.write_attribute("xlink:href", &href);
            w.write_attribute("x", &point.x().round());
            w.write_attribute("y", &point.y().round());
            w.end_element();
        }

        w.end_document()
    }

    fn format_id(id_prefix: &Option<String>, glyph_name: &str) -> String {
        match id_prefix {
            Some(id_prefix) => format!("{}.{}", id_prefix, glyph_name),
            None => glyph_name.to_owned(),
        }
    }
}

impl<'info> Symbols<'info> {
    fn new_glyph(&mut self, glyph_name: String, info: &'info Info) -> usize {
        let index = self.symbols.len();
        self.symbols.push(Symbol::new(glyph_name, info));
        index
    }

    fn current_path(&mut self) -> &mut String {
        &mut self.symbols.last_mut().unwrap().path
    }
}

impl<'info> Symbol<'info> {
    fn new(glyph_name: String, info: &'info Info) -> Self {
        Symbol {
            glyph_name,
            path: String::new(),
            info,
        }
    }
}

impl<'info> OutlineSink for Symbols<'info> {
    fn move_to(&mut self, point: Vector2F) {
        let point = self.transform * point;
        self.current_path()
            .push_str(&format!(" M{},{}", point.x(), point.y()));
    }

    fn line_to(&mut self, point: Vector2F) {
        let point = self.transform * point;
        self.current_path()
            .push_str(&format!(" L{},{}", point.x(), point.y()));
    }

    fn quadratic_curve_to(&mut self, control: Vector2F, point: Vector2F) {
        let control = self.transform * control;
        let point = self.transform * point;
        self.current_path().push_str(&format!(
            " Q{},{} {},{}",
            control.x(),
            control.y(),
            point.x(),
            point.y()
        ));
    }

    fn cubic_curve_to(&mut self, ctrl: LineSegment2F, to: Vector2F) {
        let ctrl_from = self.transform * ctrl.from();
        let ctrl_to = self.transform * ctrl.to();
        let to = self.transform * to;
        self.current_path().push_str(&format!(
            " C{},{} {},{} {},{}",
            ctrl_from.x(),
            ctrl_from.y(),
            ctrl_to.x(),
            ctrl_to.y(),
            to.x(),
            to.y()
        ));
    }

    fn close(&mut self) {
        self.current_path().push_str(" Z"); // close path
    }
}
