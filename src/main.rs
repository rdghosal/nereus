use std::{env, fs, io, path::Path, process};

#[derive(Debug)]
struct Source {
    pub contents: String,
    file_count: usize,
}

impl Source {
    fn new() -> Self {
        Source {
            contents: String::new(),
            file_count: 0,
        }
    }

    pub fn append(&mut self, contents: &str) {
        self.contents.push_str(contents);
        self.file_count += 1;
    }
}

fn read_files(dir: &Path, source: &mut Source) -> Result<(), io::Error> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                read_files(&path, source)?;
            } else {
                source.append(&fs::read_to_string(entry.path()).expect("oops"));
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
    let mut s = Source::new();
    let _ = read_files(Path::new(&args[1]), &mut s);
    dbg!("{:?}", s.contents);
}
