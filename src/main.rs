use std::{env, format, fs, io, path::Path, process};

#[derive(Debug)]
struct PydanticModel {
    class_name: String,
    parents: Vec<String>,
    fields: Vec<(String, String)>,
}

const INDENT: &str = "    ";

fn lex(source: String) -> Vec<PydanticModel> {
    let mut models = vec![];
    let mut i = 0;
    let lines = source.split("\n").collect::<Vec<_>>();
    while i < lines.len() {
        // Cannot trim as whitespace is significant in Python
        // let line = lines[i].trim();
        let line = lines[i];
        if line.starts_with("class") {
            // Scan class names, including those of super classes.
            let mut class_name = line.split(' ').collect::<Vec<&str>>()[1];
            let parents: Vec<String>;
            match class_name.find('(') {
                Some(start) => {
                    let end = class_name.find(")").unwrap();
                    let parent_args = &class_name[start..end];
                    parents = parent_args
                        .split(", ")
                        .map(|p| p.to_string())
                        .collect::<Vec<String>>();
                    class_name = &class_name[..start];
                }
                None => {
                    eprintln!("Detected invalid syntax in class: {}", class_name);
                    process::exit(-3);
                }
            };
            i += 1;

            // Consume decorators and methods.
            if lines[i].starts_with(&format!("@")) {
                i += 1;
                continue;
            } else if lines[i].starts_with(&format!("{}def", INDENT)) {
                while lines[i].starts_with(&format!("{}{}", INDENT, INDENT)) {
                    i += 1;
                }
            }

            // Scan fields.
            // In pydantic, fields are denoted as `field_name: type`.
            let mut fields: Vec<(String, String)> = vec![];
            while lines[i].starts_with(INDENT) && lines[i].contains(": ") {
                // Remove leading indent.
                let curr_line = lines[i].trim();
                let field_and_type: Vec<&str> = curr_line.split(": ").collect();
                fields.push((field_and_type[0].to_string(), field_and_type[1].to_string()));
                i += 1;
            }

            models.push(PydanticModel {
                class_name: class_name.to_string(),
                parents,
                fields,
            })
        }
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
    read_files(Path::new(&args[1]), &mut source).expect("oops");
    let models = lex(source);
    dbg!("{?#}", models);
}
