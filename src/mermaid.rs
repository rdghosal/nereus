use crate::{
    consts,
    scanner::{PyClass, PyMethodAccess},
};

pub struct ClassDiagram;
impl ClassDiagram {
    pub fn make(models: Vec<PyClass>, lines: &mut Vec<String>) -> Result<(), &str> {
        let inherits = " <|-- ";
        for model in models.iter() {
            if lines.is_empty() {
                lines.push("classDiagram".to_string());
            }

            // Define class as well as the fields and methods therein.
            let class_name = format!("{}class {}{{", consts::INDENT, model.name);
            lines.push(class_name);
            for prop in model.props.iter() {
                let line = if prop.type_.is_some() {
                    format!(
                        "{}{}+{} {}",
                        consts::INDENT,
                        consts::INDENT,
                        prop.name,
                        prop.type_.clone().unwrap()
                    )
                } else if prop.default.is_some() {
                    format!(
                        "{}{}+{} = {}",
                        consts::INDENT,
                        consts::INDENT,
                        prop.name,
                        prop.default.clone().unwrap()
                    )
                } else {
                    format!("{}{}+{}", consts::INDENT, consts::INDENT, prop.name)
                };
                lines.push(line);
            }

            for method in &model.methods {
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
                for param in method.params.clone() {
                    let type_ = param.type_.unwrap_or_default();
                    if type_.is_empty() {
                        args.push(param.name);
                    } else {
                        args.push(format!("{} {}", type_, param.name));
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

            for parent in model.parents.iter() {
                lines.push(format!(
                    "{}`{}`{}{}",
                    consts::INDENT,
                    parent,
                    inherits,
                    model.name
                ));
            }
        }
        Ok(())
    }
}
