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
    #[options(help = "dump all bitmaps in EBDT/CBDT tables")]
    Bitmaps(BitmapOpts),

    #[options(help = "dump font information")]
    Dump(DumpOpts),

    #[options(help = "check if a font has a particular table")]
    HasTable(HasTableOpts),

    #[options(help = "parse the supplied font, reporting any failures")]
    Validate(ValidateOpts),

    #[options(help = "subset a font")]
    Subset(SubsetOpts),

    #[options(help = "apply shaping to glyphs from a font")]
    Shape(ShapeOpts),
}

#[derive(Debug, Options)]
pub struct BitmapOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(required, help = "path to directory to write to")]
    pub output: String,

    #[options(free, required, help = "path to font to dump")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct DumpOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(help = "treat the file as a CFF font/table")]
    pub cff: bool,

    #[options(no_short, help = "print the cmap encoding records")]
    pub encodings: bool,

    #[options(help = "dump the content of this table", meta = "TABLE")]
    pub table: Option<String>,

    #[options(
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(help = "dump the specified glyph", meta = "GLYPH_ID")]
    pub glyph: Option<u16>,

    #[options(no_short, help = "print the glyph names in the font")]
    pub glyph_names: bool,

    #[options(help = "print the hmtx table")]
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
        help = "index of the font to dump (for TTC, WOFF2)",
        meta = "INDEX",
        default = "0"
    )]
    pub index: usize,

    #[options(help = "print file name")]
    pub print_file: bool,

    #[options(free, required, help = "path to font to dump")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct ValidateOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(free, required, help = "path to font")]
    pub font: String,
}

#[derive(Debug, Options)]
pub struct SubsetOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(
        required,
        help = "subset the font to include glyphs from TEXT",
        meta = "TEXT"
    )]
    pub text: String,

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
#[options(help = "E.g. shape -f some.ttf -s deva -l HIN 'Some text'")]
pub struct ShapeOpts {
    #[options(help = "print help message")]
    pub help: bool,

    #[options(required, help = "path to font file", meta = "PATH")]
    pub font: String,

    #[options(required, help = "script to shape", meta = "SCRIPT")]
    pub script: String,

    #[options(required, help = "language to shape", meta = "LANG")]
    pub lang: String,

    #[options(free, required, help = "text to shape")]
    pub text: String,
}
