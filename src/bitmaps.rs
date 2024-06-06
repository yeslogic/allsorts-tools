use std::borrow::Cow;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use allsorts::binary::read::ReadScope;
use allsorts::bitmap::{BitDepth, Bitmap, BitmapGlyph, EncapsulatedFormat};
use allsorts::font_data::FontData;

use allsorts::Font;

use crate::cli::BitmapOpts;
use crate::BoxError;
use allsorts::font::MatchingPresentation;
use allsorts::tag::DisplayTag;

pub fn main(opts: BitmapOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let table_provider = font_file.table_provider(opts.index)?;
    let mut font = Font::new(table_provider)?;

    let output_path = Path::new(&opts.output);
    if !output_path.exists() {
        fs::create_dir(output_path)?;
    }

    for ch in opts.text.chars() {
        let (glyph_id, _) = font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None);
        if glyph_id == 0 {
            eprintln!("No glyph for '{}'", ch);
            continue;
        }

        match font.lookup_glyph_image(glyph_id, opts.size, BitDepth::ThirtyTwo)? {
            Some(bitmap) => {
                let strike_path = output_path.join(&format!(
                    "{}x{}",
                    bitmap.ppem_x.unwrap_or(0),
                    bitmap.ppem_y.unwrap_or(0)
                ));
                if !strike_path.exists() {
                    fs::create_dir(&strike_path)?;
                }

                dump_bitmap(&strike_path, glyph_id, &bitmap)?;
            }
            None => {
                eprintln!("No bitmap for {} ('{}')", glyph_id, ch);
            }
        }
    }

    Ok(0)
}

fn dump_bitmap(path: &Path, glyph_id: u16, bitmap: &BitmapGlyph) -> Result<(), BoxError> {
    match &bitmap.bitmap {
        Bitmap::Embedded(embedded) => {
            let glyph_path = path.join(&format!("{}.png", glyph_id));
            let file = File::create(&glyph_path)?;
            let w = BufWriter::new(file);
            let mut encoder =
                png::Encoder::new(w, u32::from(embedded.width), u32::from(embedded.height));
            encoder.set_color(if embedded.format != BitDepth::ThirtyTwo {
                png::ColorType::Grayscale
            } else {
                png::ColorType::RGBA
            });
            let bit_depth = match embedded.format {
                BitDepth::One => png::BitDepth::One,
                BitDepth::Two => png::BitDepth::Two,
                BitDepth::Four => png::BitDepth::Four,
                BitDepth::Eight | BitDepth::ThirtyTwo => png::BitDepth::Eight,
            };
            encoder.set_depth(bit_depth);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&embedded.data)?;
        }
        Bitmap::Encapsulated(encapsulated) => {
            let extension = match encapsulated.format {
                EncapsulatedFormat::Jpeg => Cow::from("jpg"),
                EncapsulatedFormat::Png => Cow::from("png"),
                EncapsulatedFormat::Tiff => Cow::from("tiff"),
                EncapsulatedFormat::Svg => Cow::from("svg"),
                EncapsulatedFormat::Other(format) => Cow::from(DisplayTag(format).to_string()),
            };

            let glyph_path = path.join(&format!("{}.{}", glyph_id, extension.trim_end()));
            fs::write(glyph_path, &encapsulated.data)?;
        }
    }

    Ok(())
}
