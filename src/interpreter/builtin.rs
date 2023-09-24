use std::{fs::File, io::Read, path::PathBuf};

use anyhow::Context;

use super::value::Value;

pub fn read_file<P: Into<PathBuf>>(file_name: P) -> anyhow::Result<Value> {
    let mut file = File::open(file_name.into()).context("failed to open a file for reading")?;

    let mut string = String::new();

    file.read_to_string(&mut string)
        .context("failed to read a file")?;

    Ok(string.into())
}

pub fn call_env(
    env: &crate::interpreter::environment::Environment,
    variable: &String,
) -> Option<Value> {
    env.get_variable_value(variable)
        .map(|v| v.to_owned().into())
}
