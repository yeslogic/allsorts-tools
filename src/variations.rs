use allsorts::binary::read::ReadScope;
use allsorts::font_data::FontData;
use allsorts::tables::variable_fonts::fvar::FvarTable;
use allsorts::tables::variable_fonts::stat::StatTable;
use allsorts::tables::{FontTableProvider, NameTable};
use allsorts::tag;
use allsorts::tag::DisplayTag;

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

    let name_table_data = provider.read_table_data(tag::NAME)?;
    let name_table = ReadScope::new(&name_table_data).read::<NameTable>()?;
    let stat_table_data = provider.read_table_data(tag::STAT)?;
    let stat_table = ReadScope::new(&stat_table_data).read::<StatTable>()?;

    println!("Axes: ({})\n", fvar.axes().count());
    for axis in fvar.axes() {
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
        let subfamily = name_table.string_for_id(instance.subfamily_name_id);
        let postscript_name = instance
            .post_script_name_id
            .and_then(|name_id| name_table.string_for_id(name_id));

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
        let coords = instance
            .coordinates
            .iter()
            .map(f32::from)
            .collect::<Vec<_>>();
        println!("    Coordinates: {:?}", coords);
    }

    println!("\nStyle Attributes:");
    for table in stat_table.axis_value_tables() {
        let table = table?;
        dbg!(&table);
        let name_id = table.value_name_id();
        println!(
            "{}",
            name_table
                .string_for_id(name_id)
                .as_deref()
                .unwrap_or("Unknown")
        );
    }

    Ok(())
}
