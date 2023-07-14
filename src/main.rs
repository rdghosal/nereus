use std::{
    borrow::Borrow, cell::RefCell, collections::HashMap, env, format, fs, io, path::Path, process,
    rc::Rc, str::FromStr,
};

const INDENT: &str = "    ";
const PYDANTIC_BASE_MODEL_REFS: [&str; 2] = ["pydantic.BaseModel", "BaseModel"];

#[derive(Debug, Default, Clone)]
struct PydanticModel {
    class_name: String,
    parents: Vec<String>,
    fields: Vec<(String, String)>,
}

impl PydanticModel {
    pub fn inherits_base_model(&self) -> bool {
        let mut inherits = false;
        for parent in self.parents.iter().map(|p| p.as_str()) {
            if is_base_model(parent) {
                inherits = true;
                break;
            }
        }
        inherits
    }
}

#[derive(Clone, Debug)]
struct Node {
    model: PydanticModel,
    children: RefCell<Vec<Rc<Node>>>,
    is_root: bool,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            model: Default::default(),
            children: RefCell::new(vec![]),
            is_root: false,
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

fn parse(models: Vec<PydanticModel>) -> Vec<Rc<Node>> {
    let mut registry: HashMap<&str, usize> = HashMap::new();
    let default_node: Node = Default::default();
    let mut nodes: Vec<Rc<Node>> = vec![Rc::new(default_node); models.len()];

    // Populate registry.
    for (i, model) in models.iter().enumerate() {
        registry.insert(&model.class_name, i);
    }

    // Create nodes, identifying `roots`, whose super class is `pydantic.BaseModel`.
    for (i, model) in models.iter().enumerate() {
        let node = Rc::new(Node {
            model: model.clone(),
            children: RefCell::new(vec![]),
            is_root: model.inherits_base_model(),
        });
        dbg!("made node {}!", &node);
        for parent in model.parents.iter().map(|p| p.as_str()) {
            dbg!("checking parent {}!", &parent);
            if node.is_root {
                dbg!("found a root {}!", &parent);
                continue;
            }
            if !registry.contains_key(parent) && !is_base_model(parent) {
                eprintln!("Found reference to undefined super class {}", parent);
                process::exit(-4);
            }
            let index = registry.get(parent).unwrap();
            let parent_model = &models[*index];

            // Check whether the node in `nodes` is a default.
            if nodes[*index].model.class_name == parent_model.class_name {
                let parent_node = &mut nodes[*index];
                parent_node.children.borrow_mut().push(Rc::clone(&node));
            } else {
                let parent_node = Rc::new(Node {
                    model: parent_model.clone(),
                    children: RefCell::new(vec![Rc::clone(&node)]),
                    is_root: model.inherits_base_model(),
                });
                nodes[*index] = parent_node;
            }
        }
        dbg!("adding node to list");
        nodes[i] = node;
    }
    dbg!("returning nodes!");
    nodes
        .into_iter()
        .filter(|n| n.is_root)
        .collect::<Vec<Rc<Node>>>()
}

fn is_base_model(class_name: &str) -> bool {
    PYDANTIC_BASE_MODEL_REFS.contains(&class_name)
}

fn read_files(dir: &Path, source: &mut String) -> Result<(), io::Error> {
    dbg!("using path {}", &dir);
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

fn make_mermaid_cls(node: Rc<Node>, mut output: String) -> String {
    let inherits = " <|-- ";
    for child in node.children.borrow().iter() {
        output.push_str(
            format!(
                "\r\n{}{}{}{}",
                INDENT, &node.model.class_name, inherits, &child.model.class_name
            )
            .as_str(),
        );
    }
    let class_name = format!("\r\n{}class {}{{", INDENT, node.model.class_name);
    output.push_str(class_name.as_str());
    for field in &node.model.fields {
        output.push_str(format!("\r\n{}{}+{} {}", INDENT, INDENT, field.0, field.1).as_str());
    }
    output.push_str(format!("\r\n{}}}", INDENT).as_str());
    for child in node.children.borrow().iter() {
        output = make_mermaid_cls(child.clone(), output);
    }
    output
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
    dbg!("{?#}", &models);
    let nodes = parse(models);
    dbg!("{?#}", &nodes);
    let mut class_diagram = String::from_str("classDiagram").expect("oops");
    // dbg!("{}", make_mermaid_cls(nodes[0].clone(), class_diagram));
    for node in nodes {
        class_diagram = make_mermaid_cls(node, class_diagram);
    }
    fs::write("test.mermaid", class_diagram);
}
