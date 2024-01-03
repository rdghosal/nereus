use std::{fs, io, path::Path};

pub fn read_files(path: &Path, src: Option<String>) -> Result<String, io::Error> {
    let mut src = src.unwrap_or_default();
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            match entry {
                Ok(e) => {
                    let nested = e.path();
                    if nested.is_dir() {
                        src = read_files(&nested, Some(src))?;
                    } else {
                        let contents = fs::read_to_string(nested)?;
                        src.push_str(&contents);
                    }
                }
                Err(..) => continue,
            }
        }
    } else {
        let contents = fs::read_to_string(path)?;
        src.push_str(&contents);
    }
    Ok(src)
}
