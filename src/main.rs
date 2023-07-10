use std::{collections::HashMap, env, format, fs, io, path::Path, process};

const INDENT: &str = "    ";
const PYDANTIC_BASE_MODEL_REFS: [&str; 2] = ["pydantic.BaseModel", "BaseModel"];

#[derive(Debug, Default, Clone)]
struct PydanticModel {
    class_name: String,
    parents: Vec<String>,
    fields: Vec<(String, String)>,
}

#[derive(Clone)]
struct Node {
    model: PydanticModel,
    children: Vec<Box<Node>>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            model: Default::default(),
            children: vec![],
        }
    }
}

fn lex(source: String) -> Vec<PydanticModel> {
    let mut models = vec![];
    let mut i = 0;
    let lines = source.split("\n").collect::<Vec<_>>();

    // NOTE: Whitespace is significant in Python
    while i < lines.len() {
        let line = lines[i];
        println!("lexing... {}", line);
        if !line.starts_with("class") {
            i += 1;
        } else {
            // Scan class names, including those of super classes.
            let mut class_name = line.split(' ').collect::<Vec<&str>>()[1];
            let parents: Vec<String>;
            println!("scanning class name {}", class_name);
            match class_name.find('(') {
                Some(start) => {
                    let end = class_name.find(")").unwrap();
                    let parent_args = &class_name[start + 1..end];
                    parents = parent_args
                        .split(", ")
                        .map(|p| p.to_string())
                        .collect::<Vec<String>>();
                    class_name = &class_name[..start];
                }
                None => {
                    eprintln!("Detected invalid syntax in class: {}", class_name);
                    process::exit(-3);
                }
            };
            i += 1;

            // Consume decorators and methods.
            if lines[i].starts_with(&format!("@")) {
                println!("skipping decorators");
                i += 1;
                continue;
            } else if lines[i].starts_with(&format!("{}def", INDENT)) {
                println!("skipping method");
                while lines[i].starts_with(&format!("{}{}", INDENT, INDENT)) {
                    i += 1;
                }
            }

            // Scan fields.
            // In pydantic, fields are denoted as `field_name: type`.
            println!("parsing fields");
            let mut fields: Vec<(String, String)> = vec![];
            while lines[i].starts_with(INDENT) && lines[i].contains(": ") {
                // Remove leading indent.
                let curr_line = lines[i].trim();
                let field_and_type: Vec<&str> = curr_line.split(": ").collect();
                fields.push((field_and_type[0].to_string(), field_and_type[1].to_string()));
                i += 1;
            }

            println!("adding model");
            models.push(PydanticModel {
                class_name: class_name.to_string(),
                parents,
                fields,
            })
        }
    }
    models
}

fn parse(models: Vec<PydanticModel>) -> Node {
    let registry: HashMap<&str, usize> = HashMap::new();
    let mut nodes: Vec<Node> = vec![Default::default(); models.len()];
    for (i, model) in models.iter().enumerate() {
        registry.insert(&model.class_name, i);
    }
    for (i, model) in models.iter().enumerate() {
        for parent in model.parents {
            if !registry.contains_key(parent.as_str()) && !is_base_model(&parent) {
                eprintln!("Found reference to undefined super class {}", parent);
                process::exit(-4);
            }
            let index = registry.get(parent.as_str()).unwrap();
            let parent_model = models[*index];
            if nodes[*index].model.class_name == parent_model.class_name {
                let mut parent_node = &mut nodes[*index];
                let child_node = Node {
                    model: *model,
                    children: vec![],
                };
                parent_node.children.push(Box::new(child_node));
                nodes[i] = child_node;
            } else {
                let child_node = Node {
                    model: *model,
                    children: vec![],
                };
                let parent_node = Node {
                    model: *model,
                    children: vec![Box::new(child_node)],
                };
                nodes[*index] = parent_node;
                nodes[i] = child_node;
            }
        }
    }
    Node {}
}

fn is_base_model(class_name: &str) -> bool {
    PYDANTIC_BASE_MODEL_REFS.contains(&class_name)
}

fn read_files(dir: &Path, source: &mut String) -> Result<(), io::Error> {
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: percy <path to .py files>");
        process::exit(-1);
    }
    let mut source = String::new();
    read_files(Path::new(&args[1]), &mut source).expect("oops");
    let models = lex(source);
    dbg!("{?#}", models);
}
