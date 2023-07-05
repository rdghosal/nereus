use std::{env, fs, io, path::Path, process};

struct PydanticModel {
    name: String,
    parent: String,
    fields: Vec<(String, String)>,
}

fn lex(source: String) -> Vec<PydanticModel> {
    let mut models = vec![];
    let mut i = 0;
    let lines = source.split("\n").collect::<Vec<_>>();
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("class") {
            let class_name = line.split(' ').collect::<Vec<&str>>()[1];
            let mut fields = vec![];
            i += 1;

            let curr_line = lines[i];
            while !curr_line.starts_with("class") {}
        }
        i += 1;
    }
    models
}

fn read_files(dir: &Path, source: &mut String) -> Result<(), io::Error> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                read_files(&path, source)?;
            } else {
                let contents = fs::read_to_string(entry.path()).expect("oops");
                source.push_str(&contents);
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: percy <path to .py files>");
        process::exit(64);
    }
    let mut source = String::new();
    let _ = read_files(Path::new(&args[1]), &mut source);
    dbg!("{:?}", source);
}
