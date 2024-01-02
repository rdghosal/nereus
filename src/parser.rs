use crate::consts::*;
use crate::models::*;

#[derive(Debug)]
pub struct ParseError(String);
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ParseError {}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn test_same_line_method_scan() {
//         let lines = vec![
//             "    def my_method(self, value: typing.Any):",
//             "        print(value)",
//         ];
//         let mut pos = 0;
//         let _ = scan_method(&lines, &mut pos);
//         assert_eq!(pos, 1);
//     }

//     #[test]
//     fn test_listed_arg_method_scan() {
//         let lines = vec![
//             "    def my_method(",
//             "        self,",
//             "        value: typing.Any = 'my, default'",
//             "    ) -> list[str | tuple[str, str]]:",
//             "        return ['hello world!']",
//         ];
//         let mut pos = 0;
//         let m = scan_method(&lines, &mut pos);
//         dbg!(m.unwrap());
//         assert_eq!(pos, 4);
//     }

//     #[test]
//     fn test_staggered_arg_method_scan() {
//         let lines = vec![
//             "    def my_method(self,",
//             "        value: typing.Any) -> None:",
//             "        print(value)",
//         ];
//         let mut pos = 0;
//         let _ = scan_method(&lines, &mut pos);
//         assert_eq!(pos, 2);
//     }

//     #[test]
//     fn test_declaration_name() {
//         let signature = "    def my_method(self,";
//         let declaration = signature.get_declr_name().unwrap();
//         assert_eq!(declaration, "my_method");
//     }
// }

pub fn parse(source: String) -> Result<Vec<PyClass>, ParseError> {
    let mut models = vec![];
    let mut i = 0;
    let lines = preprocess(&source);
    while i < lines.len() {
        // Ignore all module-level statements and expressions that aren't
        // class definitions, including multi-line docstrings.
        skip_multiline_docstring(&lines, &mut i);
        loop {
            if lines[i].is_class_def() && lines[i].indent_count() == 0 {
                break;
            }
            i += 1;
        }
        models.push(parse_class(&lines, &mut i)?);
    }
    models.remove_dups();
    Ok(models)
}

fn preprocess(source: &str) -> Vec<&str> {
    // NOTE: Whitespace is significant in Python
    // Split and filter out ignorable lines.
    source
        .lines()
        .filter(|s| {
            !s.is_empty()
                && !s.is_import()
                && !s.is_comment()
                && !s.is_full_docstring()
                && !s.is_decorator()
        })
        .collect::<Vec<_>>()
}

fn parse_class(lines: &Vec<&str>, curr_pos: &mut usize) -> Result<PyClass, ParseError> {
    // Parse class names, including those of super classes.
    let (name, parents) = parse_class_def(lines, curr_pos)?;

    // Parse class namespace, defined by an indent level of >0.
    let mut fields: Vec<PyParam> = vec![];
    let mut methods: Vec<PyMethod> = vec![];
    while lines[*curr_pos].indent_count() > 0 {
        // Ignore statements and expressions scoped to, e.g., methods.
        loop {
            if *curr_pos < lines.len() && lines[*curr_pos].indent_count() > 1 {
                *curr_pos += 1;
            } else {
                break;
            }
        }
        if *curr_pos >= lines.len() {
            break;
        }

        skip_multiline_docstring(lines, curr_pos);
        skip_placeholder(lines, curr_pos);
        let line = lines[*curr_pos];

        // Found another class definition.
        // Preserve position for next iteration, but abort the current silently.
        if line.is_class_def() && line.indent_count() == 0 {
            break;
        // Skip ellipsis and `pass`.
        } else if line.is_method_def() {
            methods.push(parse_class_method(lines, curr_pos)?);
        // TODO: handle field access
        // Found field.
        } else if line.is_class_field() {
            fields.push(parse_class_field(lines, curr_pos)?);
        // Skip unparsable line.
        } else {
            *curr_pos += 1;
        }
    }
    Ok(PyClass {
        name,
        parents,
        fields,
        methods,
    })
}

fn parse_class_def(
    lines: &Vec<&str>,
    curr_pos: &mut usize,
) -> Result<(PyClsName, Vec<PyClsName>), ParseError> {
    let line = lines[*curr_pos];
    let cls_def = line.split(' ').nth(1).unwrap();
    match cls_def.find('(') {
        Some(start) => {
            let end = cls_def.find(')').unwrap();
            let parent_args = &cls_def[start + 1..end];
            let parents = parent_args
                .split(',')
                .map(|p| p.trim().to_owned())
                .collect::<Vec<String>>();
            *curr_pos += 1;
            Ok((
                cls_def[..start].split(' ').nth(1).unwrap().to_owned(),
                parents,
            ))
        }
        None => {
            if let Some(end) = cls_def.find(':') {
                *curr_pos += 1;
                Ok((cls_def[..end].split(' ').nth(1).unwrap().to_owned(), vec![]))
            } else {
                Err(ParseError(format!(
                    "Failed to identify class name terminator (:) in class {}",
                    cls_def
                )))
            }
        }
    }
}

fn parse_class_method(lines: &Vec<&str>, curr_pos: &mut usize) -> Result<PyMethod, ParseError> {
    // Parse method name.
    let name = get_method_name(lines, curr_pos)?;
    let access = if name.starts_with('_') {
        PyMethodAccess::Private
    } else {
        PyMethodAccess::Public
    };
    let mut args = vec![];
    let mut returns = Option::None;

    // Parse arguments.
    let arg_str = parse_bounded('(', lines, curr_pos, false)?;
    for arg in split_string(&arg_str, ',') {
        let arg_and_default = split_string(&arg, '=');
        let name_and_type = split_string(&arg_and_default[0], ':');
        args.push(PyParam {
            name: name_and_type[0].to_owned(),
            dtype: Some(name_and_type.get(1).unwrap().to_owned()),
            default: Some(arg_and_default.get(1).unwrap().to_owned()),
        });
    }

    // Parse return type.
    // NOTE: Closing parenthesis and terminating token (colon, :) are always
    // found on the same line. Likewise for return annotations.
    let line = lines[*curr_pos];
    if let Some(p) = lines[*curr_pos].find(')') {
        let r = line[p + 1..]
            .replace(")", "")
            .replace("->", "")
            .replace(":", "")
            .trim()
            .to_owned();
        if !r.is_empty() {
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
        name,
        access,
        args,
        returns,
    })
}

fn get_method_name(lines: &Vec<&str>, curr_pos: &mut usize) -> Result<String, ParseError> {
    let tok = '(';
    let line = lines[*curr_pos];
    if !line.is_method_def() || !line.contains(tok) {
        return Err(ParseError(format!(
            "Attempted to parse invalid method definition {}",
            &line
        )));
    }
    let name = line.trim().split(' ').nth(1).unwrap();
    let end = name.find(tok).unwrap();
    Ok(name[..end].to_owned())
}

fn parse_class_field(lines: &Vec<&str>, curr_pos: &mut usize) -> Result<PyParam, ParseError> {
    let line = lines[*curr_pos];
    *curr_pos += 1;
    let substrs = line
        .split([':', '='])
        .map(|s| s.trim())
        .collect::<Vec<&str>>();
    match substrs.len() {
        3.. => Ok(PyParam {
            name: substrs[0].to_owned(),
            dtype: Some(substrs[1].to_owned()),
            default: Some(substrs[2].to_owned()),
        }),
        2 => {
            if line.contains('=') {
                Ok(PyParam {
                    name: substrs[0].to_owned(),
                    dtype: None,
                    default: Some(substrs[1].to_owned()),
                })
            } else {
                Ok(PyParam {
                    name: substrs[0].to_owned(),
                    dtype: Some(substrs[1].to_owned()),
                    default: None,
                })
            }
        }
        1 => Ok(PyParam {
            name: substrs[0].to_owned(),
            dtype: None,
            default: None,
        }),
        _ => Err(ParseError(format!("Failed to parse class field {}", line))),
    }
}

fn parse_bounded(
    left: char,
    lines: &Vec<&str>,
    curr_pos: &mut usize,
    inclusive: bool,
) -> Result<String, ParseError> {
    let right = match left {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        _ => panic!("Received unhandled boundary token {}", left),
    };
    let l_pos = lines[*curr_pos].find(left);
    if l_pos.is_none() {
        return Err(ParseError(format!(
            "Left boundary token '{}' not found in line '{}'",
            left, lines[*curr_pos]
        )));
    }

    let start = *curr_pos;
    let mut inside = vec![];
    loop {
        let line = if *curr_pos == start {
            &lines[*curr_pos][l_pos.unwrap() + 1..]
        } else {
            lines[*curr_pos]
        };

        if let Some(r_pos) = line.find(right) {
            inside.push(&line[..r_pos]);
            break;
        } else {
            inside.push(line);
        }

        *curr_pos += 1;
        if *curr_pos == lines.len() {
            return Err(ParseError(
                "Failed to scan bounded lexeme. Right boundary (closing) token '{}' not found."
                    .to_owned(),
            ));
        }
    }

    let mut joined = inside
        .iter()
        .map(|line| {
            let mut line_ = line.trim().to_owned();
            if line_.ends_with(',') {
                line_.push_str(" ");
            }
            line_
        })
        .collect::<Vec<String>>()
        .join("");

    if inclusive {
        joined = format!("{}{}{}", right, joined, left);
    }
    Ok(joined.to_owned())
}

fn split_string(line: &str, delim: char) -> Vec<String> {
    let mut result = vec![];
    let mut buffer = String::new();
    let mut iter = line.chars();
    let mut pos = 0;
    loop {
        match iter.next() {
            Some(ch) => match ch {
                '"' | '\'' => {
                    let pystr = parse_pystr(&line[pos..]);
                    for c in pystr.chars() {
                        buffer.push(c);
                        let _ = iter.next();
                    }
                }
                _ if ch == delim => {
                    result.push(buffer.clone());
                    buffer.clear();
                }
                _ => buffer.push(ch),
            },
            None => break,
        }
        pos += 1;
    }
    result.push(buffer.clone());
    result
        .iter()
        .map(|t| t.trim().to_owned())
        .collect::<Vec<String>>()
}

fn parse_pystr(line: &str) -> String {
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

fn skip_multiline_docstring(lines: &Vec<&str>, curr_pos: &mut usize) {
    let line = lines[*curr_pos];
    if !line.is_docstring() {
        return;
    }
    *curr_pos += 1;
    loop {
        let trimmed = line.trim();
        if trimmed.ends_with(DocstringMarker::SINGLE) || trimmed.ends_with(DocstringMarker::DOUBLE)
        {
            *curr_pos += 1;
        } else {
            break;
        }
    }
    *curr_pos += 1;
}

fn skip_placeholder(lines: &Vec<&str>, curr_pos: &mut usize) {
    if lines[*curr_pos].is_placeholder() {
        *curr_pos += 1;
    }
}
