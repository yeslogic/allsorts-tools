use allsorts_tools::{dump, shape, subset};

fn main() {
    match std::env::args().nth(1).as_ref().map(String::as_str) {
        Some("subset") => subset::main(),
        Some("dump") => dump::main(),
        Some("shape") => shape::main(),
        Some(_) | None => usage(),
    }
}

fn usage() {
    eprintln!("Usage: allsorts dump\n       allsorts shape\n       allsorts subset");
    std::process::exit(1)
}
