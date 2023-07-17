use std::{cell::RefCell, collections::HashMap, process, rc::Rc};

use crate::models::{Node, PydanticModel};

pub fn parse(models: Vec<PydanticModel>) -> Vec<Rc<Node>> {
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
        // dbg!("made node {}!", &node);
        for parent in model.parents.iter().map(|p| p.as_str()) {
            // dbg!("checking parent {}!", &parent);
            if node.is_root {
                // dbg!("found a root {}!", &parent);
                continue;
            }
            if !registry.contains_key(parent) && !PydanticModel::is_base_model(parent) {
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
        // dbg!("adding node to list");
        nodes[i] = node;
    }
    // dbg!("returning nodes!");
    nodes
        .into_iter()
        .filter(|n| n.is_root)
        .collect::<Vec<Rc<Node>>>()
}
