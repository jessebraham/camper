use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    fs,
    io::Write as _,
    path::PathBuf,
};

use anyhow::{bail, Context, Result};
use directories::UserDirs;
use serde::{Deserialize, Serialize};

use crate::format::Format;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Config {
    pub fan_id: Option<u32>,
    pub identity: Option<String>,
    pub library: Option<PathBuf>,
    pub format: Option<Format>,
}

impl Config {
    const CONFIG_PATH: &'static str = ".camper";
    const CONFIG_FILE: &'static str = "config.toml";

    pub fn new(fan_id: u32, identity: String, library: PathBuf, format: Format) -> Self {
        Self {
            fan_id: Some(fan_id),
            identity: Some(identity),
            library: Some(library),
            format: Some(format),
        }
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_file_path()?;
        let config = match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)?,
            _ => Config::default(),
        };

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path()?;
        if !path.exists() {
            fs::File::create(&path).with_context(|| {
                format!("unable to create configuration file '{}'", path.display())
            })?;
        }

        let mut file = fs::OpenOptions::new().write(true).open(&path)?;
        let toml = toml::to_string(self)?;

        write!(file, "{}", toml)
            .with_context(|| format!("unable to save configuration file '{}'", path.display()))?;

        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        let fan_id_cfgd = match self.fan_id {
            Some(id) if id > 0 => true,
            _ => false,
        };

        let identity_cfgd = match &self.identity {
            Some(ident) if !ident.is_empty() => true,
            _ => false,
        };

        let library_cfgd = match &self.library {
            Some(lib) if lib.exists() => true,
            _ => false,
        };

        fan_id_cfgd && identity_cfgd && library_cfgd && self.format.is_some()
    }

    /// Returns the path to the application's configuration file, ensuring the
    /// configuration directory exists in the process.
    fn config_file_path() -> Result<PathBuf> {
        let home = match UserDirs::new() {
            Some(user_dirs) => user_dirs.home_dir().to_owned(),
            None => bail!("unable to determine user's home directory"),
        };

        let path = home.join(Self::CONFIG_PATH);
        if !path.exists() {
            fs::create_dir_all(&path).with_context(|| {
                format!(
                    "unable to create configuration directory '{}'",
                    path.display()
                )
            })?;
        }

        let path = path.join(Self::CONFIG_FILE);

        Ok(path)
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let mut s = String::new();

        if let Some(fan_id) = self.fan_id {
            s.push_str(&format!("fan_id:   {}\n", fan_id));
        }
        if let Some(identity) = &self.identity {
            s.push_str(&format!("identity: {}\n", identity));
        }
        if let Some(library) = &self.library {
            s.push_str(&format!("library:  {}\n", library.display()));
        }
        if let Some(format) = self.format {
            s.push_str(&format!("format:   {}\n", format.to_string()));
        }

        write!(f, "{}", s)
    }
}
