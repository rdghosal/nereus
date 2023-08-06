use crate::{consts, parser::Node, scanner::PyMethodAccess};
use std::rc::Rc;

pub struct ClassDiagram;
impl ClassDiagram {
    pub fn make(nodes: Vec<Rc<Node>>, lines: &mut Vec<String>) {
        let inherits = " <|-- ";
        for node in nodes.iter() {
            if lines.is_empty() {
                lines.push("classDiagram".to_string());
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
                if method.is_dunder() {
                    continue;
                }
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

            for parent in node.model.parents.iter() {
                lines.push(format!(
                    "{}`{}`{}{}",
                    consts::INDENT,
                    parent,
                    inherits,
                    node.model.class_name
                ));
            }
        }
    }
}
