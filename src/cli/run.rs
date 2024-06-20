use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use clap::Args;
use rested::interpreter::{
    environment::Environment, interpret_program, ir, read_program_text,
    runner::request_id::RequestId,
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

impl RunArgs {
    pub fn handle(self, mut env: Environment) -> anyhow::Result<()> {
        if let Some(ns) = self.namespace {
            env.select_variables_namespace(ns);
        }

        let code = read_program_text(self.file)?;
        let program = interpret_program(&code, env)?;

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
        .map(RequestId::from)
        .map(|p| p.as_string())
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
            RequestId::from_str(&s)
                .expect("failed to parse prompt result entry")
                .url_or_name
        })
        .collect();

    return Ok(selected_items);
}
