use crate::scanner::PydanticModel;
use std::{cell::RefCell, collections::HashMap, process, rc::Rc};

#[derive(Clone, Debug)]
pub struct Node {
    pub model: PydanticModel,
    pub children: RefCell<Vec<Rc<Node>>>,
    pub is_root: bool,
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

pub fn parse(models: Vec<PydanticModel>) -> Vec<Rc<Node>> {
    let mut registry: HashMap<&String, usize> = HashMap::new();

    let default_node: Node = Default::default();
    let mut nodes: Vec<Rc<Node>> = vec![Rc::new(default_node); models.len()];
    let roots: Vec<Rc<Node>>;

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
        for parent in model.parents.iter() {
            if node.is_root {
                continue;
            }
            if !registry.contains_key(parent) && !PydanticModel::is_base_model(&parent) {
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

        nodes[i] = node;
    }

    roots = nodes
        .into_iter()
        .filter(|n| n.is_root)
        .collect::<Vec<Rc<Node>>>();

    if roots.is_empty() {
        eprintln!("Failed to identify child classes of `pydantic.BaseModel`");
        process::exit(-5)
    }
    roots
}
