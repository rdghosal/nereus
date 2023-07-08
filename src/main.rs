use std::{env, fs, io, path::Path, process};

struct PydanticModel {
    name: String,
    parent: String,
    fields: Vec<(String, String)>,
}

const INDENT: &str = "     ";

fn lex(source: String) -> Vec<PydanticModel> {
    let mut models = vec![];
    let mut i = 0;
    let lines = source.split("\n").collect::<Vec<_>>();
    while i < lines.len() {
        // Cannot trim as whitespace is significant in Python
        // let line = lines[i].trim();
        let line = lines[i];
        if line.starts_with("class") {
            // TODO: parse superclasses
            // TODO: omit parens and trailing colon
            let class_name = line.split(' ').collect::<Vec<&str>>()[1];
            let mut fields: Vec<(String, String)> = vec![];
            i += 1;

            while !lines[i].starts_with("class")
                || !lines[i].starts_with(&format!("{}class", INDENT))
            {
                let curr_line = &lines[i].trim();
                if curr_line.contains(": ") {
                    let field_and_type: Vec<&str> = curr_line.split(": ").collect();
                    fields.push((field_and_type[0].to_string(), field_and_type[1].to_string()));
                    i += 1;
                }
            }
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
