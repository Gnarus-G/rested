use std::collections::HashMap;

#[derive(Debug, enum_tags::Tag, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    String(String),
    Bool(bool),
    Number(f64),
    Array(Box<[Value]>),
    Object(HashMap<String, Value>),
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
