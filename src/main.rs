use gumdrop::Options;

use allsorts_tools::cli::*;
use allsorts_tools::{dump, shape, subset};

fn main() {
    let cli = Cli::parse_args_default_or_exit();

    let res = match cli.command {
        Some(Command::Dump(opts)) => dump::main(opts),
        Some(Command::Shape(opts)) => shape::main(opts),
        Some(Command::Subset(opts)) => subset::main(opts),
        None => usage(),
    };

    if let Err(err) = res {
        eprint!("Error: {}", err);
        std::process::exit(1);
    }
}

fn usage() -> ! {
    eprintln!("{}", Cli::command_list().unwrap());
    std::process::exit(1)
}
