use std::{io, process};
use strip_ansi_escapes::Writer;

extern crate strip_ansi_escapes;

fn main() {
    let mut writer = Writer::new(io::stdout());

    if let Err(error) = std::io::copy(&mut io::stdin(), &mut writer) {
        eprintln!("I/O error copying stdin to stdout: {}", error);
        process::exit(1);
    }
}
