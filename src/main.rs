use percy::{mermaid, parser, scanner};
use std::{env, fs, path::Path, process};
mod utils;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: percy <path to .py files>");
        process::exit(-1);
    }
    let src = utils::read_files(Path::new(&args[1]), Option::None)
        .expect("Failed to read source file(s).");
    let nodes = parser::parse(scanner::lex(src));
    let mut lines = vec![];
    for node in nodes {
        mermaid::make(node, &mut lines);
    }
    fs::write("test.mmd", lines.join("\r\n")).expect("Failed to write output to file.");
}
