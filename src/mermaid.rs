use crate::{
    consts::INDENT,
    models::{PyClass, PyMethodAccess},
};

#[derive(Debug)]
pub struct MermaidError(String);
impl std::fmt::Display for MermaidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for MermaidError {}

pub struct ClassDiagram;
impl ClassDiagram {
    pub fn make(models: Vec<PyClass>) -> Result<String, MermaidError> {
        let mut result = vec![];
        let inherits = " <|-- ";
        for model in models.iter() {
            // Define class as well as the fields and methods therein.
            result.push(String::from("classDiagram"));

            // Define class as well as the fields and methods therein.
            let class_name = format!("{INDENT}class {}{{", model.name);
            result.push(class_name);
            result.extend(Self::make_class_fields(model));
            result.extend(Self::make_class_methods(model));
            for parent in model.parents.iter() {
                result.push(format!("{INDENT}`{parent}`{inherits}{}", model.name));
            }
        }
        Ok(result.join("\r\n"))
    }

    fn make_class_methods(model: &PyClass) -> Vec<String> {
        let mut result = Vec::with_capacity(model.methods.len());
        for method in &model.methods {
            if method.is_dunder() {
                continue;
            }
            let access_modifier = match method.access {
                PyMethodAccess::Public => '+',
                PyMethodAccess::Private => '-',
            };

            let mut method_str = format!("{INDENT}{INDENT}{access_modifier}{}(", method.name);
            let mut args: Vec<String> = vec![];
            for arg in method.args.iter() {
                if let Some(t) = &arg.dtype {
                    args.push(format!("{t} {}", arg.name));
                } else {
                    args.push(arg.name.to_owned());
                }
            }
            if args.len() > 0 {
                let args_str = args.join(", ");
                method_str.push_str(args_str.as_str());
            }
            method_str.push_str(")");
            if let Some(return_type) = &method.returns {
                method_str.push_str(&format!(" {return_type}"));
            }
            result.push(method_str);
        }
        result.push(format!("{}}}", INDENT));
        result
    }

    fn make_class_fields(model: &PyClass) -> Vec<String> {
        let mut result = Vec::with_capacity(model.fields.len());
        for field in model.fields.iter() {
            let line = match (&field.dtype, &field.default) {
                (Some(t), _) => {
                    format!("{INDENT}{INDENT}+{} {t}", field.name)
                }
                (_, Some(d)) => {
                    format!("{INDENT}{INDENT}+{} = {d}", field.name)
                }
                _ => format!("{INDENT}{INDENT}+{}", field.name),
            };
            result.push(line);
        }
        result
    }
}
