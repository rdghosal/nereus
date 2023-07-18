use crate::consts;
use std::process;

const PYDANTIC_BASE_MODEL_REFS: [&str; 2] = ["pydantic.BaseModel", "BaseModel"];

#[derive(Default, Debug, Clone)]
pub enum PyMethodAccess {
    #[default]
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct PyMethod {
    pub name: String,
    pub args: Vec<(String, Option<String>)>,
    pub returns: Option<String>,
    pub access: PyMethodAccess,
}

#[derive(Debug, Default, Clone)]
pub struct PydanticModel {
    pub class_name: String,
    pub parents: Vec<String>,
    pub fields: Vec<(String, String)>,
    pub methods: Vec<PyMethod>,
}

impl PydanticModel {
    pub fn is_base_model(class_name: &String) -> bool {
        PYDANTIC_BASE_MODEL_REFS.contains(&class_name.as_str())
    }

    pub fn inherits_base_model(&self) -> bool {
        let mut inherits = false;
        for parent in self.parents.iter() {
            if PydanticModel::is_base_model(&parent) {
                inherits = true;
                break;
            }
        }
        inherits
    }
}

pub fn lex(source: String) -> Vec<PydanticModel> {
    let mut models = vec![];
    let mut i = 0;
    let lines = source
        .split("\n")
        .filter(|s| {
            let is_scoped = s.starts_with(&format!("{}{}", consts::INDENT, consts::INDENT));
            let trimmed = s.trim();
            !is_scoped
                && !trimmed.is_empty()
                && !trimmed.starts_with("import")
                && !trimmed.starts_with("from")
                && !trimmed.starts_with("&")
                && !trimmed.starts_with("\"\"\"")
                && !trimmed.starts_with("'''")
                && !trimmed.starts_with("#")
        })
        .collect::<Vec<_>>();

    // NOTE: Whitespace is significant in Python
    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with("class") {
            i += 1;
        } else {
            let mut class_name = line.split(' ').collect::<Vec<&str>>()[1];
            let mut fields: Vec<(String, String)> = vec![];
            let mut methods: Vec<PyMethod> = vec![];

            // Scan class names, including those of super classes.
            let parents: Vec<String>;
            match class_name.find('(') {
                Some(start) => {
                    let end = class_name.find(")").unwrap();
                    let parent_args = &class_name[start + 1..end];
                    parents = parent_args
                        .split(",")
                        .map(|p| p.trim().to_string())
                        .collect::<Vec<String>>();
                    class_name = &class_name[..start];
                }
                None => {
                    eprintln!("Detected invalid syntax in class: {}", class_name);
                    process::exit(-3);
                }
            };
            i += 1;

            // Scan fields.
            // In pydantic, fields are denoted as `field_name: type`.
            // println!("parsing fields");
            while lines[i].starts_with(consts::INDENT) && lines[i].contains(": ") {
                // Remove leading consts::INDENT.
                let curr_line = lines[i].trim();
                let field_and_type: Vec<&str> = curr_line.split(": ").collect();
                // TODO: ignore default arguments and fields
                fields.push((field_and_type[0].to_string(), field_and_type[1].to_string()));
                i += 1;
            }

            // Consume decorators and methods.
            while i < lines.len()
                && (lines[i].starts_with(&format!("{}def", consts::INDENT))
                    || lines[i].starts_with(&format!("{}@", consts::INDENT)))
            {
                if lines[i].starts_with(&format!("{}@", consts::INDENT)) {
                    let is_validator = lines[i].contains("validator");
                    i += 1;
                    if is_validator {
                        i += 1;
                        if i == lines.len() {
                            break;
                        }
                        continue;
                    }
                }
                if lines[i].starts_with(&format!("{}def", consts::INDENT)) {
                    methods.push(scan_method(&lines, &mut i));
                }
            }
            models.push(PydanticModel {
                class_name: class_name.to_string(),
                parents,
                fields,
                methods,
            })
        }
    }
    models
}

fn scan_method(lines: &Vec<&str>, curr_pos: &mut usize) -> PyMethod {
    // Remove consts::INDENT and trailing spaces.
    let method_signature = lines[*curr_pos].trim();
    if !method_signature.contains('(') {
        eprintln!(
            "Failed to find opening parenthesis in method signature {}",
            method_signature
        );
        process::exit(-7);
    }

    let method_name = method_signature.split('(').collect::<Vec<&str>>()[0];
    let method_name = method_name.replace("def ", "");

    let mut args: Vec<(String, Option<String>)> = vec![];
    let mut found_closing_parens = false;
    let mut returns: Option<String> = Option::None;

    while !found_closing_parens {
        let mut line = lines[*curr_pos].trim();
        if let Option::Some(pos) = line.find('(') {
            line = &line[pos + 1..];
        }

        // Parse return.
        found_closing_parens = line.contains(')');
        if found_closing_parens {
            let arg_and_return = line.split(')').map(|s| s.trim()).collect::<Vec<&str>>();
            line = arg_and_return[0];
            let returns_ = arg_and_return[1]
                .replace(":", "")
                .replace("->", "")
                .trim()
                .to_string();
            if !returns_.is_empty() {
                returns = Option::Some(returns_);
            }
        }

        // Parse arguments.
        let args_ = line.split(',').map(|a| a.trim());
        for a in args_ {
            if a == "\\" || a == "*" || a == "" {
                continue;
            }
            let field_and_type: Vec<&str> = a.split(':').collect();
            let arg = (
                field_and_type[0].to_string(),
                if field_and_type.len() == 1 {
                    None
                } else {
                    Some(field_and_type[1].trim().to_string())
                },
            );
            args.push(arg);
        }
        *curr_pos += 1;

        if *curr_pos == lines.len() && !found_closing_parens {
            eprintln!(
                "Failed to find closing parenthesis to parameters defined for method {}",
                method_name
            );
            process::exit(-6);
        }
    }
    PyMethod {
        name: method_name.clone(),
        args,
        returns,
        access: if method_name.starts_with('_') {
            PyMethodAccess::Private
        } else {
            PyMethodAccess::Public
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_same_line_method_scan() {
        let lines = vec![
            "    def my_method(self, value: typing.Any):",
            "        print(value)",
        ];
        let mut pos = 0;
        let _ = scan_method(&lines, &mut pos);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_listed_arg_method_scan() {
        let lines = vec![
            "    def my_method(",
            "        self,",
            "        value: typing.Any",
            "    ) -> list[str | tuple[str, str]]:",
            "        return ['hello world!']",
        ];
        let mut pos = 0;
        let _ = scan_method(&lines, &mut pos);
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_staggered_arg_method_scan() {
        let lines = vec![
            "    def my_method(self,",
            "        value: typing.Any) -> None:",
            "        print(value)",
        ];
        let mut pos = 0;
        let _ = scan_method(&lines, &mut pos);
        assert_eq!(pos, 2);
    }
}