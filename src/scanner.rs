use crate::consts;
use std::collections::HashSet;

trait UniqueVec {
    fn remove_dups(&mut self);
}
impl UniqueVec for Vec<PyClass> {
    fn remove_dups(&mut self) {
        let mut found = HashSet::new();
        self.retain(|cls| found.insert(cls.class_name.clone()));
    }
}

struct DocstringMarker;
impl DocstringMarker {
    const SINGLE: &str = "'''";
    const DOUBLE: &str = "\"\"\"";

    fn is_docstring(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with(DocstringMarker::SINGLE) || trimmed.starts_with(DocstringMarker::DOUBLE)
    }
}

struct Placeholder;
impl Placeholder {
    const PASS: &str = "pass";
    const ELLIPSIS: &str = "...";

    fn is_placeholder(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with(Placeholder::PASS) || trimmed.starts_with(Placeholder::ELLIPSIS)
    }
}

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

impl PyMethod {
    pub fn is_dunder(&self) -> bool {
        self.name.starts_with("__") && self.name.ends_with("__")
    }
}

#[derive(Debug, Default, Clone)]
pub struct PyClass {
    pub class_name: String,
    pub parents: Vec<String>,
    pub fields: Vec<(String, Option<String>, Option<String>)>,
    pub methods: Vec<PyMethod>,
}

#[derive(Debug)]
pub struct ScanError(String);
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ScanError {}

pub fn lex(source: String) -> Result<Vec<PyClass>, ScanError> {
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
                && !trimmed.starts_with("#")
                && !(trimmed.starts_with(DocstringMarker::SINGLE)
                    && trimmed.ends_with(DocstringMarker::SINGLE)
                    && trimmed.len() >= 6)
                && !(trimmed.starts_with(DocstringMarker::DOUBLE)
                    && trimmed.ends_with(DocstringMarker::DOUBLE)
                    && trimmed.len() >= 6)
        })
        .collect::<Vec<_>>();

    // NOTE: Whitespace is significant in Python
    while i < lines.len() {
        let line = lines[i];
        if DocstringMarker::is_docstring(lines[i]) {
            skip_multiline_docstring(&lines, &mut i);
        } else if !line.starts_with("class") {
            i += 1;
        } else {
            let mut class_name = line.split(' ').collect::<Vec<&str>>()[1];
            let mut fields: Vec<(String, Option<String>, Option<String>)> = vec![];
            let mut methods: Vec<PyMethod> = vec![];

            // Scan class names, including those of super classes.
            let mut parents: Vec<String> = vec![];
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
                    if let Some(term) = class_name.find(':') {
                        class_name = &class_name[..term];
                    } else {
                        return Err(ScanError(format!(
                            "Failed to identify class name terminator (:) in class {}",
                            class_name
                        )));
                    }
                }
            }

            i += 1;

            // Scan fields.
            // In pydantic, fields are denoted as `field_name: type`.
            while i < lines.len() && lines[i].starts_with(consts::INDENT) {
                // Consume decorators and methods.
                if is_decorator(lines[i]) {
                    while !is_method(lines[i]) {
                        i += 1;
                        if i > lines.len() {
                            return Err(ScanError(
                                "Failed to scan decorator. corresponding method not found."
                                    .to_string(),
                            ));
                        }
                    }
                    // // Skip scan of validator method.
                    // if is_validator {
                    //     i += 1;
                    // }
                } else if is_method(lines[i]) {
                    methods.push(scan_method(&lines, &mut i)?);
                } else if lines[i].contains(":") {
                    let field_and_type: Vec<&str> =
                        lines[i].split([':', '=']).map(|s| s.trim()).collect();
                    fields.push((
                        field_and_type[0].to_string(),
                        Some(field_and_type[1].to_string()),
                        None,
                    ));
                    i += 1;
                } else if lines[i].contains("=") {
                    let field_and_type: Vec<&str> = lines[i].split('=').map(|s| s.trim()).collect();
                    fields.push((
                        field_and_type[0].to_string(),
                        None,
                        Some(field_and_type[1].to_string()),
                    ));
                    i += 1;
                } else if DocstringMarker::is_docstring(lines[i]) {
                    skip_multiline_docstring(&lines, &mut i);
                } else if Placeholder::is_placeholder(lines[i]) {
                    i += 1;
                } else if lines[i].trim().chars().all(char::is_alphanumeric) {
                    fields.push((lines[i].trim().to_string(), None, None));
                    i += 1;
                } else {
                    println!("Skipping unscannable line {}", lines[i]);
                    i += 1;
                    // return Err(ScanError(
                    //     format!("Failed to complete scanning of Python source. Unexpected token found in line '{}'.", &lines[i])
                    // ));
                }
            }

            models.push(PyClass {
                class_name: class_name.to_string(),
                parents,
                fields,
                methods,
            })
        }
    }
    models.remove_dups();
    Ok(models)
}

fn scan_method(lines: &Vec<&str>, curr_pos: &mut usize) -> Result<PyMethod, ScanError> {
    // Remove consts::INDENT and trailing spaces.
    let method_signature = lines[*curr_pos].trim();
    if !method_signature.contains('(') {
        return Err(ScanError(format!(
            "Failed to find opening parenthesis in method signature {}",
            method_signature
        )));
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
            return Err(ScanError(format!(
                "Failed to find closing parenthesis to parameters defined for method {}",
                method_name
            )));
        }
    }
    Ok(PyMethod {
        name: method_name.clone(),
        args,
        returns,
        access: if method_name.starts_with('_') {
            PyMethodAccess::Private
        } else {
            PyMethodAccess::Public
        },
    })
}

fn is_decorator(line: &str) -> bool {
    line.starts_with(&format!("{}@", consts::INDENT))
}

fn is_method(line: &str) -> bool {
    line.starts_with(&format!("{}def", consts::INDENT))
}

// fn is_validator(line: &str) -> bool {
//     line.contains("validator")
// }

fn skip_multiline_docstring(lines: &Vec<&str>, curr_pos: &mut usize) {
    *curr_pos += 1;
    while !(lines[*curr_pos].contains(DocstringMarker::SINGLE)
        || lines[*curr_pos].contains(DocstringMarker::DOUBLE))
    {
        *curr_pos += 1;
    }
    *curr_pos += 1;
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
