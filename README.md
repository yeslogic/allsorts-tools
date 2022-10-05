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

* `bitmaps` — dump bitmaps from bitmap fonts
* `cmap` — print character to glyph mappings
* `dump` — dump font information
* `has-table` — check if a font has a particular table
* `shape` — apply shaping to glyphs from a font
* `subset` — subset a font
* `svg` — generate SVGs from glyphs
* `validate` — parse the supplied font, reporting any failures

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

`--name` includes the meta data contained in the `name` table in the output.

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

### `subset`

The `subset` tool takes a source font and some text and writes a new version of
the source font only containing the glyphs required for the supplied text.

#### Example

    $ allsorts subset -t 'This a subsetting test' NotoSansJP-Regular.otf noto-subset.otf
    Number of glyphs in new font: 13

### `shape`

The `shape` tool shapes the supplied text according to the supplied font, language, and
script. It prints out the glyphs before and after shaping.

#### Example

    $ shape -f fonts/devanagari/AnnapurnaSIL-Regular.ttf -s deva -l HIN 'शब्दों और वाक्यों की तरह'
    # output omitted

### `svg`

The `svg` tool takes a source font and some text. It shapes the text, then generates an SVG
of the glyphs.

#### Example

    $ svg --font /usr/share/fonts/inter/Inter-Regular.ttf --render 'Café' --flip

```svg
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 -241 2212 1210">
    <symbol id="allsorts.uni0043" overflow="visible">
        <path d=" M673.2954,-500 L585.22723,-500 Q574.57385,-550.42615 544.38916,-584.87213 Q514.2045,-619.3182 471.59088,-637.07385 Q428.97726,-654.8295 380.6818,-654.8295 Q314.63068,-654.8295 261.18607,-621.44885 Q207.74147,-588.0682 176.31392,-523.0824 Q144.88635,-458.0966 144.88635,-363.63635 Q144.88635,-269.17612 176.31392,-204.19034 Q207.74147,-139.20454 261.18607,-105.82386 Q314.63068,-72.44318 380.6818,-72.44318 Q428.97726,-72.44318 471.59088,-90.19886 Q514.2045,-107.954544 544.38916,-142.40056 Q574.57385,-176.84659 585.22723,-227.27272 L673.2954,-227.27272 Q659.8011,-152.69885 618.2528,-99.609375 Q576.7045,-46.519886 515.2699,-18.288351 Q453.8352,9.943182 380.6818,9.943182 Q287.64203,9.943182 215.19885,-35.511364 Q142.75568,-80.965904 101.20738,-164.77272 Q59.65909,-248.57954 59.65909,-363.63635 Q59.65909,-478.69318 101.20738,-562.5 Q142.75568,-646.3068 215.19885,-691.76135 Q287.64203,-737.2159 380.6818,-737.2159 Q453.8352,-737.2159 515.2699,-708.9844 Q576.7045,-680.7528 618.2528,-627.6633 Q659.8011,-574.57385 673.2954,-500 Z"/>
    </symbol>
    <symbol id="allsorts.uni0061" overflow="visible">
        <path d=" M237.2159,12.78409 Q185.36931,12.78409 143.1108,-6.9247155 Q100.85227,-26.633522 75.994316,-64.09801 Q51.13636,-101.5625 51.13636,-154.82954 Q51.13636,-201.70454 69.60227,-231.00142 Q88.06818,-260.29828 118.963066,-276.98862 Q149.85796,-293.67896 187.32243,-302.02414 Q224.78693,-310.36932 262.7841,-315.3409 Q312.5,-321.73294 343.57242,-325.10654 Q374.64487,-328.4801 389.02698,-336.6477 Q403.4091,-344.81534 403.4091,-365.0568 L403.4091,-367.8977 Q403.4091,-420.45453 374.82242,-449.57385 Q346.23578,-478.69318 288.35226,-478.69318 Q228.33806,-478.69318 194.24715,-452.41476 Q160.15625,-426.13635 146.30681,-396.3068 L66.76136,-424.71588 Q88.06818,-474.4318 123.757095,-502.30823 Q159.44601,-530.18463 201.8821,-541.3707 Q244.31818,-552.5568 285.51135,-552.5568 Q311.78976,-552.5568 346.05823,-546.3423 Q380.3267,-540.1278 412.46448,-520.95166 Q444.60226,-501.77554 465.9091,-463.06818 Q487.21588,-424.36078 487.21588,-359.375 L487.21588,0 L403.4091,0 L403.4091,-73.86363 L399.1477,-73.86363 Q390.625,-56.107952 370.73862,-35.866478 Q350.85226,-15.625 317.8267,-1.4204545 Q284.80112,12.78409 237.2159,12.78409 Z M250,-62.5 Q299.7159,-62.5 333.98438,-82.03125 Q368.25284,-101.5625 385.83096,-132.45738 Q403.4091,-163.35226 403.4091,-197.44318 L403.4091,-274.1477 Q398.08237,-267.75568 380.14914,-262.60654 Q362.2159,-257.45737 338.95596,-253.72868 Q315.696,-250 293.85654,-247.33664 Q272.01703,-244.6733 258.5227,-242.89772 Q225.85226,-238.63635 197.62073,-229.22585 Q169.3892,-219.81534 152.16618,-201.17188 Q134.94318,-182.5284 134.94318,-150.56818 Q134.94318,-106.8892 167.43608,-84.6946 Q199.92897,-62.5 250,-62.5 Z"/>
    </symbol>
    <symbol id="allsorts.uni0066" overflow="visible">
        <path d=" M319.60226,-545.4545 L319.60226,-474.4318 L197.44318,-474.4318 L197.44318,0 L113.63636,0 L113.63636,-474.4318 L25.56818,-474.4318 L25.56818,-545.4545 L113.63636,-545.4545 L113.63636,-620.73865 Q113.63636,-667.6136 135.65341,-698.8636 Q157.67046,-730.1136 192.8267,-745.7386 Q227.98294,-761.3636 267.04544,-761.3636 Q297.94034,-761.3636 317.4716,-756.392 Q337.00284,-751.4204 346.5909,-747.15906 L322.44318,-674.7159 Q316.05112,-676.84656 304.86505,-680.0426 Q293.67896,-683.2386 275.56818,-683.2386 Q234.01988,-683.2386 215.73152,-662.2869 Q197.44318,-641.3352 197.44318,-600.85223 L197.44318,-545.4545 Z"/>
    </symbol>
    <symbol id="allsorts.uni00E9" overflow="visible">
        <path d=" M305.3977,11.363636 Q226.5625,11.363636 169.56676,-23.615057 Q112.57102,-58.59375 81.85369,-121.62642 Q51.13636,-184.65909 51.13636,-268.4659 Q51.13636,-352.2727 81.85369,-416.37073 Q112.57102,-480.46875 167.79118,-516.51276 Q223.01135,-552.5568 296.875,-552.5568 Q339.48862,-552.5568 381.03693,-538.35223 Q422.5852,-524.1477 456.67612,-492.36505 Q490.76703,-460.58237 511.0085,-408.38068 Q531.25,-356.17896 531.25,-279.82953 L531.25,-244.31818 L135.2983,-244.31818 Q138.1392,-156.96022 184.83664,-110.44034 Q231.53409,-63.920452 305.3977,-63.920452 Q354.7585,-63.920452 390.26987,-85.22727 Q425.78125,-106.53409 441.76135,-149.14772 L522.72723,-126.42045 Q503.55112,-64.63068 446.0227,-26.633522 Q388.4943,11.363636 305.3977,11.363636 Z M135.2983,-316.76135 L446.0227,-316.76135 Q446.0227,-386.0085 405.53976,-431.64063 Q365.0568,-477.2727 296.875,-477.2727 Q248.93465,-477.2727 213.7784,-454.90054 Q178.62215,-432.52838 158.38068,-395.77414 Q138.1392,-359.01987 135.2983,-316.76135 Z M254.26135,-619.3182 L340.9091,-784.0909 L438.92044,-784.0909 L328.125,-619.3182 Z"/>
    </symbol>
    <use xlink:href="#allsorts.uni0043" x="0" y="0"/>
    <use xlink:href="#allsorts.uni0061" x="727" y="0"/>
    <use xlink:href="#allsorts.uni0066" x="1291" y="0"/>
    <use xlink:href="#allsorts.uni00E9" x="1629" y="0"/>
</svg>
```

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
