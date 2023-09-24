use std::{fs::File, io::Read};

use anyhow::{anyhow, Context};
use thiserror::Error;

use super::value::Value;

pub fn read_file(file_name: Value) -> anyhow::Result<Value> {
    match file_name {
        Value::String(file_path) => {
            let mut file = File::open(file_path).context("failed to open a file for reading")?;

            let mut string = String::new();

            file.read_to_string(&mut string)
                .context("failed to read a file")?;

            Ok(string.into())
        }
        Value::Null => todo!(),
        Value::Array(elements) => todo!(),
        Value::Object(map) => todo!(),
        Value::Bool(_) => todo!(),
        Value::Number(_) => todo!(),
    }
}

#[derive(Debug, Error)]
pub enum CallEnvError {
    #[error("no variable found by the name {:?}", 0)]
    NotFound(String),
    #[error("env calls are only valid with string parameters, got null")]
    InvalidTypeNull,
    #[error("env calls are only valid with string parameters, got an array")]
    InvalidTypeArray,
    #[error("env calls are only valid with string parameters, got an object")]
    InvalidTypeObject,
}

pub fn call_env(
    env: &crate::interpreter::environment::Environment,
    variable: Value,
) -> std::result::Result<Value, CallEnvError> {
    match variable {
        Value::String(variable) => {
            let value = env
                .get_variable_value(&variable)
                .ok_or(CallEnvError::NotFound(variable))
                .map(|v| v.to_owned())?;

            Ok(value.into())
        }
        Value::Null => return Err(CallEnvError::InvalidTypeNull),
        Value::Array(..) => return Err(CallEnvError::InvalidTypeArray),
        Value::Object(..) => return Err(CallEnvError::InvalidTypeObject),
        Value::Bool(_) => todo!(),
        Value::Number(_) => todo!(),
    }
}
