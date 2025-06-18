use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use allsorts::cff::outline::CFFOutlines;
use allsorts::context::Glyph;
use allsorts::glyph_position::{GlyphLayout, GlyphPosition, TextDirection};
use allsorts::gpos::{Info, Placement};
use allsorts::gsub::GlyphOrigin;
use allsorts::outline::{OutlineBuilder, OutlineSink};
use allsorts::pathfinder_geometry::line_segment::LineSegment2F;
use allsorts::pathfinder_geometry::transform2d::Matrix2x2F;
use allsorts::pathfinder_geometry::vector::{vec2f, Vector2F, Vector2I};
use allsorts::post::PostTable;
use allsorts::tables::variable_fonts::OwnedTuple;
use allsorts::tables::FontTableProvider;
use allsorts::Font;
use xmlwriter::XmlWriter;

use crate::BoxError;

struct Symbol<'info> {
    glyph_name: String,
    path: String,
    info: &'info Info,
    origin: Option<Vector2F>,
}

pub trait GlyphName {
    fn gid_to_glyph_name(&self, gid: u16) -> Option<String>;
}

pub struct NamedOutliner<'a, T> {
    pub table: T,
    pub post: Option<PostTable<'a>>,
}

/// A margin for the SVG
///
/// Fields are (top, right, bottom, left) I.e. the same order as CSS.
#[derive(Debug, Default, Copy, Clone)]
pub struct Margin {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl FromStr for Margin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s
            .split(',')
            .map(|part| part.parse())
            .collect::<Result<Vec<f32>, _>>()
            .map_err(|err| err.to_string())?;
        match parts.as_slice() {
            &[top, right, bottom, left] => Ok(Margin {
                top,
                right,
                bottom,
                left,
            }),
            &[num] => Ok(Margin {
                top: num,
                right: num,
                bottom: num,
                left: num,
            }),
            _ => Err(format!(
                "Expected margin of either a single number or 4 numbers, got {}",
                parts.len()
            )),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl FromStr for Colour {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 8 {
            return Err(String::from(
                "colour is not of the form: four hex values RRGGBBAA",
            ));
        }

        let values = s
            .as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())?;
        Ok(Colour {
            r: values[0],
            g: values[1],
            b: values[2],
            a: values[3],
        })
    }
}

impl Display for Colour {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Colour { r, g, b, a: _ } = self;
        write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
    }
}

impl Colour {
    pub fn opacity(&self) -> f32 {
        self.a as f32 / 255.
    }
}

struct ViewBox {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Display for ViewBox {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ViewBox {
            x,
            y,
            width,
            height,
        } = self;
        write!(f, "{} {} {} {}", x, y, width, height)
    }
}

impl<'a, 'data> GlyphName for CFFOutlines<'a, 'data> {
    fn gid_to_glyph_name(&self, glyph_id: u16) -> Option<String> {
        let font = self.table.fonts.first()?;
        if font.is_cid_keyed() {
            return None;
        }
        let sid = font.charset.id_for_glyph(glyph_id)?;
        self.table.read_string(sid).map(ToString::to_string).ok()
    }
}

impl<'a, T> GlyphName for NamedOutliner<'a, T> {
    fn gid_to_glyph_name(&self, glyph_id: u16) -> Option<String> {
        self.post
            .as_ref()
            .and_then(|post| post.glyph_name(glyph_id).ok().flatten())
            .map(ToString::to_string)
    }
}

impl<'a, T> OutlineBuilder for NamedOutliner<'a, T>
where
    T: OutlineBuilder,
{
    type Error = T::Error;

    fn visit<V: OutlineSink>(
        &mut self,
        glyph_index: u16,
        tuple: Option<&OwnedTuple>,
        visitor: &mut V,
    ) -> Result<(), Self::Error> {
        self.table.visit(glyph_index, tuple, visitor)
    }
}

#[derive(Clone)]
pub enum SVGMode {
    /// SVGs are being generated to comply with the expected output of the
    /// [Unicode text rendering tests](https://github.com/unicode-org/text-rendering-tests).
    ///
    /// The String is the testcase name to be used as a prefix on ids.
    TextRenderingTests(String),
    /// SVGs are being generated for human viewing
    View {
        mark_origin: bool,
        margin: Margin,
        fg: Option<Colour>,
        bg: Option<Colour>,
    },
}

pub struct SVGWriter {
    mode: SVGMode,
    transform: Matrix2x2F,
    usage: Vec<(usize, Vector2F)>,
}

struct Symbols<'info> {
    transform: Matrix2x2F,
    symbols: Vec<Symbol<'info>>,
    mode: SVGMode,
    initial_move_to: Vector2I,
    last_line_to: Option<Vector2I>,
}

impl SVGWriter {
    pub fn new(mode: SVGMode, transform: Matrix2x2F) -> Self {
        SVGWriter {
            mode,
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
        tuple: Option<&OwnedTuple>,
    ) -> Result<String, BoxError>
    where
        T: OutlineBuilder + GlyphName,
        F: FontTableProvider,
    {
        let mut layout = GlyphLayout::new(font, infos, direction, false);
        let glyph_positions = layout.glyph_positions()?;
        let iter = infos.iter().zip(glyph_positions.iter().copied());
        let svg = match direction {
            TextDirection::LeftToRight => self.glyphs_to_svg_impl(builder, font, tuple, iter),
            TextDirection::RightToLeft => self.glyphs_to_svg_impl(builder, font, tuple, iter.rev()),
        }
        .map_err(|err| format!("error building SVG: {}", err))?;
        Ok(svg)
    }

    fn glyphs_to_svg_impl<'infos, F, T, I>(
        mut self,
        builder: &mut T,
        font: &mut Font<F>,
        tuple: Option<&OwnedTuple>,
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
            mode: self.mode.clone(),
            initial_move_to: Vector2I::zero(),
            last_line_to: None,
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
                builder.visit(glyph_index, tuple, &mut symbols)?;
                if self.annotate() {
                    symbols.annotate(symbol_index, pos.x_offset as f32, pos.y_offset as f32);
                }
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
        let view_box = self.view_box(x_max, f32::from(ascender), f32::from(descender));
        w.write_attribute("viewBox", &view_box);
        if let Some(colour) = self.bg_colour() {
            w.start_element("rect");
            w.write_attribute("x", &view_box.x);
            w.write_attribute("y", &view_box.y);
            w.write_attribute("width", &view_box.width);
            w.write_attribute("height", &view_box.height);
            w.write_attribute("fill", &colour);
            if colour.opacity() != 1. {
                w.write_attribute("fill-opacity", &colour.opacity());
            }
            w.end_element()
        }

        // Write symbols
        for symbol in &symbols.symbols {
            w.start_element("symbol");
            w.write_attribute("id", &symbol.id(&self.mode));
            for (key, value) in symbol.data(&self.mode) {
                w.write_attribute(key, &value);
            }
            w.write_attribute("overflow", "visible");
            w.start_element("path");
            w.write_attribute("d", &symbol.path);
            if let Some(colour) = self.fg_colour() {
                w.write_attribute("fill", &colour);
                if colour.opacity() != 1. {
                    w.write_attribute("fill-opacity", &colour.opacity());
                }
            }
            w.end_element();
            if let Some(origin) = symbol.origin {
                w.start_element("path");
                w.write_attribute("d", &self.crosshair_path(origin));
                w.write_attribute("stroke", "red");
                w.write_attribute("stroke-width", &(self.transform.extract_scale().x() * 10.));
                w.end_element();
            }
            w.end_element();
        }

        // Write use statements
        for (symbol_index, point) in self.usage {
            w.start_element("use");
            let symbol = &symbols.symbols[symbol_index];
            w.write_attribute("xlink:href", &format!("#{}", symbol.id(&self.mode)));
            w.write_attribute("x", &point.x().round());
            w.write_attribute("y", &point.y().round());
            w.end_element();
        }

        w.end_document()
    }

    fn view_box(&self, x_max: f32, ascender: f32, descender: f32) -> ViewBox {
        let Margin {
            top,
            right,
            bottom,
            left,
        } = self.margin();
        let is_flipped = self.transform.m22() < 0.0;
        let min_y = if is_flipped { -ascender } else { descender };
        let scale_x = self.transform.extract_scale().x();
        let scale_y = self.transform.extract_scale().y();

        let x = ((0. - left) * scale_x).round() as i32;
        let y = ((min_y - top) * scale_y).round() as i32;
        let width = ((x_max + left + right) * scale_x).round() as i32;
        let height = ((ascender - descender + top + bottom) * scale_y).round() as i32;
        ViewBox {
            x,
            y,
            width,
            height,
        }
    }

    fn crosshair_path(&self, origin: Vector2F) -> String {
        let x = origin.x();
        let y = origin.y();
        let crosshair_size = 100. * self.transform.extract_scale().x();
        let xl = x - crosshair_size;
        let xr = x + crosshair_size;
        let yb = y - crosshair_size;
        let yt = y + crosshair_size;
        format!("M{},{} L{},{} M{},{} L{},{}", xl, y, xr, y, x, yb, x, yt)
    }

    fn annotate(&self) -> bool {
        matches!(
            self.mode,
            SVGMode::View {
                mark_origin: true,
                ..
            }
        )
    }

    fn margin(&self) -> Margin {
        match self.mode {
            SVGMode::TextRenderingTests(_) => Margin::default(),
            SVGMode::View { margin, .. } => margin,
        }
    }

    fn fg_colour(&self) -> Option<Colour> {
        match self.mode {
            SVGMode::TextRenderingTests(_) => None,
            SVGMode::View { fg, .. } => fg,
        }
    }

    fn bg_colour(&self) -> Option<Colour> {
        match self.mode {
            SVGMode::TextRenderingTests(_) => None,
            SVGMode::View { bg, .. } => bg,
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

    fn annotate(&mut self, index: usize, x: f32, y: f32) {
        self.symbols[index].annotate(vec2f(x, y));
    }
}

impl<'info> Symbol<'info> {
    fn new(glyph_name: String, info: &'info Info) -> Self {
        Symbol {
            glyph_name,
            path: String::new(),
            info,
            origin: None,
        }
    }

    fn id(&self, mode: &SVGMode) -> Cow<'_, str> {
        match mode {
            SVGMode::TextRenderingTests(id_prefix) => {
                format!("{}.{}", id_prefix, self.glyph_name).into()
            }
            SVGMode::View { .. } => Cow::from(&self.glyph_name),
        }
    }

    fn data(&self, mode: &SVGMode) -> HashMap<&'static str, String> {
        match mode {
            SVGMode::TextRenderingTests(_) => HashMap::new(),
            SVGMode::View { .. } => {
                let bool_true = String::from("true");
                let mut data = HashMap::new();
                if matches!(
                    self.info.placement,
                    Placement::MarkAnchor(_, _, _) | Placement::MarkOverprint(_)
                ) {
                    data.insert("data-mark", bool_true.clone());
                }
                data.insert("data-glyph-index", self.info.glyph.glyph_index.to_string());
                data.insert(
                    "data-liga-component-pos",
                    self.info.glyph.liga_component_pos.to_string(),
                );
                data.insert(
                    "data-glyph-origin",
                    match self.info.glyph.glyph_origin {
                        GlyphOrigin::Char(_) => String::from("char"),
                        GlyphOrigin::Direct => String::from("direct"),
                    },
                );
                if self.info.glyph.small_caps() {
                    data.insert("data-small-caps", bool_true.clone());
                }
                if self.info.glyph.multi_subst_dup() {
                    data.insert("data-multi-subst-dup", bool_true.clone());
                }
                if self.info.glyph.is_vert_alt() {
                    data.insert("data-is-vert-alt", bool_true.clone());
                }
                if self.info.glyph.fake_bold() {
                    data.insert("data-fake-bold", bool_true.clone());
                }
                if self.info.glyph.fake_italic() {
                    data.insert("data-fake-italic", bool_true.clone());
                }
                data
            }
        }
    }

    fn annotate(&mut self, origin: Vector2F) {
        self.origin = Some(origin);
    }
}

// When rendering in TextRenderingTests mode the paths are "normalised" by
// truncating them. The matches what the other test harnesses do and makes the
// output SVGs match the expectations, which have had the same treatment.
//
// Additionally, the expected SVGs in the test suite require matching a
// FreeType optimisation where a line-to back to the start of the path
// is dropped, as close-path will handle that.
impl<'info> OutlineSink for Symbols<'info> {
    fn move_to(&mut self, point: Vector2F) {
        let point = self.transform * point;
        let path = match self.mode {
            SVGMode::TextRenderingTests(_) => {
                let point = Vector2I::new(point.x() as i32, point.y() as i32);
                self.initial_move_to = point;
                self.last_line_to = None;
                format!(" M{},{}", point.x(), point.y())
            }
            SVGMode::View { .. } => format!(" M{},{}", point.x(), point.y()),
        };
        self.current_path().push_str(&path);
    }

    fn line_to(&mut self, point: Vector2F) {
        let point = self.transform * point;
        let path = match self.mode {
            SVGMode::TextRenderingTests(_) => {
                let point = Vector2I::new(point.x() as i32, point.y() as i32);
                self.last_line_to = Some(point);
                format!(" L{},{}", point.x(), point.y())
            }
            SVGMode::View { .. } => format!(" L{},{}", point.x(), point.y()),
        };
        self.current_path().push_str(&path);
    }

    fn quadratic_curve_to(&mut self, control: Vector2F, point: Vector2F) {
        let control = self.transform * control;
        let point = self.transform * point;
        let path = match self.mode {
            SVGMode::TextRenderingTests(_) => {
                self.last_line_to = None;
                format!(
                    " Q{},{} {},{}",
                    control.x() as i32,
                    control.y() as i32,
                    point.x() as i32,
                    point.y() as i32
                )
            }
            SVGMode::View { .. } => format!(
                " Q{},{} {},{}",
                control.x(),
                control.y(),
                point.x(),
                point.y()
            ),
        };
        self.current_path().push_str(&path);
    }

    fn cubic_curve_to(&mut self, ctrl: LineSegment2F, to: Vector2F) {
        let ctrl_from = self.transform * ctrl.from();
        let ctrl_to = self.transform * ctrl.to();
        let to = self.transform * to;
        let path = match self.mode {
            SVGMode::TextRenderingTests(_) => {
                self.last_line_to = None;
                format!(
                    " C{},{} {},{} {},{}",
                    ctrl_from.x() as i32,
                    ctrl_from.y() as i32,
                    ctrl_to.x() as i32,
                    ctrl_to.y() as i32,
                    to.x() as i32,
                    to.y() as i32
                )
            }
            SVGMode::View { .. } => format!(
                " C{},{} {},{} {},{}",
                ctrl_from.x(),
                ctrl_from.y(),
                ctrl_to.x(),
                ctrl_to.y(),
                to.x(),
                to.y()
            ),
        };
        self.current_path().push_str(&path);
    }

    fn close(&mut self) {
        if matches!(self.mode, SVGMode::TextRenderingTests(_)) {
            match self.last_line_to {
                Some(last_line_to) if last_line_to == self.initial_move_to => {
                    // Suppress last line to
                    if let Some(m_pos) = self.current_path().rfind(" L") {
                        self.current_path().truncate(m_pos);
                    }
                }
                _ => {}
            }
        }

        self.current_path().push_str(" Z"); // close path
    }
}
