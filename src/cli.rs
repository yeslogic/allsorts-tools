use std::ffi::OsString;

use gumdrop::Options;

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

    #[options(help = "apply shaping to glyphs from a font")]
    Shape(ShapeOpts),

    #[options(help = "subset a font")]
    Subset(SubsetOpts),

    #[options(help = "output an SVG rendition of the supplied text")]
    Svg(SvgOpts),

    #[options(help = "parse the supplied font, reporting any failures")]
    Validate(ValidateOpts),

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
#[options(help = "E.g. shape -f some.ttf -s deva -l HIN 'Some text'")]
pub struct ShapeOpts {
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

    #[options(required, help = "script to shape", meta = "SCRIPT")]
    pub script: String,

    #[options(required, help = "language to shape", meta = "LANG")]
    pub lang: String,

    #[options(free, required, help = "text to shape")]
    pub text: String,

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
        help = "index of the font to dump (for TTC, WOFF2)",
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
pub struct ViewOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(required, help = "script to shape", meta = "SCRIPT")]
    pub script: String,

    #[options(help = "language to shape", meta = "LANG")]
    pub lang: Option<String>,

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
}
