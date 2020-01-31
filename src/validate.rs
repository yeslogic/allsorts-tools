use std::borrow::Borrow;

use allsorts::binary::read::ReadScope;
use allsorts::error::ParseError;
use allsorts::fontfile::FontFile;
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::{FontTableProvider, HeadTable, MaxpTable};
use allsorts::tag;

use crate::cli::ValidateOpts;
use crate::BoxError;
use std::convert::TryFrom;

pub fn main(opts: ValidateOpts) -> Result<(), BoxError> {
    let buffer = std::fs::read(&opts.font)?;
    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontFile>()?;
    let table_provider = font_file.table_provider(0)?; // TODO: Handle all fonts in collection
    let failed = dump_glyphs(&opts.font, &table_provider)?;
    if failed {
        std::process::exit(1);
    }

    Ok(())
}

fn dump_glyphs(path: &str, provider: &impl FontTableProvider) -> Result<bool, ParseError> {
    let table = provider.table_data(tag::HEAD)?.expect("no head table");
    let scope = ReadScope::new(table.borrow());
    let head = scope.read::<HeadTable>()?;

    let table = provider.table_data(tag::MAXP)?.expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let mut failed = false;
    if provider.has_table(tag::CFF) {
        let cff = provider
            .table_data(tag::CFF)?
            .expect("unable to read CFF table");
        match check_cff_table(ReadScope::new(&cff)) {
            Ok(()) => (),
            Err(err) => {
                failed = true;
                println!("{}: CFF Error - {}", path, err)
            }
        }
    } else {
        let table = provider.table_data(tag::LOCA)?.expect("no loca table");
        let scope = ReadScope::new(table.borrow());
        let loca = scope
            .read_dep::<LocaTable>((usize::from(maxp.num_glyphs), head.index_to_loc_format))?;

        let table = provider.table_data(tag::GLYF)?.expect("no glyf table");
        let scope = ReadScope::new(table.borrow());
        let mut glyf = scope.read_dep::<GlyfTable>(&loca)?;

        for (index, glyph) in glyf.records.iter_mut().enumerate() {
            match glyph.parse() {
                Ok(()) => (),
                Err(err) => {
                    failed = true;
                    println!("{} [{}]: {}", path, index, err)
                }
            }
        }
    }

    Ok(failed)
}

fn check_cff_table<'a>(scope: ReadScope<'a>) -> Result<(), ParseError> {
    use allsorts::cff::{self, CFFVariant, FontDict, Operator, CFF};

    let cff = scope.read::<CFF>()?;
    if cff.name_index.count != 1 {
        return Err(ParseError::BadIndex);
    }
    let font = cff.fonts.get(0).ok_or(ParseError::MissingValue)?;
    let char_strings_offset = font
        .top_dict
        .get_i32(Operator::CharStrings)
        .ok_or(ParseError::MissingValue)??;
    let _char_strings_index = scope
        .offset(usize::try_from(char_strings_offset)?)
        .read::<cff::Index<'_>>()?;
    match &font.data {
        CFFVariant::Type1(ref _type1) => {}
        CFFVariant::CID(cid) => {
            for (_i, object) in cid.font_dict_index.iter().enumerate() {
                let font_dict = ReadScope::new(object).read::<FontDict>()?;
                let (_private_dict, _private_dict_offset) = font_dict.read_private_dict(&scope)?;
            }
        }
    }

    Ok(())
}
