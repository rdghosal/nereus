use std::{cell::RefCell, collections::HashMap, env, format, fs, io, path::Path, process, rc::Rc};

use percy::{
    lexer,
    models::{Node, PyMethodAccess},
    parser,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: percy <path to .py files>");
        process::exit(-1);
    }
    let mut source = String::new();
    read_files(Path::new(&args[1]), &mut source).expect("oops");
    let models = lexer::lex(source);
    // dbg!("{?#}", &models);
    let nodes = parser::parse(models);
    // dbg!("{?#}", &nodes);
    let mut lines = vec![
        "classDiagram".to_string(),
        format!("{}class `pydantic.BaseModel`", lexer::INDENT),
    ];
    if nodes.is_empty() {
        eprintln!("Failed to identify child classes of `pydantic.BaseModel`");
        process::exit(-5)
    }
    for node in nodes {
        lines = make_mermaid_cls(node, lines);
    }
    fs::write("test.mmd", lines.join("\r\n")).expect("oopsie");
}

fn read_files(dir: &Path, source: &mut String) -> Result<(), io::Error> {
    // dbg!("using path {}", &dir);
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                read_files(&path, source)?;
            } else {
                let contents = fs::read_to_string(entry.path())?;
                source.push_str(&contents);
            }
        }
    }
    Ok(())
}

fn make_mermaid_cls(node: Rc<Node>, mut lines: Vec<String>) -> Vec<String> {
    // Declare relationship with pydantic.BaseModel.
    let inherits = " <|-- ";
    if node.is_root {
        lines.push(format!(
            "{}`pydantic.BaseModel`{}{}",
            lexer::INDENT,
            inherits,
            node.model.class_name
        ));
    }

    // Define class as well as the fields and methods therein.
    let class_name = format!("{}class {}{{", lexer::INDENT, node.model.class_name);
    lines.push(class_name);
    for field in &node.model.fields {
        lines.push(format!(
            "{}{}+{} {}",
            lexer::INDENT,
            lexer::INDENT,
            field.0,
            field.1
        ));
    }
    for method in &node.model.methods {
        let access_modifier: &str;
        match method.access {
            PyMethodAccess::Public => access_modifier = "+",
            PyMethodAccess::Private => access_modifier = "-",
        }

        let mut method_str = format!(
            "{}{}{}{}(",
            lexer::INDENT,
            lexer::INDENT,
            access_modifier,
            method.name,
        );
        let mut args: Vec<String> = vec![];
        for (arg_name, type_) in method.args.clone() {
            let type_ = type_.unwrap_or_default();
            if type_.is_empty() {
                args.push(arg_name);
            } else {
                args.push(format!("{} {}", type_, arg_name));
            }
        }
        if args.len() > 0 {
            let args_str = args.join(", ");
            method_str.push_str(args_str.as_str());
        }
        method_str.push_str(")");
        if let Option::Some(return_type) = &method.returns {
            method_str.push_str(format!(" {}", return_type).as_str());
        }
        lines.push(method_str);
    }
    lines.push(format!("{}}}", lexer::INDENT));

    // Declare relationship with child classes, whose respective
    // class definitions are to follow.
    for child in node.children.borrow().iter() {
        lines.push(format!(
            "{}{}{}{}",
            lexer::INDENT,
            &node.model.class_name,
            inherits,
            &child.model.class_name
        ));
    }
    for child in node.children.borrow().iter() {
        lines = make_mermaid_cls(child.clone(), lines);
    }
    lines
}
