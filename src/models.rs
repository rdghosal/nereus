use crate::consts;
use std::collections::HashSet;

pub type PyClsName = String;
pub type PyType = String;
type PyValue = String;

#[derive(Debug, Clone)]
pub struct PyParam {
    pub name: String,
    pub dtype: Option<PyType>,
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
    pub args: Vec<PyParam>,
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
    pub name: PyClsName,
    pub parents: Vec<PyClsName>,
    pub fields: Vec<PyParam>,
    pub methods: Vec<PyMethod>,
}

pub trait UniqueVec {
    fn remove_dups(&mut self);
}
impl UniqueVec for Vec<PyClass> {
    fn remove_dups(&mut self) {
        let mut found = HashSet::new();
        self.retain(|cls| found.insert(cls.name.clone()));
    }
}

pub trait PyLine {
    fn is_docstring(&self) -> bool;
    fn is_full_docstring(&self) -> bool;
    fn is_placeholder(&self) -> bool;
    fn is_decorator(&self) -> bool;
    fn is_method_def(&self) -> bool;
    fn is_class_def(&self) -> bool;
    fn is_class_field(&self) -> bool;
    fn is_import(&self) -> bool;
    fn is_comment(&self) -> bool;
    fn indent_count(&self) -> usize;
    fn starts_with_token(&self, token: &str) -> bool;
    fn is_enum_variant(&self) -> bool;
}

impl PyLine for &str {
    fn is_docstring(&self) -> bool {
        let trimmed = self.trim();
        trimmed.starts_with(consts::DocstringMarker::SINGLE)
            || trimmed.starts_with(consts::DocstringMarker::DOUBLE)
    }

    fn is_placeholder(&self) -> bool {
        let trimmed = self.trim();
        trimmed.starts_with(consts::Placeholder::PASS)
            || trimmed.starts_with(consts::Placeholder::ELLIPSIS)
    }

    fn is_decorator(&self) -> bool {
        self.trim().starts_with("@")
    }

    fn starts_with_token(&self, token: &str) -> bool {
        let parsed = self.trim().split(' ').nth(0);
        parsed.is_some() && (parsed.unwrap() == token)
    }

    fn is_method_def(&self) -> bool {
        self.starts_with_token("def")
    }

    fn is_class_def(&self) -> bool {
        self.starts_with_token("class")
    }

    fn is_class_field(&self) -> bool {
        !self.is_class_def()
            && self.indent_count() == 1
            && (self.contains('=') || self.contains(':') || self.is_enum_variant())
    }

    fn is_import(&self) -> bool {
        let trimmed = self.trim();
        trimmed.starts_with("import") || trimmed.starts_with("from")
    }

    fn is_full_docstring(&self) -> bool {
        let trimmed = self.trim();
        (trimmed.starts_with(consts::DocstringMarker::SINGLE)
            && trimmed.ends_with(consts::DocstringMarker::SINGLE)
            && trimmed.len() >= 6)
            || (trimmed.starts_with(consts::DocstringMarker::DOUBLE)
                && trimmed.ends_with(consts::DocstringMarker::DOUBLE)
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
