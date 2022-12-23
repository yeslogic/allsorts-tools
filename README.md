<h1 align="center">
  <img src="https://github.com/yeslogic/allsorts/raw/master/allsorts.svg?sanitize=1" alt=""><br>
  Allsorts Tools
</h1>

<div align="center">
  <strong>Example tools for the <a href="https://github.com/yeslogic/allsorts">Allsorts</a> font parser, shaping engine, and subsetter</strong>
</div>

<br>

<div align="center">
  <a href="https://travis-ci.com/yeslogic/allsorts-tools">
    <img src="https://travis-ci.com/yeslogic/allsorts-tools.svg?token=4GA6ydxNNeb6XeELrMmg&amp;branch=master" alt="Build Status"></a>
  <a href="https://github.com/yeslogic/allsorts-tools/blob/master/LICENSE">
    <img src="https://img.shields.io/github/license/yeslogic/allsorts-tools.svg" alt="License">
  </a>
</div>

<br>

[Allsorts](https://github.com/yeslogic/allsorts)
is a font parser, shaping engine, and subsetter for OpenType, WOFF, and WOFF2
written entirely in Rust. This repository contains tools that were developed to
debug and test Allsorts and provide examples of its use.

**Note:** These tools are for demonstration and reference purposes. You should
not rely on them for production workflows.

## Tools

Available tools:

* [`bitmaps`](#bitmaps) — dump bitmaps from bitmap fonts
* [`cmap`](#cmap) — print character to glyph mappings
* [`dump`](#dump) — dump font information
* [`has-table`](#has-table) — check if a font has a particular table
* [`layout-features`](#layout-features) - print a list of a font's GSUB and GPOS features
* [`shape`](#shape) — apply shaping to glyphs from a font
* [`subset`](#subset) — subset a font
* [`validate`](#validate) — parse the supplied font, reporting any failures
* [`view`](#view) - generate SVGs from glyphs

### `bitmaps`

The `bitmaps` tool extracts bitmaps from fonts containing glyph bitmaps in
either the `EBLC`/`EBDT` or `CBLC`/`CBDT` tables.

`-o` is the path to the directory to write the bitmaps to. It will be created
if it does not exist.

The images are written out as PNGs in a sub-directory for each strike (size).
The format is `{ppem_x}x{ppem_y}@{bit_depth}`, the files are named
`{glyph_id}.png`:

    terminus
    ├── 12x12@1
    │  ├── 0.png
    │  ├── 1.png
    │  ├── 2.png
    │  ├── 3.png
    │  ├── 4.png
    │  ├── 5.png
    │  ├── 6.png
    │  ├── 7.png
    ⋮  ⋮
    ├── 14x14@1
    │  ├── 0.png
    ⋮  ⋮
    └── 32x32@1
    ⋮  ⋮

#### Example

    allsorts bitmaps -o noto-color-emoji NotoColorEmoji.ttf

### `cmap`

The `cmap` tool chooses a preferred `cmap` sub-table and dumps the character to
glyph index entries. If the encoding of the table is Unicode then the characters
are printed along with the code point, otherwise just the numeric value of the
character is printed.

`-f`, `--font` specifies the path to the font file.

`-i`, `--index` is index of the font to dump (for TTC, WOFF2) (default: 0).

#### Example

    $ allsorts cmap --font profontn.otb
    cmap sub-table encoding: Unicode
    '' U+0000 -> 0
    '' U+0001 -> 1
    '' U+0002 -> 2
    ⋮
    '?' U+003F -> 63
    '@' U+0040 -> 64
    'A' U+0041 -> 65
    'B' U+0042 -> 66
    ⋮
    '»' U+00BB -> 187
    '¼' U+00BC -> 188
    '½' U+00BD -> 189
    '¾' U+00BE -> 190
    '¿' U+00BF -> 191
    'À' U+00C0 -> 192
    'Á' U+00C1 -> 193
    'Â' U+00C2 -> 194
    'Ã' U+00C3 -> 195
    'Ä' U+00C4 -> 196
    ⋮


### `dump`

The `dump` tool prints or extracts information from a font file.

`allsorts dump path/to/font` prints out information about the font.

`--name` includes the metadata contained in the `name` table in the output.

`-c` can be used to print information about a CFF font or table not
wrapped in a TrueType or OpenType container.

`-t` extracts the named table from the supplied font. The output should be
redirected to a file. E.g. `allsorts dump -t glyf > glyf.bin`

`-g` prints information about a specific glyph in a font.

`-l` prints out all offsets in the `loca` table in the font.

#### Example

    $ allsorts dump noto-subset.otd | head
    TTF
     - version: 0x4f54544f
     - num_tables: 9

    CFF  (checksum: 0x625ba831, offset: 156, length: 166505)
    OS/2 (checksum: 0x9f6306c8, offset: 166664, length: 96)
    cmap (checksum: 0x131b2742, offset: 166760, length: 274)
    head (checksum: 0x09e560e8, offset: 167036, length: 54)
    hhea (checksum: 0x0c1109cf, offset: 167092, length: 36)
    hmtx (checksum: 0x1b9b0310, offset: 167128, length: 52)
    maxp (checksum: 0x000d5000, offset: 167180, length: 6)
    name (checksum: 0x1f3037ad, offset: 167188, length: 418)
    post (checksum: 0xff860032, offset: 167608, length: 32)

    - CFF:
     - version: 1.0
     - name: NotoSansJP-Regular
     - num glyphs: 13
     - charset: Custom
     - variant: CID

### `has-table`

The `has-table` tool checks if the supplied font file contains the table passed
via the `-t` argument.  If the font contains the table it exits with status
success (0), if the font does not contain the table it exits with status 1.
The `-p` option makes the tool print the path to the font if it contains the
table.

This tool is handy combined with `find`, to locate fonts that have the desired table.

#### Example

In this example, we search the current directory for files ending in `ttf`,
`otf`, or `otc` and check to see if they contain an `EBLC` table. If the table
is found the path to the font is printed.

    find . -regextype posix-extended -type f -iregex '.*\.(ttf|otf|otc)$' -exec allsorts has-table -t EBLC -p {} \;

### `layout-features`

Prints an indented list of a font's GSUB and GPOS features.

#### Example

    $ layout-features fonts/devanagari/AnnapurnaSIL-Regular.ttf
    Table: GSUB
      Script: DFLT
        Language: default
          Feature: aalt
            Lookups: 56
          Feature: abvs
            Lookups: 27,28,29,30
          Feature: akhn
            Lookups: 4
          Feature: blwf
            Lookups: 9
    # additional output omitted

### `shape`

The `shape` tool shapes the supplied text according to the supplied font, language, and
script. It prints out the glyphs before and after shaping.

#### Example

    $ shape -f fonts/devanagari/AnnapurnaSIL-Regular.ttf -s deva -l HIN 'शब्दों और वाक्यों की तरह'
    # output omitted

### `subset`

The `subset` tool takes a source font and some text and writes a new version of
the source font only containing the glyphs required for the supplied text.

#### Example

    $ allsorts subset -t 'This a subsetting test' NotoSansJP-Regular.otf noto-subset.otf
    Number of glyphs in new font: 13

### `validate`

The `validate` tool attempts to parse all the glyphs (or various DICTs in the
case of CFF) in the supplied font. It reports any errors encountered but is
otherwise silent. This command was useful for bulk testing Allsorts against a
large repertoire of real world fonts.

#### Example

    $ allsorts validate ../allsorts/tests/fonts/bengali/Lohit-Bengali.ttf

#### Bulk Validation Example

    $ fd '\.(ttf|otf|ttc)$' /usr/share/fonts | sort | parallel --bar allsorts validate {}

### `view`

The `view` tool shapes the supplied text or list of codepoints according to the
supplied font, language, and script. Then, it generates an SVG of the glyphs.

`--mark-origin` will place a cross-hair at the origin of each glyph in the generated
SVG.

#### Example Using Text

    $ view -f fonts/devanagari/NotoSerifDevanagari-Regular.ttf -s deva -t 'खि'
    # output omitted

#### Example Using Codepoints

    $ view -f fonts/devanagari/NotoSerifDevanagari-Regular.ttf -s deva -c '916,93f'
    # output omitted

#### Example Using Glyph Indices (and Features)

In this example, the OpenType `pres` feature is enabled, which allows glyph 30
to be replaced by its special presentation form (glyph 547).

    $ view -f fonts/devanagari/NotoSerifDevanagari-Regular.ttf -s deva --features pres -i '30,54'
    # output omitted

## Building and Installing

### From Source

**Minimum Supported Rust Version:** 1.51.0

To build the tools ensure you have [Rust 1.51.0 or newer installed](https://www.rust-lang.org/tools/install).

* Build: `cargo build --release`
* Install: `cargo install --path .`

### Arch Linux

There is an [AUR package for `allsorts-tools`](https://aur.archlinux.org/packages/allsorts-tools/):

    git clone https://aur.archlinux.org/allsorts-tools.git
    cd allsorts-tools
    makepkg -si

## Contributing

Contributions are welcome, please refer to the
[Allsorts contributing guide](https://github.com/yeslogic/allsorts/blob/master/CONTRIBUTING.md)
for more details.

## Code of Conduct

We aim to uphold the Rust community standards:

> We are committed to providing a friendly, safe and welcoming environment for
> all, regardless of gender, sexual orientation, disability, ethnicity,
> religion, or similar personal characteristic.

We follow the [Rust code of conduct](https://www.rust-lang.org/policies/code-of-conduct).

## License

Allsorts and these tools are distributed under the terms of the Apache License
(Version 2.0).

See [LICENSE](LICENSE) for details.
