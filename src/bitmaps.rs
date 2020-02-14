use std::borrow::Borrow;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use bitreader::{BitReader, BitReaderError};

use allsorts::binary::read::ReadScope;
use allsorts::bitmap::{self, BitDepth, BitmapSize, CBDTTable, CBLCTable, GlyphBitmapData};
use allsorts::fontfile::FontFile;
use allsorts::tables::FontTableProvider;
use allsorts::tag::{self};

use crate::cli::BitmapOpts;
use crate::BoxError;

pub fn main(opts: BitmapOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontFile>()?;
    let table_provider = font_file.table_provider(opts.index)?;

    let table = table_provider
        .table_data(tag::CBLC)?
        .or(table_provider.table_data(tag::EBLC)?)
        .ok_or("font does not have CBLC or EBLC tables")?;
    let scope = ReadScope::new(table.borrow());
    let cblc = scope.read::<CBLCTable<'_>>().unwrap();
    let table = table_provider
        .table_data(tag::CBDT)?
        .or(table_provider.table_data(tag::EBDT)?)
        .ok_or("font does not have CBDT or EBDT tables")?;
    let scope = ReadScope::new(table.borrow());
    let cbdt = scope.read::<CBDTTable<'_>>().unwrap();

    let output_path = Path::new(&opts.output);
    if !output_path.exists() {
        fs::create_dir(output_path)?;
    }

    for strike in cblc.bitmap_sizes.iter_res() {
        let strike = strike?;
        let strike_path = output_path.join(&format!(
            "{}x{}@{}",
            strike.ppem_x, strike.ppem_y, strike.bit_depth as u8
        ));
        if !strike_path.exists() {
            fs::create_dir(&strike_path)?;
        }
        dump_bitmaps(strike, &strike_path, &cblc, &cbdt)?;
    }

    Ok(0)
}

fn dump_bitmaps<'a>(
    strike: BitmapSize,
    path: &Path,
    cblc: &'a CBLCTable<'a>,
    cbdt: &'a CBDTTable<'a>,
) -> Result<(), BoxError> {
    for glyph_id in strike.start_glyph_index..=strike.end_glyph_index {
        if let Some(bitmap) =
            bitmap::lookup(glyph_id, strike.ppem_x, BitDepth::ThirtyTwo, cblc, cbdt)?
        {
            let glyph_path = path.join(&format!("{}.png", glyph_id));
            match bitmap {
                (_, GlyphBitmapData::Format17 { data, .. })
                | (_, GlyphBitmapData::Format18 { data, .. })
                | (_, GlyphBitmapData::Format19 { data, .. }) => {
                    // Already PNG, just write it out
                    fs::write(&glyph_path, data)?;
                }
                (_, GlyphBitmapData::Format8 { components, .. })
                | (_, GlyphBitmapData::Format9 { components, .. }) => {
                    let ids = components
                        .iter()
                        .map(|component| component.glyph_id.to_string())
                        .collect::<Vec<_>>();
                    println!("glyph {} is comprised of {}", glyph_id, ids.join(", "))
                }
                (_, bitmap) => {
                    // Convert to PNG and write out
                    if bitmap.width() == 0 || bitmap.height() == 0 {
                        println!(
                            "glyph {} has 0 dimension: {}x{}",
                            glyph_id,
                            bitmap.width(),
                            bitmap.height()
                        );
                        continue;
                    }
                    let file = File::create(&glyph_path)?;
                    let w = BufWriter::new(file);
                    let mut encoder =
                        png::Encoder::new(w, u32::from(bitmap.width()), u32::from(bitmap.height()));
                    encoder.set_color(if strike.bit_depth != BitDepth::ThirtyTwo {
                        png::ColorType::Grayscale
                    } else {
                        png::ColorType::RGBA
                    });
                    let bit_depth = match strike.bit_depth {
                        BitDepth::One => png::BitDepth::One,
                        BitDepth::Two => png::BitDepth::Two,
                        BitDepth::Four => png::BitDepth::Four,
                        BitDepth::Eight | BitDepth::ThirtyTwo => png::BitDepth::Eight,
                    };
                    encoder.set_depth(bit_depth);
                    let mut writer = encoder.write_header()?;
                    write_image_data(&strike, &bitmap, &mut writer)?;
                }
            }
        }
    }

    Ok(())
}

fn write_image_data(
    strike: &BitmapSize,
    bitmap: &GlyphBitmapData,
    writer: &mut png::Writer<BufWriter<File>>,
) -> Result<(), BoxError> {
    match bitmap {
        // Format 1: small metrics, byte-aligned data.
        GlyphBitmapData::Format1 { data, .. } => {
            writer.write_image_data(data)?;
        }
        // Format 2: small metrics, bit-aligned data.
        GlyphBitmapData::Format2 {
            small_metrics,
            data,
        } => {
            let image_data = unpack_bit_aligned_data(
                strike.bit_depth,
                small_metrics.width,
                small_metrics.height,
                data,
            )?;
            writer.write_image_data(&image_data)?;
        }
        // Format 5: metrics in EBLC, bit-aligned image data only.
        GlyphBitmapData::Format5 { big_metrics, data } => {
            let image_data = unpack_bit_aligned_data(
                strike.bit_depth,
                big_metrics.width,
                big_metrics.height,
                data,
            )?;
            writer.write_image_data(&image_data)?;
        }
        // Format 6: big metrics, byte-aligned data.
        GlyphBitmapData::Format6 { .. } => unimplemented!("format 6 need a test font"),
        // Format7: big metrics, bit-aligned data.
        GlyphBitmapData::Format7 { data, big_metrics } => {
            let image_data = unpack_bit_aligned_data(
                strike.bit_depth,
                big_metrics.width,
                big_metrics.height,
                data,
            )?;
            writer.write_image_data(&image_data)?;
        }
        _ => unreachable!("handled in dump_bitmaps"),
    }

    Ok(())
}

fn unpack_bit_aligned_data(
    bit_depth: BitDepth,
    width: u8,
    height: u8,
    data: &[u8],
) -> Result<Vec<u8>, BitReaderError> {
    let bits_per_row = bit_depth as usize * usize::from(width);
    let whole_bytes_per_row = bits_per_row >> 3;
    let remaining_bits = (bits_per_row & 7) as u8;
    let bytes_per_row = whole_bytes_per_row + if remaining_bits != 0 { 1 } else { 0 };

    let mut offset = 0;
    let mut image_data = vec![0u8; usize::from(height) * bytes_per_row];
    let mut reader = BitReader::new(data);
    for _ in 0..height {
        // Read whole bytes, then the remainder
        for byte in image_data[offset..(offset + whole_bytes_per_row)].iter_mut() {
            *byte = reader.read_u8(8)?;
        }
        offset += whole_bytes_per_row;
        if remaining_bits != 0 {
            *image_data.get_mut(offset).unwrap() =
                reader.read_u8(remaining_bits)? << (8 - remaining_bits);
            offset += 1;
        }
    }

    Ok(image_data)
}
