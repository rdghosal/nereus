use crate::{consts, lexer::PyMethodAccess, parser::Node};
use std::rc::Rc;

pub fn make(node: Rc<Node>, lines: &mut Vec<String>) {
    if lines.is_empty() {
        lines.push("classDiagram".to_string());
        lines.push(format!("{}class `pydantic.BaseModel`", consts::INDENT));
    }

    let inherits = " <|-- ";
    if node.is_root {
        lines.push(format!(
            "{}`pydantic.BaseModel`{}{}",
            consts::INDENT,
            inherits,
            node.model.class_name
        ));
    }

    // Define class as well as the fields and methods therein.
    let class_name = format!("{}class {}{{", consts::INDENT, node.model.class_name);
    lines.push(class_name);
    for field in &node.model.fields {
        lines.push(format!(
            "{}{}+{} {}",
            consts::INDENT,
            consts::INDENT,
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
            consts::INDENT,
            consts::INDENT,
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
    lines.push(format!("{}}}", consts::INDENT));

    // Declare relationship with child classes, whose respective
    // class definitions are to follow.
    for child in node.children.borrow().iter() {
        lines.push(format!(
            "{}{}{}{}",
            consts::INDENT,
            &node.model.class_name,
            inherits,
            &child.model.class_name
        ));
    }
    for child in node.children.borrow().iter() {
        make(child.clone(), lines);
    }
}
