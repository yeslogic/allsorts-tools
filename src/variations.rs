use allsorts::binary::read::ReadScope;
use allsorts::font_data::FontData;
use allsorts::tables::variable_fonts::fvar::FvarTable;
use allsorts::tables::{FontTableProvider, NameTable};
use allsorts::tag;
use allsorts::tag::DisplayTag;
use encoding_rs::{MACINTOSH, UTF_16BE};

use crate::cli::VariationsOpts;
use crate::BoxError;

pub fn main(opts: VariationsOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let provider = font_file.table_provider(opts.index)?;
    print_variations(&provider)?;

    Ok(0)
}

fn print_variations(provider: &impl FontTableProvider) -> Result<(), BoxError> {
    let Some(table) = provider.table_data(tag::FVAR)? else {
        println!("Font does not appear to be a variable font (no fvar table found)");
        return Ok(());
    };
    let scope = ReadScope::new(&table);
    let fvar = scope.read::<FvarTable>()?;

    let name_table_data = provider.table_data(tag::NAME)?;
    let name_table = name_table_data
        .as_ref()
        .map(|data| ReadScope::new(data).read::<NameTable>())
        .transpose()?;

    println!("Axes: ({})\n", fvar.axes().count());
    for axis in fvar.axes() {
        let axis = axis?;
        println!(
            "- {} = min: {}, max: {}, default: {}",
            DisplayTag(axis.axis_tag),
            f32::from(axis.min_value),
            f32::from(axis.max_value),
            f32::from(axis.default_value)
        )
    }
    println!("\nInstances:");
    for instance in fvar.instances() {
        let instance = instance?;
        let subfamily = english_name_for_name_id(&name_table, instance.subfamily_name_id);
        let postscript_name = instance
            .post_script_name_id
            .and_then(|name_id| english_name_for_name_id(&name_table, name_id));

        println!(
            "\n      Subfamily: {}",
            subfamily.as_deref().unwrap_or("Unknown"),
        );
        if instance.post_script_name_id.is_some() {
            println!(
                "PostScript Name: {}",
                postscript_name.as_deref().unwrap_or("Unknown")
            );
        }
        println!(
            "    Coordinates: {:?}",
            instance
                .coordinates
                .iter()
                .map(f32::from)
                .collect::<Vec<_>>()
        );
    }

    Ok(())
}

fn english_name_for_name_id(name_table: &Option<NameTable>, name_id: u16) -> Option<String> {
    name_table.as_ref().and_then(|name_table| {
        name_table
            .name_records
            .iter()
            .find_map(|record| {
                if record.name_id != name_id {
                    return None;
                }
                // Match English records
                match (record.platform_id, record.encoding_id, record.language_id) {
                    (0, _, _) => Some((record, UTF_16BE)),
                    (1, 0, 0) => Some((record, MACINTOSH)),
                    (
                        3,
                        1,
                        0x0C09 | 0x2809 | 0x1009 | 0x2409 | 0x4009 | 0x1809 | 0x2009 | 0x4409
                        | 0x1409 | 0x3409 | 0x4809 | 0x1C09 | 0x2C09 | 0x0809 | 0x0409 | 0x3009,
                    ) => Some((record, UTF_16BE)),
                    (
                        3,
                        10,
                        0x0C09 | 0x2809 | 0x1009 | 0x2409 | 0x4009 | 0x1809 | 0x2009 | 0x4409
                        | 0x1409 | 0x3409 | 0x4809 | 0x1C09 | 0x2C09 | 0x0809 | 0x0409 | 0x3009,
                    ) => Some((record, UTF_16BE)),
                    _ => None,
                }
            })
            .and_then(|(record, encoding)| {
                let offset = usize::from(record.offset);
                let length = usize::from(record.length);
                let name_data = name_table
                    .string_storage
                    .offset_length(offset, length)
                    .ok()?
                    .data();
                Some(crate::decode(encoding, name_data))
            })
    })
}
