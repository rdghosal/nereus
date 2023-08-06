use crate::scanner::PydanticModel;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct Node {
    pub model: PydanticModel,
    pub children: RefCell<Vec<Rc<Node>>>,
    pub is_root: bool,
}

#[derive(Debug)]
pub struct ParseError(String);
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ParseError {}

trait UniqueVec {
    fn remove_dups(&mut self);
}
impl UniqueVec for Vec<PydanticModel> {
    fn remove_dups(&mut self) {
        let mut found = HashSet::new();
        self.retain(|cls| found.insert(cls.class_name.clone()));
    }
}

pub fn parse(models: &mut Vec<PydanticModel>) -> Result<Vec<Rc<Node>>, ParseError> {
    let mut registry: HashMap<&String, usize> = HashMap::new();
    models.remove_dups();

    let mut nodes: Vec<Option<Rc<Node>>> = vec![None; models.len()];
    let roots: Vec<Rc<Node>>;

    // Populate registry.
    for (i, model) in models.iter().enumerate() {
        registry.insert(&model.class_name, i);
    }

    // Create nodes, identifying `roots`, whose super class is `pydantic.BaseModel`.
    for (i, model) in models.iter().enumerate() {
        let node: Rc<Node>;
        match &nodes[i] {
            Some(n) => node = n.clone(),
            None => {
                node = Rc::new(Node {
                    model: model.clone(),
                    children: RefCell::new(vec![]),
                    is_root: model.is_orphan(),
                })
            }
        }

        for parent in model.parents.iter() {
            if node.is_root {
                continue;
            }

            if let Some(index) = registry.get(parent) {
                let parent_model = &models[*index];
                match &nodes[*index] {
                    Some(p) => p.children.borrow_mut().push(Rc::clone(&node)),
                    None => {
                        let parent_node = Rc::new(Node {
                            model: parent_model.clone(),
                            children: RefCell::new(vec![Rc::clone(&node)]),
                            is_root: parent_model.is_orphan(),
                        });
                        nodes[*index] = Some(parent_node);
                    }
                };
            }
        }
        nodes[i] = Some(node);
    }

    roots = nodes
        .into_iter()
        .map(|n| n.unwrap())
        .collect::<Vec<Rc<Node>>>();

    if roots.is_empty() {
        return Err(ParseError(
            "Failed to identify child classes of `pydantic.BaseModel`".to_string(),
        ));
    }
    Ok(roots)
}
