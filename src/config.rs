use std::{fs, path::PathBuf};

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

        #[cfg(windows)]
        let home_dir_key = "USERPROFILE";

        #[cfg(unix)]
        let home_dir_key = "HOME";

        let home = std::env::var(home_dir_key).unwrap_or_else(|_| {
            panic!(
                "failed to read the user's home directory, using the {} environment variable",
                home_dir_key
            )
        });

        let scratch_dir = PathBuf::from(home).join(folder_name);

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
