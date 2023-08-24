use crate::consts;
use std::collections::HashSet;

trait UniqueVec {
    fn remove_dups(&mut self);
}
impl UniqueVec for Vec<PyClass> {
    fn remove_dups(&mut self) {
        let mut found = HashSet::new();
        self.retain(|cls| found.insert(cls.name.clone()));
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
    fn get_declr_name(&self) -> Result<String, ScanError>;
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

    fn get_declr_name(&self) -> Result<String, ScanError> {
        let term = '(';
        if !(self.is_method() || self.is_class()) || !self.contains(term) {
            return Err(ScanError(format!(
                "Attempted to parse invalid declaration {}",
                &self
            )));
        }

        let trimmed = self.replace(consts::INDENT, "");
        if let Some(name) = trimmed.split(' ').nth(1) {
            let end = name.find(term).unwrap();
            return Ok(name[..end].to_string());
        } else {
            return Err(ScanError(format!(
                "Failed to parse declaration '{}'. Invalid format.",
                &trimmed
            )));
        }
    }
}

fn split_string(string: String, token: char) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];
    let mut current = String::new();
    let mut iter = string.chars();
    let mut pos: usize = 0;
    loop {
        match iter.next() {
            Some(ch) => {
                if ch == token {
                    tokens.push(current.clone());
                    current.clear();
                } else if ch == '"' || ch == '\'' {
                    let pystr = scan_pystr(&string[pos..]);
                    for c in pystr.chars() {
                        current.push(c);
                        let _ = iter.next();
                    }
                } else {
                    current.push(ch);
                }
            }
            None => break,
        }
        pos += 1;
    }
    tokens.push(current.clone());
    tokens
        .iter()
        .map(|t| t.trim().to_string())
        .collect::<Vec<String>>()
}

fn scan_pystr(line: &str) -> String {
    let mut pystr = String::new();
    let mut delims: Vec<char> = vec![];
    for ch in line.chars() {
        pystr.push(ch);
        if ch == '\'' || ch == '"' {
            // If last quote matches current, we've closed that substr.
            if let Some(d) = delims.last() {
                if ch == *d {
                    delims.pop();
                }
                if delims.is_empty() {
                    break;
                }
            // If not, we're scanning a new substr.
            } else {
                delims.push(ch);
            }
        }
    }
    pystr
}

type PyType = String;
type PyValue = String;
type ClassName = String;

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

#[derive(Debug, Clone)]
pub struct PyParam {
    pub name: String,
    pub type_: Option<PyType>,
    pub default: Option<PyValue>,
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
    pub params: Vec<PyParam>,
    pub returns: Option<PyType>,
    pub access: PyMethodAccess,
}

impl PyMethod {
    pub fn is_dunder(&self) -> bool {
        self.name.starts_with("__") && self.name.ends_with("__")
    }
}

#[derive(Debug, Default, Clone)]
pub struct PyClass {
    pub name: ClassName,
    pub parents: Vec<ClassName>,
    pub props: Vec<PyParam>,
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

    // Split and filter out ignorable lines.
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

        // Module-level docstrings.
        if line.is_docstring() {
            skip_multiline_docstring(&lines, &mut i);
        // Ignore all module-level statements and expressions that aren't
        // class definitions.
        } else if !(line.is_class() && line.indent_count() == 0) {
            i += 1;
        // Ignore all other lines.
        } else {
            let mut class_name = line.split(' ').nth(1).unwrap();
            let mut props: Vec<PyParam> = vec![];
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

            // Scan class namespace.
            while i < lines.len() && lines[i].indent_count() > 0 {
                let line = lines[i];
                if line.indent_count() > 1 {
                    // Ignore statements and expressions scoped to, e.g.,
                    // methods.
                    while i < lines.len() && lines[i].indent_count() > 1 {
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
                // TODO: handle field access
                } else if line.contains(":") && !line.is_class() {
                    let field_and_type: Vec<&str> =
                        line.split([':', '=']).map(|s| s.trim()).collect();
                    props.push(PyParam {
                        name: field_and_type[0].to_string(),
                        type_: Some(field_and_type[1].to_string()),
                        default: None,
                    });
                    i += 1;
                } else if line.contains("=") {
                    let field_and_type: Vec<&str> = line.split('=').map(|s| s.trim()).collect();
                    props.push(PyParam {
                        name: field_and_type[0].to_string(),
                        type_: None,
                        default: Some(field_and_type[1].to_string()),
                    });
                    i += 1;
                } else if line.is_enum_variant() {
                    props.push(PyParam {
                        name: line.trim().to_string(),
                        type_: None,
                        default: None,
                    });
                    i += 1;
                } else {
                    println!("Skipping unscannable line {}", line);
                    i += 1;
                }
            }

            models.push(PyClass {
                name: class_name.to_string(),
                parents,
                props,
                methods,
            })
        }
    }
    models.remove_dups();
    Ok(models)
}

struct BoundToken {}
impl BoundToken {
    const PAREN_L: char = '(';
    const PAREN_R: char = ')';
    const BRACKET_L: char = '[';
    const BRACKET_R: char = ']';
    const BRACE_L: char = '{';
    const BRACE_R: char = '}';
    const L_TOKENS: [char; 3] = [
        BoundToken::PAREN_L,
        BoundToken::BRACKET_L,
        BoundToken::BRACE_L,
    ];

    fn get_right(left: &char) -> char {
        match *left {
            BoundToken::PAREN_L => BoundToken::PAREN_R,
            BoundToken::BRACKET_L => BoundToken::BRACKET_R,
            BoundToken::BRACE_L => BoundToken::BRACE_R,
            _ => panic!("Received unhandled boundary token {}", left),
        }
    }

    fn find_left(line: &str) -> Option<(char, usize)> {
        for token in BoundToken::L_TOKENS.iter() {
            if let Some(pos) = line.find(*token) {
                return Some((*token, pos));
            }
        }
        return None;
    }
}

// TODO: handle recursive cases.
fn scan_enclosed(
    lines: &Vec<&str>,
    line_pos: &mut usize,
    char_pos: Option<usize>,
    inclusive: bool,
) -> Result<String, ScanError> {
    let first = &lines[*line_pos][char_pos.unwrap_or_default()..];

    if let Some((left, l_pos)) = BoundToken::find_left(first) {
        let right = BoundToken::get_right(&left);
        let start = *line_pos;
        let mut inside: Vec<String> = vec![];

        loop {
            let line = if *line_pos == start {
                &first[l_pos + 1..]
            } else {
                lines[*line_pos]
            };

            if let Some((_, l_pos_)) = BoundToken::find_left(&line) {
                inside.push(scan_enclosed(lines, line_pos, Some(l_pos_), inclusive)?);
            } else if let Some(r_pos) = line.find(right) {
                inside.push(line[..r_pos].to_string());
                break;
            } else {
                inside.push(line.to_string());
            }

            *line_pos += 1;
            if *line_pos == lines.len() {
                return Err(ScanError(
                    "Failed to scan bounded lexeme. Right boundary (closing) token '{}' not found."
                        .to_string(),
                ));
            }
        }

        let mut joined = inside
            .iter()
            .map(|line| {
                let mut line_ = line.trim().to_string();
                if line_.ends_with(',') {
                    line_.push_str(" ");
                }
                line_
            })
            .collect::<Vec<String>>()
            .join("");
        if inclusive {
            joined = format!("{}{}{}", left, joined.trim(), right);
        }

        return Ok(joined);
    }

    return Err(ScanError(format!(
        "Left enclosing token not found in line '{}'",
        lines[*line_pos]
    )));
}

fn scan_method(lines: &Vec<&str>, curr_pos: &mut usize) -> Result<PyMethod, ScanError> {
    // TODO: remove below after testing.
    // Remove consts::INDENT and trailing spaces.
    let signature = lines[*curr_pos];
    let name = signature.get_declr_name()?;
    let mut params: Vec<PyParam> = vec![];
    let mut returns: Option<PyType> = Option::None;

    let param_str = scan_enclosed(lines, curr_pos, None, false)?;
    for param in split_string(param_str, ',') {
        let mut param_and_default = split_string(param, '=').into_iter();
        let name_and_type = param_and_default.next();
        let default = param_and_default.next();

        let mut s = split_string(name_and_type.unwrap(), ':').into_iter();
        let param_name = s.next();
        let type_ = s.next();

        params.push(PyParam {
            name: param_name.unwrap(),
            type_,
            default,
        });
    }

    // Closing parenthesis and terminating token (colon, :) are always
    // found on the same line.
    // Likewise for return annotations.
    let line = lines[*curr_pos];
    if let Some(p) = line.find(')') {
        let mut r = line[p + 1..]
            .replace(")", "")
            .replace("->", "")
            .replace(":", "")
            .trim()
            .to_string();
        if !r.is_empty() {
            if let Some(t) = r.find('[') {
                let t_concat = scan_enclosed(lines, curr_pos, None, true)?;
                r = r[..t].to_string();
                r.push_str(t_concat.as_str());
            }
            returns = Option::Some(r);
        }
    } else {
        panic!(
            "Reached invalid line {} after parameter parse. Expected closing parenthesis ')'",
            line
        )
    }

    *curr_pos += 1;

    Ok(PyMethod {
        name: name.to_string(),
        params,
        returns,
        access: if name.starts_with('_') {
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
            "        value: typing.Any = 'my, default'",
            "    ) -> list[str | tuple[str, str]]:",
            "        return ['hello world!']",
        ];
        let mut pos = 0;
        let m = scan_method(&lines, &mut pos);
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

    #[test]
    fn test_declaration_name() {
        let signature = "    def my_method(self,";
        let declaration = signature.get_declr_name().unwrap();
        assert_eq!(declaration, "my_method");
    }

    #[test]
    fn test_staggered_default_parse() {
        let lines = vec![
            "    (",
            "        value1,",
            "        value2,",
            "        value3,",
            "    )",
        ];
        let scanned = scan_enclosed(&lines, &mut 0, None, true);
        assert_eq!(scanned.unwrap(), "(value1, value2, value3,)");
    }

    #[test]
    fn test_type_parse() {
        let lines = vec![
            "    tuple[",
            "        int,",
            "        str,",
            "        int",
            "    ]",
        ];
        let scanned = scan_enclosed(&lines, &mut 0, None, true);
        let res = format!("tuple{}", scanned.unwrap().as_str());
        assert_eq!(res, "tuple[int, str, int]");
    }

    #[test]
    fn test_listed_return_parse() {
        let lines = vec![
            "    def my_method(",
            "        self,",
            "        value: typing.Any = 'my, default'",
            "    ) -> list[",
            "            str | tuple[str, str]",
            "        ]:",
            "        return ['hello world!']",
        ];
        let mut pos = 0;
        let m = scan_method(&lines, &mut pos).unwrap();
        dbg!(&m);
        assert_eq!(m.returns, Some("list[str | tuple[str, str]]".to_string()));
    }
}
