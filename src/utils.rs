use std::{fs, io, path::Path};

pub fn read_files(dir: &Path, src: Option<String>) -> Result<String, io::Error> {
    let mut src = src.unwrap_or(String::new());
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                src = read_files(&path, Option::Some(src))?;
            } else {
                let contents = fs::read_to_string(entry.path())?;
                src.push_str(&contents);
            }
        }
    }
    Ok(src)
}
