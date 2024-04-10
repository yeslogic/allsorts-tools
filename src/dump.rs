use std::borrow::Borrow;
use std::convert::{self, TryFrom};
use std::io::{self, Write};
use std::str;

use atty::Stream;
use encoding_rs::{MACINTOSH, UTF_16BE};

use allsorts::binary::read::ReadScope;
use allsorts::cff::{self, CFFVariant, Charset, FontDict, Operand, Operator, CFF};
use allsorts::error::ParseError;
use allsorts::font::read_cmap_subtable;
use allsorts::font_data::FontData;
use allsorts::glyph_info::GlyphNames;
use allsorts::tables::cmap::{Cmap, CmapSubtable};
use allsorts::tables::glyf::GlyfTable;
use allsorts::tables::loca::LocaTable;
use allsorts::tables::{
    FontTableProvider, HeadTable, HheaTable, HmtxTable, MaxpTable, NameTable, OffsetTable,
    OpenTypeData, TTCHeader,
};
use allsorts::tag::{self, DisplayTag};
use allsorts::woff::WoffFont;
use allsorts::woff2::{Woff2Font, Woff2GlyfTable, Woff2LocaTable};

use crate::cli::DumpOpts;
use crate::{decode, BoxError, ErrorMessage};

type Tag = u32;

#[derive(Copy, Clone)]
struct Flags {
    encodings: bool,
    glyphs_names: bool,
    name: bool,
}

pub fn main(opts: DumpOpts) -> Result<i32, BoxError> {
    let flags = Flags::from(&opts);
    let table = opts
        .table
        .map(|table| tag::from_string(&table))
        .transpose()?;
    if table.is_some() && atty::is(Stream::Stdout) {
        return Err(ErrorMessage("Not printing binary data to tty.").into());
    }

    let buffer = std::fs::read(&opts.font)?;

    if opts.cff {
        dump_cff_table(ReadScope::new(&buffer))?;
        return Ok(0);
    }

    let scope = ReadScope::new(&buffer);
    let font_file = scope.read::<FontData>()?;
    let table_provider = font_file.table_provider(opts.index)?;

    if opts.loca {
        dump_loca_table(&table_provider)?;
    } else if opts.head {
        dump_head_table(&table_provider)?;
    } else if opts.hmtx {
        dump_hmtx_table(&table_provider)?;
    } else if let Some(glyph_id) = opts.glyph {
        dump_glyph(&table_provider, glyph_id)?;
    } else {
        match &font_file {
            FontData::OpenType(font_file) => match &font_file.data {
                OpenTypeData::Single(ttf) => dump_ttf(&font_file.scope, ttf, table, flags)?,
                OpenTypeData::Collection(ttc) => dump_ttc(&font_file.scope, ttc, table, flags)?,
            },
            FontData::Woff(woff_file) => dump_woff(woff_file, table, flags)?,
            FontData::Woff2(woff_file) => dump_woff2(
                woff_file.table_data_block_scope(),
                woff_file,
                table,
                opts.index,
                flags,
            )?,
        }
    }

    if flags.encodings {
        print_cmap_encodings(&table_provider)?;
    }
    if flags.glyphs_names {
        println!();
        print_glyph_names(&table_provider)?;
    }

    Ok(0)
}

fn dump_ttc<'a>(
    scope: &ReadScope<'a>,
    ttc: &TTCHeader<'a>,
    tag: Option<Tag>,
    flags: Flags,
) -> Result<(), BoxError> {
    println!("TTC");
    println!(" - version: {}.{}", ttc.major_version, ttc.minor_version);
    println!(" - num_fonts: {}", ttc.offset_tables.len());
    println!();
    for offset_table_offset in &ttc.offset_tables {
        let offset_table_offset = usize::try_from(offset_table_offset).map_err(ParseError::from)?;
        let offset_table = scope.offset(offset_table_offset).read::<OffsetTable>()?;
        dump_ttf(scope, &offset_table, tag, flags)?;
    }
    println!();
    Ok(())
}

fn dump_ttf<'a>(
    scope: &ReadScope<'a>,
    ttf: &OffsetTable<'a>,
    tag: Option<Tag>,
    flags: Flags,
) -> Result<(), BoxError> {
    if let Some(tag) = tag {
        return dump_raw_table(ttf.read_table(scope, tag)?);
    }

    println!("TTF");
    println!(" - version: 0x{:08x}", ttf.sfnt_version);
    println!(" - num_tables: {}", ttf.table_records.len());
    println!();
    for table_record in &ttf.table_records {
        println!(
            "{} (checksum: 0x{:08x}, offset: {}, length: {})",
            DisplayTag(table_record.table_tag),
            table_record.checksum,
            table_record.offset,
            table_record.length
        );
        let table = table_record.read_table(scope)?;

        if table_record.table_tag == tag::MAXP {
            let maxp = table.read::<MaxpTable>()?;
            println!(" - num_glpyhs: {}", maxp.num_glyphs);
        }
    }
    if let Some(cff_table_data) = ttf.read_table(scope, tag::CFF)? {
        println!();
        dump_cff_table(cff_table_data)?;
    }
    println!();
    if flags.name {
        if let Some(name_table_data) = ttf.read_table(scope, tag::NAME)? {
            let name_table = name_table_data.read::<NameTable>()?;
            dump_name_table(&name_table)?;
        }
    }
    Ok(())
}

fn dump_woff(woff: &WoffFont<'_>, tag: Option<Tag>, flags: Flags) -> Result<(), BoxError> {
    let scope = &woff.scope;
    if let Some(tag) = tag {
        if let Some(entry) = woff.table_directory.iter().find(|entry| entry.tag == tag) {
            let table = entry.read_table(&woff.scope)?;

            return dump_raw_table(Some(table.scope().clone()));
        } else {
            eprintln!("Table {} not found", DisplayTag(tag));
        }

        return Ok(());
    }

    println!("TTF in WOFF");
    println!(" - num_tables: {}\n", woff.table_directory.len());

    for entry in &woff.table_directory {
        println!(
            "{} (original checksum: 0x{:08x}, compressed length: {} original length: {})",
            DisplayTag(entry.tag),
            entry.orig_checksum,
            entry.comp_length,
            entry.orig_length
        );
        let _table = entry.read_table(scope)?;
    }

    let metadata = woff.extended_metadata()?;
    if let Some(metadata) = metadata {
        println!("\nExtended Metadata:\n{}", metadata);
    }

    println!();
    if flags.name {
        if let Some(entry) = woff
            .table_directory
            .iter()
            .find(|entry| entry.tag == tag::NAME)
        {
            let table = entry.read_table(&woff.scope)?;
            let name_table = table.scope().read::<NameTable>()?;
            dump_name_table(&name_table)?;
        }
    }

    Ok(())
}

fn dump_woff2<'a>(
    scope: ReadScope<'a>,
    woff: &Woff2Font<'a>,
    tag: Option<Tag>,
    index: usize,
    flags: Flags,
) -> Result<(), BoxError> {
    if let Some(tag) = tag {
        let table = woff.read_table(tag, index)?;
        return dump_raw_table(table.as_ref().map(|buf| buf.scope()));
    }

    println!("TTF in WOFF2");
    println!(" - num tables: {}", woff.table_directory.len());
    if let Some(collection_directory) = &woff.collection_directory {
        println!(" - num fonts: {}", collection_directory.fonts().count());
    }
    println!(
        " - sizeof font data: {} compressed {} uncompressed\n",
        woff.woff_header.total_compressed_size,
        woff.table_data_block.len()
    );

    for entry in &woff.table_directory {
        println!("{} {:?}", DisplayTag(entry.tag), entry,);
    }

    let metadata = woff.extended_metadata()?;
    if let Some(metadata) = metadata {
        println!("\nExtended Metadata:\n{}", metadata);
    }

    if let Some(entry) = woff.find_table_entry(tag::GLYF, index) {
        println!();
        let table = entry.read_table(&scope)?;
        let head = woff
            .read_table(tag::HEAD, index)?
            .ok_or(ParseError::BadValue)?
            .scope()
            .read::<HeadTable>()?;
        let maxp = woff
            .read_table(tag::MAXP, index)?
            .ok_or(ParseError::BadValue)?
            .scope()
            .read::<MaxpTable>()?;
        let loca_entry = woff
            .find_table_entry(tag::LOCA, index)
            .ok_or(ParseError::BadValue)?;
        let loca = loca_entry.read_table(&woff.table_data_block_scope())?;
        let loca = loca.scope().read_dep::<Woff2LocaTable>((
            loca_entry,
            usize::from(maxp.num_glyphs),
            head.index_to_loc_format,
        ))?;
        let glyf = table.scope().read_dep::<Woff2GlyfTable>((entry, &loca))?;

        println!("Read glyf table with {} glyphs:", glyf.num_glyphs());
        for glyph in glyf.records() {
            println!("- {:?}", glyph);
        }
    }

    if flags.name {
        if let Some(table) = woff.read_table(tag::NAME, index)? {
            println!();
            let name_table = table.scope().read::<NameTable>()?;
            dump_name_table(&name_table)?;
        }
    }

    Ok(())
}

fn dump_name_table(name_table: &NameTable) -> Result<(), ParseError> {
    for name_record in &name_table.name_records {
        let platform = name_record.platform_id;
        let encoding = name_record.encoding_id;
        let language = name_record.language_id;
        let offset = usize::from(name_record.offset);
        let length = usize::from(name_record.length);
        let name_data = name_table
            .string_storage
            .offset_length(offset, length)?
            .data();
        let name = match (platform, encoding, language) {
            (0, _, _) => decode(UTF_16BE, name_data),
            (1, 0, _) => decode(MACINTOSH, name_data),
            (3, 0, _) => decode(UTF_16BE, name_data),
            (3, 1, _) => decode(UTF_16BE, name_data),
            (3, 10, _) => decode(UTF_16BE, name_data),
            _ => format!(
                "(unknown platform={} encoding={} language={})",
                platform, encoding, language
            ),
        };
        match get_name_meaning(name_record.name_id) {
            Some(meaning) => println!("{}", meaning,),
            None => println!("name {}", name_record.name_id,),
        }
        println!("{:?}", name);
        println!();
    }

    if let Some(langtag_records) = &name_table.opt_langtag_records {
        for langtag in langtag_records.iter() {
            let name_data = name_table
                .string_storage
                .offset_length(langtag.offset.into(), langtag.length.into())?
                .data();
            let name = decode(UTF_16BE, name_data);
            println!("langtag {}", name);
        }
    }

    Ok(())
}

fn dump_head_table(provider: &impl FontTableProvider) -> Result<(), ParseError> {
    let head = ReadScope::new(&provider.read_table_data(tag::HEAD)?).read::<HeadTable>()?;
    println!("{:#?}", head);
    Ok(())
}

fn dump_hmtx_table(provider: &impl FontTableProvider) -> Result<(), ParseError> {
    let table = provider.table_data(tag::MAXP)?.expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let hhea = ReadScope::new(&provider.read_table_data(tag::HHEA)?).read::<HheaTable>()?;

    let num_glyphs = usize::from(maxp.num_glyphs);
    let num_metrics = usize::from(hhea.num_h_metrics);
    let hmtx_data = provider.table_data(tag::HMTX)?.expect("no hmtx table");
    let hmtx = ReadScope::new(&hmtx_data).read_dep::<HmtxTable<'_>>((num_glyphs, num_metrics))?;

    println!("hmtx:");
    for (index, metrics) in hmtx.h_metrics.iter().enumerate() {
        println!("{}: {:?}", index, metrics);
    }

    Ok(())
}

fn dump_loca_table(provider: &impl FontTableProvider) -> Result<(), ParseError> {
    let table = provider.table_data(tag::HEAD)?.expect("no head table");
    let scope = ReadScope::new(table.borrow());
    let head = scope.read::<HeadTable>()?;

    let table = provider.table_data(tag::MAXP)?.expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let table = provider.table_data(tag::LOCA)?.expect("no loca table");
    let scope = ReadScope::new(table.borrow());
    let loca =
        scope.read_dep::<LocaTable>((usize::from(maxp.num_glyphs), head.index_to_loc_format))?;

    println!("loca:");
    for (glyph_id, offset) in loca.offsets.iter().enumerate() {
        println!("{}: {}", glyph_id, offset);
    }

    Ok(())
}

fn dump_cff_table<'a>(scope: ReadScope<'a>) -> Result<(), ParseError> {
    let cff = scope.read::<CFF>()?;

    println!("- CFF:");
    println!(" - version: {}.{}", cff.header.major, cff.header.minor);
    for obj in cff.name_index.iter() {
        let name = String::from_utf8_lossy(obj);
        println!(" - name: {}", name);
    }

    if cff.name_index.len() != 1 {
        return Err(ParseError::BadIndex);
    }
    let font = cff.fonts.get(0).ok_or(ParseError::MissingValue)?;
    let char_strings_index = &font.char_strings_index;
    println!(" - num glyphs: {}", char_strings_index.len());
    println!(
        " - charset: {}",
        match font.charset {
            Charset::ISOAdobe => "ISO Adobe",
            Charset::Expert => "Expert",
            Charset::ExpertSubset => "Expert Subset",
            Charset::Custom(_) => "Custom",
        }
    );
    println!(
        " - variant: {}",
        match font.data {
            CFFVariant::CID(_) => "CID",
            CFFVariant::Type1(_) => "Type 1",
        }
    );
    println!();
    println!(" - Top DICT");
    dump_cff_dict(&cff, &font.top_dict, 2);
    match &font.data {
        CFFVariant::Type1(ref type1) => {
            println!();
            println!(
                " - encoding: {}",
                match type1.encoding {
                    cff::Encoding::Standard => "Standard",
                    cff::Encoding::Expert => "Expert",
                    cff::Encoding::Custom(_) => "Custom",
                }
            );
            println!();
            println!(" - Private DICT");
            dump_cff_dict(&cff, &type1.private_dict, 2);
            let (subrs_count, subrs_size) = match type1.local_subr_index {
                Some(ref index) => (index.len(), index.data_len()),
                None => (0, 0),
            };
            println!(" - Local subrs: {} ({} bytes)", subrs_count, subrs_size);
        }
        CFFVariant::CID(cid) => {
            for (i, object) in cid.font_dict_index.iter().enumerate() {
                println!();
                println!(" - Font DICT {}", i);
                let font_dict = ReadScope::new(object).read_dep::<FontDict>(cff::MAX_OPERANDS)?;
                dump_cff_dict(&cff, &font_dict, 2);
                println!();
                println!("  - Private DICT");
                let (private_dict, _private_dict_offset) =
                    font_dict.read_private_dict::<cff::PrivateDict>(&scope, cff::MAX_OPERANDS)?;
                dump_cff_dict(&cff, &private_dict, 4);
            }
            let (subrs_count, subrs_size) =
                cid.local_subr_indices
                    .iter()
                    .fold((0, 0), |(mut count, mut size), index| {
                        if let Some(index) = index {
                            count += index.len();
                            size += index.data_len();
                        }
                        (count, size)
                    });
            println!();
            println!(
                " - Local subrs: {} ({} bytes) in {} indices",
                subrs_count,
                subrs_size,
                cid.local_subr_indices.len()
            );
        }
    }
    println!(
        " - Global subrs: {} ({} bytes)",
        cff.global_subr_index.len(),
        cff.global_subr_index.data_len()
    );

    Ok(())
}

fn dump_glyph(provider: &impl FontTableProvider, glyph_id: u16) -> Result<(), ParseError> {
    let table = provider.table_data(tag::HEAD)?.expect("no head table");
    let scope = ReadScope::new(table.borrow());
    let head = scope.read::<HeadTable>()?;

    let table = provider.table_data(tag::MAXP)?.expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let table = provider.table_data(tag::LOCA)?.expect("no loca table");
    let scope = ReadScope::new(table.borrow());
    let loca =
        scope.read_dep::<LocaTable>((usize::from(maxp.num_glyphs), head.index_to_loc_format))?;

    let table = provider.table_data(tag::GLYF)?.expect("no glyf table");
    let scope = ReadScope::new(table.borrow());
    let glyf = scope.read_dep::<GlyfTable>(&loca)?;

    let mut glyph = glyf
        .records()
        .get(usize::from(glyph_id))
        .ok_or(ParseError::BadValue)?
        .clone();
    glyph.parse()?;
    println!("{:#?}", glyph);

    Ok(())
}

fn dump_cff_dict<T: cff::DictDefault>(cff: &CFF, dict: &cff::Dict<T>, indent: usize) {
    for x in dict.iter().map(|(op, ops)| (op, ops.as_slice())) {
        match x {
            // For operators with a string id operand, resolve the string
            (op @ Operator::Version, &[Operand::Integer(sid)])
            | (op @ Operator::Notice, &[Operand::Integer(sid)])
            | (op @ Operator::Copyright, &[Operand::Integer(sid)])
            | (op @ Operator::FullName, &[Operand::Integer(sid)])
            | (op @ Operator::FamilyName, &[Operand::Integer(sid)])
            | (op @ Operator::Weight, &[Operand::Integer(sid)])
            | (op @ Operator::BaseFontName, &[Operand::Integer(sid)]) => {
                let string = u16::try_from(sid)
                    .ok()
                    .and_then(|sid| cff.read_string(sid).ok())
                    .unwrap_or("<unable to read>");
                println!("{:indent$}- {:?}: {}", " ", op, string);
            }
            (
                op @ Operator::ROS,
                &[Operand::Integer(registry), Operand::Integer(ordering), Operand::Integer(supplement)],
            ) => {
                let registry = u16::try_from(registry)
                    .ok()
                    .and_then(|sid| cff.read_string(sid).ok())
                    .unwrap_or("<unable to read>");
                let ordering = u16::try_from(ordering)
                    .ok()
                    .and_then(|sid| cff.read_string(sid).ok())
                    .unwrap_or("<unable to read>");
                println!(
                    "{:indent$}- {:?}: {}-{}-{}",
                    " ", op, registry, ordering, supplement
                );
            }
            (op, operands) => println!("{:indent$}- {:?}: {:?}", " ", op, operands),
        }
    }
}

fn dump_raw_table(scope: Option<ReadScope>) -> Result<(), BoxError> {
    if let Some(scope) = scope {
        io::stdout()
            .write_all(scope.data())
            .map_err(|err| err.into())
    } else {
        Err(ErrorMessage("Table not found").into())
    }
}

fn get_name_meaning(name_id: u16) -> Option<&'static str> {
    match name_id {
        NameTable::COPYRIGHT_NOTICE => Some("Copyright"),
        NameTable::FONT_FAMILY_NAME => Some("Font Family"),
        NameTable::FONT_SUBFAMILY_NAME => Some("Font Subfamily"),
        NameTable::UNIQUE_FONT_IDENTIFIER => Some("Unique Identifier"),
        NameTable::FULL_FONT_NAME => Some("Full Font Name"),
        NameTable::VERSION_STRING => Some("Version"),
        NameTable::POSTSCRIPT_NAME => Some("PostScript Name"),
        NameTable::TRADEMARK => Some("Trademark"),
        NameTable::MANUFACTURER_NAME => Some("Manufacturer"),
        NameTable::DESIGNER => Some("Designer"),
        NameTable::DESCRIPTION => Some("Description"),
        NameTable::URL_VENDOR => Some("URL Vendor"),
        NameTable::URL_DESIGNER => Some("URL Designer"),
        NameTable::LICENSE_DESCRIPTION => Some("License Description"),
        NameTable::LICENSE_INFO_URL => Some("License Info URL"),
        NameTable::TYPOGRAPHIC_FAMILY_NAME => Some("Typographic Family"),
        NameTable::TYPOGRAPHIC_SUBFAMILY_NAME => Some("Typographic Subfamily"),
        NameTable::COMPATIBLE_FULL => Some("Compatible Full"),
        NameTable::SAMPLE_TEXT => Some("Sample Text"),
        NameTable::POSTSCRIPT_CID_FINDFONT_NAME => Some("PostScript CID findfont"),
        NameTable::WWS_FAMILY_NAME => Some("WWS Family Name"),
        NameTable::WWS_SUBFAMILY_NAME => Some("WWS Subfamily Name"),
        NameTable::LIGHT_BACKGROUND_PALETTE => Some("Light Background Palette"),
        NameTable::DARK_BACKGROUND_PALETTE => Some("Dark Background Palette"),
        NameTable::VARIATIONS_POSTSCRIPT_NAME_PREFIX => Some("Variations PostScript Name Prefix"),
        _ => None,
    }
}

fn print_glyph_names(provider: &impl FontTableProvider) -> Result<(), ParseError> {
    let table = provider.table_data(tag::MAXP)?.expect("no maxp table");
    let scope = ReadScope::new(table.borrow());
    let maxp = scope.read::<MaxpTable>()?;

    let post_data = provider
        .table_data(tag::POST)
        .ok()
        .and_then(convert::identity)
        .map(|data| Box::from(&*data));

    let table = provider.table_data(tag::CMAP)?;
    let scope = table.as_ref().map(|data| ReadScope::new(data.borrow()));
    let cmap = scope.map(|scope| scope.read::<Cmap<'_>>()).transpose()?;

    let cmap_subtable = cmap
        .as_ref()
        .and_then(|cmap| read_cmap_subtable(cmap).ok())
        .and_then(convert::identity);

    let names = GlyphNames::new(&cmap_subtable, post_data);
    for glyph_id in 0..maxp.num_glyphs {
        let name = names.glyph_name(glyph_id);
        println!("{}: {}", glyph_id, name);
    }

    Ok(())
}

fn print_cmap_encodings(provider: &impl FontTableProvider) -> Result<(), ParseError> {
    let table = provider.table_data(tag::CMAP)?.expect("no cmap table");
    let scope = ReadScope::new(table.borrow());
    let cmap = scope.read::<Cmap<'_>>()?;

    println!("cmap encodings:");
    for record in cmap.encoding_records() {
        print!(" - {:?} {:?} ", record.platform_id, record.encoding_id);
        if let Ok(subtable) = cmap
            .scope
            .offset(usize::try_from(record.offset)?)
            .read::<CmapSubtable<'_>>()
        {
            match subtable {
                CmapSubtable::Format0 { .. } => println!("Sub-table format 0"),
                CmapSubtable::Format2 { .. } => println!("Sub-table format 2"),
                CmapSubtable::Format4 { .. } => println!("Sub-table format 4"),
                CmapSubtable::Format6 { .. } => println!("Sub-table format 6"),
                CmapSubtable::Format10 { .. } => println!("Sub-table format 10"),
                CmapSubtable::Format12 { .. } => println!("Sub-table format 12"),
            }
        } else {
            println!("Unable to read sub-table.");
        }
    }

    Ok(())
}

impl From<&DumpOpts> for Flags {
    fn from(opts: &DumpOpts) -> Self {
        Flags {
            encodings: opts.encodings,
            glyphs_names: opts.glyph_names,
            name: opts.name,
        }
    }
}
