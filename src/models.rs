use std::{cell::RefCell, collections::HashMap, env, format, fs, io, path::Path, process, rc::Rc};
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
    pub fn is_base_model(class_name: &str) -> bool {
        PYDANTIC_BASE_MODEL_REFS.contains(&class_name)
    }

    pub fn inherits_base_model(&self) -> bool {
        let mut inherits = false;
        for parent in self.parents.iter().map(|p| p.as_str()) {
            if PydanticModel::is_base_model(parent) {
                inherits = true;
                break;
            }
        }
        inherits
    }
}

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
