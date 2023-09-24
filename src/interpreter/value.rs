use std::{collections::HashMap, fmt::Display};

use crate::utils;

#[derive(Debug, serde::Serialize)]
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

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => f.write_str(s),
            Value::Array(elements) => f.write_str(&format!(
                "[{}]",
                elements
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )),
            Value::Object(map) => f.write_str(&format!(
                "{{{}}}",
                map.iter()
                    .map(|(key, value)| format!("\"{}\": {}", key, value))
                    .collect::<Vec<_>>()
                    .join(",")
            )),
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
        }
    }
}

impl Value {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .expect("failed to json stringify this value; even though our parser should made sure this is value is valid")
    }
}
