use std::error::Error;

mod consts;
pub mod mermaid;
pub mod parser;
pub mod scanner;

pub fn transform(src: String) -> Result<String, Box<dyn Error>> {
    let nodes = parser::parse(&mut scanner::lex(src)?)?;
    let mut lines = vec![];
    mermaid::ClassDiagram::make(nodes, &mut lines);
    Ok(lines.join("\r\n"))
}
