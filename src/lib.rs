use std::{error::Error, path::Path};

mod consts;
pub mod mermaid;
mod models;
pub mod parser;
mod utils;

pub fn from_src(src: String) -> Result<String, Box<dyn Error>> {
    let result = mermaid::ClassDiagram::make(parser::parse(src)?)?;
    Ok(result)
}

pub fn from_path(path: &Path) -> Result<String, Box<dyn Error>> {
    let src = utils::read_files(&path, None)?;
    let result = mermaid::ClassDiagram::make(parser::parse(src)?)?;
    Ok(result)
}
