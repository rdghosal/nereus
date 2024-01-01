use std::error::Error;

mod consts;
pub mod mermaid;
mod models;
pub mod parser;

pub fn transform(src: &str) -> Result<String, Box<dyn Error>> {
    let result = mermaid::ClassDiagram::make(parser::parse(src.to_string())?)?;
    Ok(result.join("\r\n"))
}
