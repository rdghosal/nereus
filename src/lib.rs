use std::error::Error;

mod consts;
pub mod mermaid;
mod models;
pub mod parser;

pub fn transform(src: &str) -> Result<String, Box<dyn Error>> {
    let mut lines = vec![];
    mermaid::ClassDiagram::make(parser::parse(src.to_string())?, &mut lines)?;
    Ok(lines.join("\r\n"))
}
