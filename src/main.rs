use std::ffi::OsStr;
use std::{env, process};

use allsorts_tools::cli::*;
use allsorts_tools::{
    bitmaps, cmap, dump, has_table, instance, layout_features, shape, specimen, subset, svg,
    validate, variations, view, BoxError,
};
use gumdrop::Options;

fn main() {
    let res = if env::args_os()
        .any(|arg| arg_starts_with(&arg, "--engine") || arg_starts_with(&arg, "--testcase"))
    {
        text_rendering_test_main()
    } else {
        allsorts_main()
    };

    match res {
        Ok(code) if code != 0 => process::exit(code),
        Ok(_) => (),
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn allsorts_main() -> Result<i32, BoxError> {
    let cli = Cli::parse_args_default_or_exit();

    match cli.command {
        Some(Command::Bitmaps(opts)) => bitmaps::main(opts),
        Some(Command::Cmap(opts)) => cmap::main(opts),
        Some(Command::Dump(opts)) => dump::main(opts),
        Some(Command::HasTable(opts)) => has_table::main(opts),
        Some(Command::Instance(opts)) => instance::main(opts),
        Some(Command::LayoutFeatures(opts)) => layout_features::main(opts),
        Some(Command::Shape(opts)) => shape::main(opts),
        Some(Command::Specimen(opts)) => specimen::main(opts),
        Some(Command::Subset(opts)) => subset::main(opts),
        Some(Command::Svg(opts)) => svg::main(opts),
        Some(Command::Validate(opts)) => validate::main(opts),
        Some(Command::Variations(opts)) => variations::main(opts),
        Some(Command::View(opts)) => view::main(opts),
        None => usage(),
    }
}

/// Special code path to conform to the CLI interface expected by the unicode text rendering tests
/// https://github.com/unicode-org/text-rendering-tests
fn text_rendering_test_main() -> Result<i32, BoxError> {
    if env::args_os().any(|arg| arg == "--version") {
        println!("Allsorts/{}", allsorts::VERSION);
        Ok(0)
    } else {
        let opts = SvgOpts::parse_args_default_or_exit();
        svg::main(opts)
    }
}

fn usage() -> ! {
    eprintln!("{}", Cli::command_list().unwrap());
    process::exit(2)
}

fn arg_starts_with(arg: &OsStr, prefix: &str) -> bool {
    arg.to_str().map_or(false, |s| s.starts_with(prefix))
}
