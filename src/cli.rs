use std::ffi::OsString;

use gumdrop::Options;

use crate::writer::{Colour, Margin};

#[derive(Debug, Options)]
pub struct Cli {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(command)]
    pub command: Option<Command>,
}

#[derive(Debug, Options)]
pub enum Command {
    #[options(help = "dump bitmaps for supplied text")]
    Bitmaps(BitmapOpts),

    #[options(help = "dump the character map")]
    Cmap(CmapOpts),

    #[options(help = "dump font information")]
    Dump(DumpOpts),

    #[options(help = "check if a font has a particular table")]
    HasTable(HasTableOpts),

    #[options(help = "create a static instance from a variable font")]
    Instance(InstanceOpts),

    #[options(help = "print a list of a font's GSUB and GPOS features")]
    LayoutFeatures(LayoutFeaturesOpts),

    #[options(help = "apply shaping to glyphs from a font")]
    Shape(ShapeOpts),

    #[options(help = "subset a font")]
    Subset(SubsetOpts),

    #[options(
        help = "output an SVG rendition of the supplied text (for unicode text-rendering tests)"
    )]
    Svg(SvgOpts),

    #[options(help = "parse the supplied font, reporting any failures")]
    Validate(ValidateOpts),

    #[options(help = "print a list of a font's variations")]
    Variations(VariationsOpts),

    #[options(help = "output an SVG rendition of the supplied text")]
    View(ViewOpts),
}

#[derive(Debug, Options)]
pub struct BitmapOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(required, help = "path to directory to write to")]
    pub output: String,

    #[options(required, help = "font size to find bitmaps for")]
    pub size: u16,

    #[options(free, required, help = "text to extract bitmaps for")]
    pub text: String,
}

#[derive(Debug, Options)]
pub struct CmapOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,
}

#[derive(Debug, Options)]
pub struct DumpOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(help = "treat the file as a CFF font/table")]
    pub cff: bool,

    #[options(help = "dump the raw binary content of this table", meta = "TABLE")]
    pub table: Option<String>,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(help = "include CMAP encodings in output", no_short)]
    pub encodings: bool,

    #[options(help = "dump the specified glyph", meta = "GLYPH_ID")]
    pub glyph: Option<u16>,

    #[options(help = "include glyph names in output", no_short)]
    pub glyph_names: bool,

    #[options(help = "include strings from the name table in output", no_short)]
    pub name: bool,

    #[options(help = "print the head table", no_short)]
    pub head: bool,

    #[options(help = "print the hmtx table", no_short)]
    pub hmtx: bool,

    #[options(help = "print the loca table")]
    pub loca: bool,

    #[options(free, required, help = "path to font to dump")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct HasTableOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(help = "table to check for", meta = "TABLE")]
    pub table: String,

    #[options(
        help = "index of the font to check (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(help = "print file name")]
    pub print_file: bool,

    #[options(short = "v", help = "select fonts that don't have the given table")]
    pub invert_match: bool,

    #[options(free, required, help = "paths of fonts to check")]
    pub fonts: Vec<OsString>,
}

#[derive(Debug, Options)]
pub struct InstanceOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    // TODO: allow specifying the name of a STAT instance
    #[options(help = "comma-separated list of user-tuple values", meta = "TUPLE")]
    pub tuple: String,

    #[options(required, help = "path to destination font")]
    pub output: String,

    #[options(free, required, help = "path to input variable font file")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct LayoutFeaturesOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(free, required, help = "path to font file")]
    pub font: String,
}

#[derive(Debug, Options)]
#[options(help = "E.g. shape -f some.ttf -s deva -l HIN 'Some text'")]
pub struct ShapeOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(
        help = "index of the font to shape (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(required, help = "script to shape", meta = "SCRIPT")]
    pub script: String,

    #[options(required, help = "language to shape", meta = "LANG")]
    pub lang: String,

    #[options(free, required, help = "text to shape")]
    pub text: String,

    #[options(help = "comma-separated list of user-tuple values", meta = "TUPLE")]
    pub tuple: Option<String>,

    #[options(help = "vertical layout, default horizontal", no_short)]
    pub vertical: bool,
}

#[derive(Debug, Options)]
pub struct SubsetOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(help = "subset the font to include glyphs from TEXT", meta = "TEXT")]
    pub text: Option<String>,

    #[options(help = "include all glyphs in the subset font")]
    pub all: bool,

    #[options(
        help = "index of the font to subset (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(free, required, help = "path to source font")]
    pub input: String,

    #[options(free, required, help = "path to destination font")]
    pub output: String,
}

#[derive(Debug, Options)]
#[options(help = "Output an SVG in the format expected by the unicode text-rendering tests")]
pub struct SvgOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(
        help = "ignored, compatibility with text-rendering-tests",
        meta = "ENGINE"
    )]
    pub engine: String,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(help = "name of test case", meta = "NAME", default = "allsorts")]
    pub testcase: String,

    #[options(help = "comma-separated list of user-tuple values", meta = "TUPLE")]
    pub tuple: Option<String>,

    #[options(help = "variation settings for test case", meta = "AXES")]
    pub variation: Option<String>,

    #[options(required, help = "text to render", meta = "TEXT")]
    pub render: String,

    #[options(help = "flip output (rotate 180deg)", no_short)]
    pub flip: bool,
}

#[derive(Debug, Options)]
pub struct ValidateOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(free, required, help = "path to font")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct VariationsOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(help = "output a HTML test file alongside the font")]
    pub test: bool,

    #[options(free, required, help = "path to font file")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct ViewOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(required, help = "script to shape", meta = "SCRIPT")]
    pub script: String,

    #[options(help = "language to shape", meta = "LANG")]
    pub lang: Option<String>,

    #[options(help = "mark the origin of each glyph with a cross-hair", no_short)]
    pub mark_origin: bool,

    #[options(
        help = "specify a margin to be added to the edge of the SVG",
        meta = "num or top,right,bottom,left",
        no_short
    )]
    pub margin: Option<Margin>,

    #[options(
        help = "set the fill colour of the glyphs",
        meta = "rrggbbaa",
        no_short
    )]
    pub fg_colour: Option<Colour>,

    #[options(
        help = "set the background colour of the generated SVG",
        meta = "rrggbbaa",
        no_short
    )]
    pub bg_colour: Option<Colour>,

    #[options(help = "alias for --fg-colour", meta = "rrggbbaa", no_short)]
    pub fg_color: Option<Colour>,

    #[options(help = "alias for --bg-colour", meta = "rrggbbaa", no_short)]
    pub bg_color: Option<Colour>,

    #[options(help = "text to render")]
    pub text: Option<String>,

    #[options(
        help = "comma-separated list of codepoints (as hexadecimal numbers) to render",
        meta = "CODEPOINTS"
    )]
    pub codepoints: Option<String>,

    #[options(
        help = "comma-separated list of glyph indices to render",
        meta = "GLYPH_INDICES"
    )]
    pub indices: Option<String>,

    #[options(
        help = "comma-separated list of OpenType features to enable (note: only enables these features)",
        meta = "FEATURES"
    )]
    pub features: Option<String>,

    #[options(help = "comma-separated list of user-tuple values", meta = "TUPLE")]
    pub tuple: Option<String>,
}
