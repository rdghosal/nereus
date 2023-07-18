mod consts;
pub mod mermaid;
pub mod parser;
pub mod scanner;

pub fn transform(src: String) -> String {
    let nodes = parser::parse(scanner::lex(src));
    let mut lines = vec![];
    for node in nodes {
        mermaid::ClassDiagram::make(node, &mut lines);
    }
    lines.join("\r\n")
}
