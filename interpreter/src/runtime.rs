use std::{collections::HashMap, error::Error, path::PathBuf};

use lexer::Location;
use parser::ast::{Expression, Identifier};

pub struct Environment {
    env_file_name: PathBuf,
    pub namespaced_variables: HashMap<String, HashMap<String, String>>,
    selected_namespace: Option<String>,
}

impl Environment {
    pub fn new(file_name: PathBuf) -> Result<Self, std::io::Error> {
        let mut env = Self {
            env_file_name: file_name,
            namespaced_variables: HashMap::from([("default".to_string(), HashMap::new())]),
            selected_namespace: None,
        };

        env.load_variables_from_file()?;

        Ok(env)
    }

    fn load_variables_from_file(&mut self) -> Result<(), std::io::Error> {
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.env_file_name)?;

        let reader = std::io::BufReader::new(file);

        self.namespaced_variables = serde_json::from_reader(reader)
            .unwrap_or(HashMap::from([("default".to_string(), HashMap::new())]));

        Ok(())
    }

    pub fn select_variables_namespace(&mut self, ns: String) {
        self.selected_namespace = Some(ns);
    }

    fn selected_namespace(&self) -> String {
        self.selected_namespace
            .clone()
            .unwrap_or("default".to_string())
    }

    pub fn get_variable_value(&self, name: String) -> Option<&String> {
        let variables_map = self
            .namespaced_variables
            .get(&self.selected_namespace())
            .unwrap();

        variables_map.get(&name)
    }

    pub fn set_variable(&mut self, name: String, value: String) -> Result<(), Box<dyn Error>> {
        let variables_map = self
            .namespaced_variables
            .get_mut(&self.selected_namespace())
            .unwrap();

        variables_map.insert(name, value);

        self.save_to_file()?;

        Ok(())
    }

    pub fn save_to_file(&self) -> Result<(), Box<dyn Error>> {
        let file = std::fs::File::options()
            .write(true)
            .truncate(true)
            .open(&self.env_file_name)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer_pretty::<_, HashMap<_, _>>(writer, &self.namespaced_variables)?;

        Ok(())
    }
}

pub struct Attribute<'source> {
    pub name: &'source str,
    pub location: Location,
    pub params: Vec<Expression<'source>>,
}

impl<'source> Attribute<'source> {
    pub fn first_params(&self) -> Option<&Expression<'source>> {
        self.params.first()
    }
}

pub struct AttributeStore<'source> {
    inner: Vec<Attribute<'source>>,
}

impl<'source> AttributeStore<'source> {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn add(&mut self, id: Identifier<'source>, params: Vec<Expression<'source>>) {
        if self.has(id.name) {
            return;
        }

        self.inner.push(Attribute {
            name: id.name,
            location: id.location,
            params,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Attribute<'source>> {
        self.inner.iter().find(|att| att.name == name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
