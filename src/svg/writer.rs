use std::collections::HashMap;

use crate::glyph::calculate_glyph_positions;
use allsorts::context::Glyph;
use allsorts::gpos::Info;
use allsorts::outline::{OutlineBuilder, OutlineSink};
use allsorts::pathfinder_geometry::line_segment::LineSegment2F;
use allsorts::pathfinder_geometry::vector::{vec2f, Vector2F};
use allsorts::tables::FontTableProvider;
use allsorts::Font;
use xmlwriter::XmlWriter;

use super::GlyphName;

struct Symbol {
    glyph_name: String,
    path: String,
}

pub struct SVGWriter {
    testcase: String,
    flip: bool,
    symbols: Vec<Symbol>,
    usage: Vec<(usize, Vector2F)>,
}

impl SVGWriter {
    pub fn new(testcase: String, flip: bool) -> Self {
        SVGWriter {
            testcase,
            flip,
            symbols: Vec::new(),
            usage: Vec::new(),
        }
    }

    pub fn glyphs_to_svg<F, T>(
        mut self,
        builder: &mut T,
        font: &mut Font<F>,
        infos: &[Info],
    ) -> Result<String, T::Error>
    where
        T: OutlineBuilder + GlyphName,
        F: FontTableProvider,
    {
        // Turn each glyph into an SVG...
        let mut x = 0.;
        let mut y = 0.;
        let glyph_positions =
            calculate_glyph_positions(font, infos, false).expect("FIXME calculate_glyph_positions");
        let glyph_positions_iter = glyph_positions
            .iter()
            .copied()
            .map(|(a, x, y)| (a as f32, x as f32, y as f32));
        let mut symbols = HashMap::new();
        for (info, (advance, glyph_x, glyph_y)) in infos.iter().zip(glyph_positions_iter) {
            let glyph_index = info.get_glyph_index();
            if let Some(&symbol_index) = symbols.get(&glyph_index) {
                self.use_glyph(symbol_index, glyph_x, glyph_y)
            } else {
                let glyph_name = builder
                    .gid_to_glyph_name(glyph_index)
                    .unwrap_or_else(|| format!("gid{}", glyph_index));
                let symbol_index = self.new_glyph(glyph_name);
                symbols.insert(glyph_index, symbol_index);
                builder.visit(glyph_index, &mut self)?;
                self.use_glyph(symbol_index, glyph_x, glyph_y);
            }
            x += advance;
        }

        Ok(self.end(x, font.hhea_table.ascender, font.hhea_table.descender))
    }

    fn new_glyph(&mut self, glyph_name: String) -> usize {
        let index = self.symbols.len();
        self.symbols.push(Symbol::new(glyph_name));
        index
    }

    fn use_glyph(&mut self, symbol_index: usize, x: f32, y: f32) {
        self.usage.push((symbol_index, vec2f(x, y)));
    }

    fn end(self, x_max: f32, ascender: i16, descender: i16) -> String {
        let mut w = XmlWriter::new(xmlwriter::Options::default());
        w.write_declaration();
        w.start_element("svg");
        w.write_attribute("version", "1.1");
        w.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        w.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        let height = ascender - descender;
        let view_box = format!("{} {} {} {}", 0, descender, x_max, height);
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
            w.write_attribute("x", &point.x());
            w.write_attribute("y", &point.y());
            w.end_element();
        }

        w.end_document()
    }

    fn current_path(&mut self) -> &mut String {
        &mut self.symbols.last_mut().unwrap().path
    }

    fn flip(&self, value: f32) -> f32 {
        if self.flip {
            -value
        } else {
            value
        }
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
        let y = self.flip(point.y());
        self.current_path()
            .push_str(&format!(" M{},{}", point.x(), y));
    }

    fn line_to(&mut self, point: Vector2F) {
        let y = self.flip(point.y());
        self.current_path()
            .push_str(&format!(" L{},{}", point.x(), y));
    }

    fn quadratic_curve_to(&mut self, control: Vector2F, point: Vector2F) {
        let y1 = self.flip(control.y());
        let y = self.flip(point.y());
        self.current_path()
            .push_str(&format!(" Q{},{} {},{}", control.x(), y1, point.x(), y));
    }

    fn cubic_curve_to(&mut self, ctrl: LineSegment2F, to: Vector2F) {
        let y1 = self.flip(ctrl.from().y());
        let y2 = self.flip(ctrl.to().y());
        let y = self.flip(to.y());
        self.current_path().push_str(&format!(
            " C{},{} {},{} {},{}",
            ctrl.from().x(),
            y1,
            ctrl.to().x(),
            y2,
            to.x(),
            y
        ));
    }

    fn close(&mut self) {
        self.current_path().push_str(" Z"); // close path
    }
}
