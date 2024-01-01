pub const INDENT: &str = "    ";

pub struct DocstringMarker;
impl DocstringMarker {
    pub const SINGLE: &'static str = "'''";
    pub const DOUBLE: &'static str = "\"\"\"";
}

pub struct Placeholder;
impl Placeholder {
    pub const PASS: &'static str = "pass";
    pub const ELLIPSIS: &'static str = "...";
}
