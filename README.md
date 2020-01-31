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

* `dump` — dump font information
* `shape` — apply shaping to glyphs from a font
* `subset` — subset a font
* `validate` — parse the supplied font, reporting any failures

### `dump`

The `dump` tool prints or extract information from a font file.

`allsorts dump path/to/font` prints out information about the font and meta
data contained in the `name` table.

`-c` option can be used to print information about a CFF font or table not
wrapped in a TrueType or OpenType container.

`-t` option extracts the named table from the supplied font. The output should be
redirected to a file. E.g. `allsorts dump -t glyf > glyf.bin`

`-g` prints information about a specific glyph in a font.

`-l` option prints out all offsets in the `loca` table in the font.

### Example

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

### `subset`

The `subset` tool takes a source font and some text and writes a new version of the source font only
containing the glyphs required for the supplied text.

#### Example

    $ allsorts subset -t 'This a subsetting test' NotoSansJP-Regular.otf noto-subset.otf
    Number of glyphs in new font: 13

### `shape`

The `shape` tool shapes the supplied text according to the supplied font, language, and
script. It prints out the glyphs before and after shaping.

#### Example

    $ shape -f fonts/devanagari/AnnapurnaSIL-Regular.ttf -s deva -l HIN 'शब्दों और वाक्यों की तरह'
    # output omitted

### `validate`

The `validate` tool attempts to parse all the glyphs (or various DICTs in the
case of CFF) in the supplied font. It reports any errors encountered but is
otherwise silent. This command was useful for bulk testing Allsorts against a
large repertoire of real world fonts.

#### Example

    $ allsorts validate ../allsorts/tests/fonts/bengali/Lohit-Bengali.ttf

#### Bulk Validation Example

    $ fd '\.(ttf|otf|ttc)$' /usr/share/fonts | sort | parallel --bar allsorts validate {}

## Building and Installing

### From Source

**Minimum Supported Rust Version:** 1.38.0

To build the tools ensure you have [Rust 1.38.0 or newer installed](https://www.rust-lang.org/tools/install).

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
