use core::panic;
use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context};
use tracing::warn;

use crate::{interpreter::environment::Environment, ENV_FILE_NAME};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub scratch_dir: PathBuf,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        return confy::load("rested", None).map_err(|e| e.into());
    }

    pub fn save(self) -> anyhow::Result<()> {
        return confy::store("rested", None, self).map_err(|e| e.into());
    }
}

impl Default for Config {
    fn default() -> Self {
        let folder_name = "rested-scratch";

        let home = get_home_dir().unwrap_or_else(|e| panic!("{e}"));

        let scratch_dir = home.join(folder_name);

        if !scratch_dir.exists() {
            fs::create_dir(&scratch_dir).unwrap_or_else(|_| {
                panic!(
                    "failed to create a directory for the scratch files: {}",
                    scratch_dir.to_string_lossy()
                )
            })
        }

        Self { scratch_dir }
    }
}

fn get_home_dir() -> anyhow::Result<PathBuf> {
    #[cfg(unix)]
    let home_dir_key = "HOME";

    let home = std::env::var(home_dir_key).with_context(|| {
        format!(
            "failed to read the user's home directory, using the {} environment variable",
            home_dir_key
        )
    })?;

    Ok(home.into())
}

pub fn get_env_from_home_dir() -> anyhow::Result<Environment> {
    let env = get_home_dir()
        .map(|home| home.join(ENV_FILE_NAME))
        .context(anyhow!("failed to resolve the environment vars definition file from home dir: should be `{ENV_FILE_NAME}` in your home dir"))
        .and_then(|path| {
            Environment::new(path).context("failed to load the environment for rstd")
        })?;

    return Ok(env);
}

fn get_env_from_dir_path(path: &std::path::Path) -> anyhow::Result<Environment> {
    if !path.is_dir() {
        return Err(anyhow::anyhow!(
            "path given needs to be a directory: '{}'",
            path.display()
        ));
    }

    let path = std::path::Path::new(&path).join(ENV_FILE_NAME);

    if !path.exists() {
        return Err(anyhow::anyhow!("no such file `{ENV_FILE_NAME}`")
            .context("the workspace doesn't have an env its own environment values"));
    }

    let env = Environment::new(path).context("failed to load the environment for rstd")?;

    return Ok(env);
}

pub fn get_env_from_dir_path_or_from_home_dir(
    path: Option<&std::path::Path>,
) -> anyhow::Result<Environment> {
    let Some(path) = path else {
        return get_env_from_home_dir();
    };

    return get_env_from_dir_path(path).or_else(|e| {
        let error = e.context(anyhow!("failed to get env from path, {}", path.display()));
        warn!("{error:#}");

        warn!("falling back to `{ENV_FILE_NAME}` in home dir");

        get_env_from_home_dir().context("failed to get env from home dir")
    });
}
