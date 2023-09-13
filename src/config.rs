use std::{fs, path::PathBuf};

use anyhow::Context;

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
    #[cfg(windows)]
    let home_dir_key = "USERPROFILE";

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

#[inline]
pub fn env_file_path() -> anyhow::Result<PathBuf> {
    get_home_dir().map(|home| home.join(".env.rd.json"))
}
