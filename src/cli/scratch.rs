use std::{
    borrow::Cow,
    fs,
    io::{BufRead, BufReader},
    ops::RangeInclusive,
    path::PathBuf,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Ok};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use rested::{config::Config, editing::edit, interpreter::environment::Environment};

use super::run::RunArgs;

#[derive(Debug, Args)]
pub struct ScratchCommandArgs {
    #[command(subcommand)]
    command: Option<ScratchCommand>,

    /// Run the saved file when done editing
    #[arg(long)]
    run: bool,

    /// Namespace in which to look for environment variables
    #[arg(short = 'n', long, requires = "run")]
    namespace: Option<String>,

    /// One or more names of the specific request(s) to run
    #[arg(short = 'r', long, requires = "run", num_args(1..))]
    request: Option<Vec<String>>,
}

#[derive(Debug, Subcommand)]
pub enum ScratchCommand {
    /// List all the scratch files created or edited from oldest to newest
    History {
        /// Don't show scratch file previews, just show paths.
        #[arg(short, long)]
        quiet: bool,

        /// Select a subset of the history by a number, or a range like 1..3 (inclusive)
        #[arg(short, long)]
        select: Option<HistorySubsetSelection>,

        /// Show the index position for each scratch file relative to the oldest edited (since) or newest edited
        /// file (ago)
        #[arg(short = 'm', long = "index-mode", default_value = "ago")]
        index_mode: HistoryIndexMode,
    },

    /// Create a new scratch file
    New,

    /// Run the last scratch file edited
    Run {
        /// Namespace in which to look for environment variables
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// One or more names of the specific request(s) to run
        #[arg(short = 'r', long, num_args(1..))]
        request: Option<Vec<String>>,

        /// Rested will prompt you for which request to pick
        #[arg(long)]
        prompt: bool,
    },

    /// Pick a scratch file to edit
    Pick {
        /// The position of a scratch file in the list of scratch files.
        number: usize,

        /// Whether to pick a file at some position before the last scratch file edited, or since the oldest
        /// one edited.
        #[arg(value_enum)]
        mode: HistoryIndexMode,
    },
}

#[derive(Debug, Clone)]
pub enum HistorySubsetSelection {
    Range(RangeInclusive<usize>),
    Single(usize),
}

impl FromStr for HistorySubsetSelection {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s
            .split("..")
            .map(|n| n.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()?;

        match parts.as_slice() {
            [l, r] => Ok(HistorySubsetSelection::Range(*l..=*r)),
            [s] => Ok(HistorySubsetSelection::Single(*s)),
            _ => Err(anyhow!(
                "invalid selection defined, should be a number like 1 or a range like 1..3"
            )),
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum HistoryIndexMode {
    /// To pick a file at some position before the latest scratch file.
    Ago,
    /// To pick a file at some position since the oldest scratch file.
    Since,
}

impl ScratchCommandArgs {
    pub fn handle(&self, env: Environment) -> anyhow::Result<()> {
        match &self.command {
            Some(command) => match command {
                ScratchCommand::History {
                    quiet,
                    index_mode,
                    select,
                } => {
                    let files = fetch_scratch_files()?;
                    let len = files.len();

                    let iterations = files
                        .into_iter()
                        .enumerate()
                        .map(|(i, path)| {
                            (
                                match index_mode {
                                    HistoryIndexMode::Ago => len - i - 1,
                                    HistoryIndexMode::Since => i,
                                },
                                path,
                            )
                        })
                        .filter(|(i, _)| match select {
                            Some(HistorySubsetSelection::Range(r)) => r.contains(i),
                            Some(HistorySubsetSelection::Single(s)) => s == i,
                            _ => true,
                        })
                        .inspect(|(i, _)| {
                            match index_mode {
                                HistoryIndexMode::Ago => eprint!("{} ago: ", i),
                                HistoryIndexMode::Since => eprint!("{} since: ", i),
                            };
                        });

                    for (_, file_path) in iterations {
                        println!("{}", file_path.to_string_lossy().bold());

                        if !quiet {
                            let three_lines = fs::File::open(file_path)
                                .map(BufReader::new)
                                .map(|reader| reader.lines().map_while(Result::ok).take(3))?;

                            for (idx, line) in three_lines.enumerate() {
                                eprintln!("{}", format!("  {}|  {}", idx + 1, line).dimmed());
                            }
                        }
                    }
                }
                ScratchCommand::New => {
                    let file_name = create_scratch_file()?;
                    edit(file_name)?;
                }
                ScratchCommand::Run {
                    namespace,
                    request,
                    prompt,
                } => {
                    let file_name = match fetch_scratch_files()?.last().cloned() {
                        Some(last) => last,
                        None => create_scratch_file()?,
                    };

                    RunArgs {
                        request: request.clone(),
                        namespace: namespace.clone(),
                        file: Some(file_name),
                        prompt: *prompt,
                    }
                    .handle(env)?;
                }
                ScratchCommand::Pick { number, mode } => {
                    let files = fetch_scratch_files()?;

                    let index = match mode {
                        HistoryIndexMode::Ago => files.len() - number - 1,
                        HistoryIndexMode::Since => *number,
                    };

                    let file_name = files
                        .get(index)
                        .ok_or_else(|| {
                            anyhow!(
                                "index '{}' is out of bounds, there are {} scratch files",
                                number,
                                files.len()
                            )
                        })
                        .context("no scratch file found")?;

                    edit(file_name)?;
                }
            },
            None => {
                let file_name = if let Some(file) = fetch_scratch_files()?.last().cloned() {
                    file
                } else {
                    create_scratch_file()?
                };

                edit(&file_name)?;

                if self.run {
                    RunArgs {
                        request: self.request.clone(),
                        namespace: self.namespace.clone(),
                        file: Some(file_name),
                        prompt: false,
                    }
                    .handle(env)?;
                }
            }
        }

        Ok(())
    }
}

fn create_scratch_file() -> anyhow::Result<PathBuf> {
    let prefix_path = Config::load()?.scratch_dir;

    let path = prefix_path.join::<String>(format!(
        "scratch-{:?}.rd",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ));

    fs::File::create(&path)?;

    Ok(path)
}

fn fetch_scratch_files() -> anyhow::Result<Vec<PathBuf>> {
    let prefix_path = Config::load()?.scratch_dir;

    let mut entries = fs::read_dir(prefix_path)?
        .map(|res| {
            res.context("failed to get a directory entry")
                .and_then(|e| {
                    e.metadata()
                        .context("failed to get metadata")
                        .and_then(|meta| {
                            meta.modified()
                                .context("failed to get a last modified time")
                        })
                        .and_then(|m| {
                            m.duration_since(UNIX_EPOCH)
                                .map(|d| d.as_millis())
                                .context("failed to convert last modified time to milliseconds")
                        })
                        .map(|last_mod_time| (e.path(), last_mod_time))
                })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    entries.sort_by(|(_, a), (_, b)| a.cmp(b));

    let scratch_files = entries
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| {
            matches!(
                path.extension().map(|e| e.to_string_lossy()),
                Some(Cow::Borrowed("rd"))
            )
        })
        .collect::<Vec<_>>();

    Ok(scratch_files)
}
