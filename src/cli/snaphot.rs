use std::path::PathBuf;

use clap::{Args, ValueEnum};
use rested::interpreter::{
    environment::Environment,
    ir::{LogDestination, RequestItem},
};

use super::run::{interpret_program_file, read_program_text};

#[derive(Debug, Args)]
pub struct SnapshotArgs {
    /// Format of the snapshot output
    pub output_format: Format,

    /// Path to the script to snapshot
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Format {
    Curl,
}

impl SnapshotArgs {
    pub fn handle(self, env: Environment) -> anyhow::Result<()> {
        let code = read_program_text(self.file)?;
        let program = interpret_program_file(&code, env)?;

        for item in program.items.iter() {
            println!("{}\n", item.to_curl_string());
        }

        Ok(())
    }
}

trait ToCurlString {
    fn to_curl_string(&self) -> String;
}

impl ToCurlString for RequestItem {
    fn to_curl_string(&self) -> String {
        let mut buffer = String::new();

        if self.dbg {
            buffer.push_str("set -xe\n");
        }

        if let Some(name) = &self.name {
            buffer.push_str(&format!("echo {}", name))
        }

        buffer.push_str(&format!("curl -X {} ", self.request.method));

        for header in self.request.headers.iter() {
            buffer.push_str("-H ");
            buffer.push_str(&format!("\"{}: {}\" ", header.name, header.value));
        }

        if let Some(body) = &self.request.body {
            buffer.push_str(&format!("-d '{}' ", body));
        }

        buffer.push_str(&self.request.url);

        if let Some(dest) = &self.log_destination {
            match dest {
                LogDestination::Std => {}
                LogDestination::File(path) => {
                    buffer.push_str(&format!(" 1> {}", path.to_string_lossy()))
                }
            }
        }

        if self.dbg {
            buffer.push_str("\nset +xe");
        }

        buffer
    }
}
