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
    fs::write("test.mmd", percy::transform(src)).expect("Failed to write output to file.");
}
