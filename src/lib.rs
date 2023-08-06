use std::error::Error;

mod consts;
pub mod mermaid;
pub mod scanner;

pub fn transform(src: String) -> Result<String, Box<dyn Error>> {
    let mut lines = vec![];
    mermaid::ClassDiagram::make(scanner::lex(src)?, &mut lines);
    Ok(lines.join("\r\n"))
}
