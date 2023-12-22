use std::fs::File;
use std::io::Write;
use std::path::Path;

use allsorts::binary::read::ReadScope;
use allsorts::font_data::{DynamicFontTableProvider, FontData};
use allsorts::tables::variable_fonts::fvar::{FvarTable, InstanceRecord, VariationAxisRecord};
use allsorts::tables::variable_fonts::stat::StatTable;
use allsorts::tables::{FontTableProvider, NameTable};
use allsorts::tag;
use allsorts::tag::DisplayTag;
use allsorts::variations::VariationError;

use crate::cli::VariationsOpts;
use crate::BoxError;

pub fn main(opts: VariationsOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let provider = font_file.table_provider(opts.index)?;

    if opts.test {
        generate_test(&provider, &opts.font)?;
    } else {
        print_variations(&provider)?;
    }

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
    let stat_table_data = provider.table_data(tag::STAT)?;
    let stat_table = stat_table_data
        .as_ref()
        .map(|stat_data| ReadScope::new(stat_data).read::<StatTable<'_>>())
        .transpose()?;

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

    if let Some(stat) = stat_table {
        println!("\nStyle Attributes:");
        for table in stat.axis_value_tables() {
            let table = table?;
            let name_id = table.value_name_id();
            println!(
                "{}",
                name_table
                    .string_for_id(name_id)
                    .as_deref()
                    .unwrap_or("Unknown")
            );
        }
    }

    Ok(())
}

fn generate_test(provider: &DynamicFontTableProvider, font: &str) -> Result<(), BoxError> {
    if !(provider.has_table(tag::FVAR) && provider.has_table(tag::GVAR)) {
        println!("Font does have both fvar and gvar");
        return Ok(());
    }

    let fvar_data = provider.read_table_data(tag::FVAR)?;
    let scope = ReadScope::new(&fvar_data);
    let fvar = scope.read::<FvarTable>()?;

    let name_table_data = provider.read_table_data(tag::NAME)?;
    let name = ReadScope::new(&name_table_data).read::<NameTable>()?;

    let output_path = font.to_string() + ".html";
    let mut out = File::create(&output_path)?;
    let axes = fvar.axes().collect::<Vec<_>>();
    let typographic_family = name
        .string_for_id(NameTable::TYPOGRAPHIC_FAMILY_NAME)
        .or_else(|| name.string_for_id(NameTable::FONT_FAMILY_NAME))
        .ok_or(VariationError::NameError)?;

    writeln!(
        out,
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<style>"
    )?;
    let mut spans = Vec::new();
    for instance in fvar.instances() {
        let instance = instance?;
        let subfamily = name
            .string_for_id(instance.subfamily_name_id)
            .ok_or_else(|| "instance has no subfamily name")?;
        let font_family = format!("{typographic_family} {subfamily}");
        let src = Path::new(font)
            .file_name()
            .and_then(|src| src.to_str())
            .ok_or_else(|| "unable to get filename of font")?;
        let font_face = font_face(&axes, &font_family, src, &instance);
        writeln!(out, "{font_face}")?;

        let span = format!(
            r#"<p style="font-family: '{font_family}', sans-serif">mix Zapf with Veljović and get quirky Béziers</p>"#
        );
        spans.push(span);
    }
    writeln!(out, "body {{ font-size: 18pt }}\n</style>\n<title>{typographic_family} Test</title>\n</head>\n<body>")?;
    let text = spans.join("\n");
    writeln!(out, "{text}")?;
    writeln!(out, "</body>\n</html>")?;

    println!("Wrote: {output_path}");
    Ok(())
}

fn font_face(
    axes: &[VariationAxisRecord],
    font_family: &str,
    src: &str,
    instance: &InstanceRecord,
) -> String {
    let font_variation_settings = instance
        .coordinates
        .iter()
        .zip(axes)
        .map(|(coord, axis)| format!("'{}' {}", DisplayTag(axis.axis_tag), f32::from(coord)))
        .collect::<Vec<_>>();
    let font_variation_settings = font_variation_settings.join(", ");

    format!(
        r#"@font-face {{
    font-family: "{font_family}";
    src: url("{src}");
    font-weight: normal;
    font-style: normal;
    font-stretch: normal;
    font-variation-settings: {font_variation_settings};
}}"#,
    )
}
