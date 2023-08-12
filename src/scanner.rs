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

trait PyLine {
    fn is_docstring(&self) -> bool;
    fn is_full_docstring(&self) -> bool;
    fn is_placeholder(&self) -> bool;
    fn is_decorator(&self) -> bool;
    fn is_method(&self) -> bool;
    fn is_class(&self) -> bool;
    fn is_import(&self) -> bool;
    fn is_comment(&self) -> bool;
    fn indent_count(&self) -> usize;
    fn starts_with_token(&self, token: &str) -> bool;
    fn is_enum_variant(&self) -> bool;
}

impl PyLine for &str {
    fn is_docstring(&self) -> bool {
        let trimmed = self.trim();
        trimmed.starts_with(DocstringMarker::SINGLE) || trimmed.starts_with(DocstringMarker::DOUBLE)
    }

    fn is_placeholder(&self) -> bool {
        let trimmed = self.trim();
        trimmed.starts_with(Placeholder::PASS) || trimmed.starts_with(Placeholder::ELLIPSIS)
    }

    fn is_decorator(&self) -> bool {
        self.trim().starts_with("@")
    }

    fn starts_with_token(&self, token: &str) -> bool {
        let parsed = self.trim().split(' ').nth(0);
        parsed.is_some() && (parsed.unwrap() == token)
    }

    fn is_method(&self) -> bool {
        self.starts_with_token("def")
    }

    fn is_class(&self) -> bool {
        self.starts_with_token("class")
    }

    fn is_import(&self) -> bool {
        let trimmed = self.trim();
        trimmed.starts_with("import") || trimmed.starts_with("from")
    }

    fn is_full_docstring(&self) -> bool {
        let trimmed = self.trim();
        (trimmed.starts_with(DocstringMarker::SINGLE)
            && trimmed.ends_with(DocstringMarker::SINGLE)
            && trimmed.len() >= 6)
            || (trimmed.starts_with(DocstringMarker::DOUBLE)
                && trimmed.ends_with(DocstringMarker::DOUBLE)
                && trimmed.len() >= 6)
    }

    fn is_comment(&self) -> bool {
        self.trim().starts_with("#")
    }

    fn is_enum_variant(&self) -> bool {
        self.trim().chars().all(char::is_alphanumeric)
    }

    fn indent_count(&self) -> usize {
        self.split(consts::INDENT).filter(|s| s.is_empty()).count()
    }
}

struct DocstringMarker;
impl DocstringMarker {
    const SINGLE: &str = "'''";
    const DOUBLE: &str = "\"\"\"";
}

struct Placeholder;
impl Placeholder {
    const PASS: &str = "pass";
    const ELLIPSIS: &str = "...";
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
            !s.is_empty()
                && !s.is_import()
                && !s.is_comment()
                && !s.is_full_docstring()
                && !s.is_decorator()
        })
        .collect::<Vec<_>>();

    // NOTE: Whitespace is significant in Python
    while i < lines.len() {
        let line = lines[i];
        if line.is_docstring() {
            skip_multiline_docstring(&lines, &mut i);
        } else if !(line.is_class() && line.indent_count() == 0) {
            i += 1;
        } else {
            let mut class_name = line.split(' ').nth(1).unwrap();
            let mut fields: Vec<(String, Option<String>, Option<String>)> = vec![];
            let mut methods: Vec<PyMethod> = vec![];

            // Scan class names, including those of super classes.
            let mut parents: Vec<String> = vec![];
            match line.find('(') {
                Some(start) => {
                    let end = line.find(")").unwrap();
                    let parent_args = &line[start + 1..end];
                    parents = parent_args
                        .split(",")
                        .map(|p| p.trim().to_string())
                        .collect::<Vec<String>>();
                    class_name = &line[..start].split(' ').nth(1).unwrap();
                }
                None => {
                    if let Some(term) = line.find(':') {
                        class_name = &line[..term].split(' ').nth(1).unwrap();
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
            while i < lines.len() && lines[i].indent_count() > 0 {
                let line = lines[i];
                if line.indent_count() > 1 {
                    while lines[i].indent_count() > 1 && i < lines.len() {
                        i += 1;
                    }
                } else if line.is_class() && line.indent_count() == 0 {
                    break;
                } else if line.is_method() {
                    methods.push(scan_method(&lines, &mut i)?);
                } else if line.is_docstring() {
                    skip_multiline_docstring(&lines, &mut i);
                } else if line.is_placeholder() {
                    i += 1;
                } else if line.contains(":") && !line.is_class() {
                    let field_and_type: Vec<&str> =
                        line.split([':', '=']).map(|s| s.trim()).collect();
                    fields.push((
                        field_and_type[0].to_string(),
                        Some(field_and_type[1].to_string()),
                        None,
                    ));
                    i += 1;
                } else if line.contains("=") {
                    let field_and_type: Vec<&str> = line.split('=').map(|s| s.trim()).collect();
                    fields.push((
                        field_and_type[0].to_string(),
                        None,
                        Some(field_and_type[1].to_string()),
                    ));
                    i += 1;
                } else if line.is_enum_variant() {
                    fields.push((line.trim().to_string(), None, None));
                    i += 1;
                } else {
                    println!("Skipping unscannable line {}", line);
                    i += 1;
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

fn skip_multiline_docstring(lines: &Vec<&str>, curr_pos: &mut usize) {
    *curr_pos += 1;
    while !(lines[*curr_pos].trim().ends_with(DocstringMarker::SINGLE)
        || lines[*curr_pos].trim().ends_with(DocstringMarker::DOUBLE))
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
