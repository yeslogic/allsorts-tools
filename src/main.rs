use gumdrop::Options;
use std::process;

use allsorts_tools::cli::*;
use allsorts_tools::{dump, has_table, shape, subset, validate};

fn main() {
    let cli = Cli::parse_args_default_or_exit();

    let res = match cli.command {
        Some(Command::Dump(opts)) => dump::main(opts),
        Some(Command::HasTable(opts)) => has_table::main(opts),
        Some(Command::Shape(opts)) => shape::main(opts),
        Some(Command::Subset(opts)) => subset::main(opts),
        Some(Command::Validate(opts)) => validate::main(opts),
        None => usage(),
    };

    match res {
        Ok(code) if code != 0 => process::exit(code),
        Ok(_) => (),
        Err(err) => {
            eprint!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn usage() -> ! {
    eprintln!("{}", Cli::command_list().unwrap());
    process::exit(2)
}
