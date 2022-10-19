use allsorts::binary::read::ReadScope;
use allsorts::font::Font;
use allsorts::font_data::FontData;
use allsorts::layout::{LangSys, LayoutTable};
use allsorts::tag::DisplayTag;

use crate::cli::LayoutFeaturesOpts;
use crate::BoxError;

pub fn main(opts: LayoutFeaturesOpts) -> Result<i32, BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let provider = font_file.table_provider(opts.index)?;
    let mut font = match Font::new(provider)? {
        Some(font) => font,
        None => {
            eprintln!("unable to find suitable cmap subtable");
            return Ok(1);
        }
    };

    if let Some(gsub_cache) = font.gsub_cache()? {
        println!("Table: GSUB");
        print_layout_features(&gsub_cache.layout_table)?;
    }

    if let Some(gpos_cache) = font.gpos_cache()? {
        println!("Table: GPOS");
        print_layout_features(&gpos_cache.layout_table)?;
    }

    Ok(0)
}

fn print_layout_features<T>(layout_table: &LayoutTable<T>) -> Result<(), BoxError> {
    if let Some(script_list) = &layout_table.opt_script_list {
        for script_record in script_list.script_records() {
            let script_table = script_record.script_table();

            println!("  Script: {}", DisplayTag(script_record.script_tag));
            if let Some(default_langsys) = script_table.default_langsys_record() {
                println!("    Language: default");
                print_features(&layout_table, &default_langsys)?;
            }
            for langsys in script_table.langsys_records() {
                println!("    Language: {}", DisplayTag(langsys.langsys_tag));
                print_features(&layout_table, langsys.langsys_table())?;
            }
        }
    }

    Ok(())
}

fn print_features<T>(layout_table: &LayoutTable<T>, langsys: &LangSys) -> Result<(), BoxError> {
    for feature_index in langsys.feature_indices_iter() {
        let feature_record = layout_table.feature_by_index(*feature_index)?;
        println!("      Feature: {}", DisplayTag(feature_record.feature_tag));

        let feature_table = feature_record.feature_table();
        let lookup_indices: String = feature_table
            .lookup_indices
            .iter()
            .map(u16::to_string)
            .collect::<Vec<String>>()
            .join(",");
        println!("        Lookups: {}", lookup_indices);
    }

    Ok(())
}
