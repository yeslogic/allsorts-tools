use std::collections::HashMap;

use allsorts::context::Glyph;
use allsorts::gpos::Info;
use allsorts::outline::{OutlineBuilder, OutlineSink};
use allsorts::pathfinder_geometry::line_segment::LineSegment2F;
use allsorts::pathfinder_geometry::transform2d::Matrix2x2F;
use allsorts::pathfinder_geometry::vector::{vec2f, Vector2F};
use allsorts::tables::FontTableProvider;
use allsorts::Font;
use xmlwriter::XmlWriter;

use super::GlyphName;
use crate::glyph::{GlyphLayout, GlyphPosition, TextDirection};
use crate::BoxError;

struct Symbol {
    glyph_name: String,
    path: String,
}

pub struct SVGWriter {
    testcase: String,
    transform: Matrix2x2F,
    symbols: Vec<Symbol>,
    usage: Vec<(usize, Vector2F)>,
}

impl SVGWriter {
    pub fn new(testcase: String, transform: Matrix2x2F) -> Self {
        SVGWriter {
            testcase,
            transform,
            symbols: Vec::new(),
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
        .map_err(|err| format!("error buliding SVG: {}", err))?;
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
        let mut symbols = HashMap::new();
        for (info, pos) in iter {
            let glyph_index = info.get_glyph_index();
            if let Some(&symbol_index) = symbols.get(&glyph_index) {
                self.use_glyph(
                    symbol_index,
                    x + pos.x_offset as f32,
                    y + pos.y_offset as f32,
                )
            } else {
                let glyph_name = builder
                    .gid_to_glyph_name(glyph_index)
                    .unwrap_or_else(|| format!("gid{}", glyph_index));
                let symbol_index = self.new_glyph(glyph_name);
                symbols.insert(glyph_index, symbol_index);
                builder.visit(glyph_index, &mut self)?;
                self.use_glyph(
                    symbol_index,
                    x + pos.x_offset as f32,
                    y + pos.y_offset as f32,
                );
            }
            x += pos.hori_advance as f32;
            y += pos.vert_advance as f32;
        }

        Ok(self.end(x, font.hhea_table.ascender, font.hhea_table.descender))
    }

    fn new_glyph(&mut self, glyph_name: String) -> usize {
        let index = self.symbols.len();
        self.symbols.push(Symbol::new(glyph_name));
        index
    }

    fn use_glyph(&mut self, symbol_index: usize, x: f32, y: f32) {
        self.usage
            .push((symbol_index, self.transform * vec2f(x, y)));
    }

    fn end(self, x_max: f32, ascender: i16, descender: i16) -> String {
        let mut w = XmlWriter::new(xmlwriter::Options::default());
        w.write_declaration();
        w.start_element("svg");
        w.write_attribute("version", "1.1");
        w.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        w.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        let x_max = self.transform.extract_scale().x() * x_max;
        let ascender = self.transform.extract_scale().y() * f32::from(ascender);
        let descender = self.transform.extract_scale().y() * f32::from(descender);
        let height = ascender - descender;
        let view_box = format!(
            "{} {} {} {}",
            0,
            descender.round(),
            x_max.round(),
            height.round()
        );
        w.write_attribute("viewBox", &view_box);

        // Write symbols
        for symbol in &self.symbols {
            w.start_element("symbol");
            let id = format!("{}.{}", self.testcase, symbol.glyph_name);
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
            let symbol = &self.symbols[symbol_index];
            let href = format!("#{}.{}", self.testcase, symbol.glyph_name);
            w.write_attribute("xlink:href", &href);
            w.write_attribute("x", &point.x().round());
            w.write_attribute("y", &point.y().round());
            w.end_element();
        }

        w.end_document()
    }

    fn current_path(&mut self) -> &mut String {
        &mut self.symbols.last_mut().unwrap().path
    }
}

impl Symbol {
    fn new(glyph_name: String) -> Self {
        Symbol {
            glyph_name,
            path: String::new(),
        }
    }
}

impl OutlineSink for SVGWriter {
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
