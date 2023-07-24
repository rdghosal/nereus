use std::{env, fs, path::Path, process};
mod utils;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: nereus <path to .py files>");
        process::exit(-1);
    }
    let src = utils::read_files(Path::new(&args[1]), Option::None)
        .expect("Failed to read source file(s).");
    match nereus::transform(src) {
        Ok(out) => fs::write("test.mmd", out).expect("Failed to write output to file."),
        Err(err) => {
            eprintln!("Failed to generate mermaid. Found error: {err}")
        }
    };
}
