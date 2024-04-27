use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
    str::FromStr,
};

use anyhow::{anyhow, Context};
use clap::Args;
use rested::{
    error::ColoredMetaError,
    interpreter::{environment::Environment, error::InterpreterError, ir},
    parser::ast::Program,
};

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Namespace in which to look for environment variables
    #[arg(short = 'n', long)]
    pub namespace: Option<String>,

    /// One or more names of the specific request(s) to run
    #[arg(short = 'r', long, num_args(1..))]
    pub request: Option<Vec<String>>,

    /// Path to the script to run. If none is provided, script is read
    /// from stdin
    pub file: Option<PathBuf>,

    /// Rested will prompt you for which request to pick
    #[arg(long, conflicts_with = "request")]
    pub prompt: bool,
}

#[derive(Debug)]
struct PromptEntry {
    method: String,
    url_or_name: String,
}

impl From<&ir::RequestItem> for PromptEntry {
    fn from(r: &ir::RequestItem) -> Self {
        let (m, n) = match r.name.clone() {
            Some(name) => (r.request.method.to_string(), name),
            None => (r.request.method.to_string(), r.request.url.clone()),
        };

        return PromptEntry {
            method: m,
            url_or_name: n,
        };
    }
}

impl FromStr for PromptEntry {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("::");

        let m = split
            .next()
            .context("can't get a prompt entry from an empty string")?;
        let n = split
            .next()
            .context("failed to get url or name from string")?;

        return Ok(PromptEntry {
            method: m.to_owned(),
            url_or_name: n.to_owned(),
        });
    }
}

impl PromptEntry {
    fn to_identifier(&self) -> String {
        return format!("{}::{}", self.method, self.url_or_name);
    }
}

impl RunArgs {
    pub fn handle(self, mut env: Environment) -> anyhow::Result<()> {
        if let Some(ns) = self.namespace {
            env.select_variables_namespace(ns);
        }

        let code = read_program_text(self.file)?;
        let program = interpret_program_file(&code, env)?;

        let requests = if self.prompt {
            Some(prompt_for_selected_request(&program)?)
        } else {
            self.request
        };

        program.run_ureq(requests.as_deref());

        Ok(())
    }
}

fn prompt_for_selected_request(program: &ir::Program) -> anyhow::Result<Vec<String>> {
    let request_names: Vec<_> = program
        .items
        .iter()
        .map(PromptEntry::from)
        .map(|p| p.to_identifier())
        .collect();

    let fuzzy_search_input = request_names.join("\n");

    use skim::prelude::*;
    use std::io::Cursor;

    let options = SkimOptionsBuilder::default().build().unwrap();

    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(fuzzy_search_input));

    let selected_items: Vec<_> = Skim::run_with(&options, Some(items))
        .map(|out| out.selected_items)
        .context("failed to run prompt to select request")?
        .iter()
        .map(|item| {
            let s = item.output();
            PromptEntry::from_str(&s)
                .expect("failed to parse prompt result entry")
                .url_or_name
        })
        .collect();

    return Ok(selected_items);
}

pub fn interpret_program_file(code: &str, env: Environment) -> anyhow::Result<ir::Program<'_>> {
    let program = Program::from(code);

    let program = program.interpret(&env).map_err(|value| match value {
        InterpreterError::ParseErrors(p) => {
            let error_string: String = p
                .errors
                .iter()
                .map(|e| ColoredMetaError(e).to_string())
                .collect();

            return anyhow!(error_string);
        }
        InterpreterError::EvalErrors(errors) => {
            let error_string: String = errors
                .iter()
                .map(|e| ColoredMetaError(e).to_string())
                .collect();

            return anyhow!(error_string);
        }
    })?;

    Ok(program)
}

pub fn read_program_text(file: Option<PathBuf>) -> anyhow::Result<String> {
    let code = file.map(fs::read_to_string).unwrap_or_else(|| {
        let mut buf = String::new();
        stdin().read_to_string(&mut buf)?;
        Ok(buf)
    })?;

    Ok(code)
}
