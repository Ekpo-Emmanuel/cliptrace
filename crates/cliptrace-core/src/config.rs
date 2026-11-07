use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("LOCALAPPDATA is unavailable and CLIPTRACE_DATA_DIR is not set")]
    NoDataDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("config write error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub blocklist: Vec<String>,
    #[serde(default = "default_secret_ttl_hours")]
    pub secret_ttl_hours: u32,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_secret_ttl_hours() -> u32 {
    24
}

fn default_retention_days() -> u32 {
    90
}

impl Default for Config {
    fn default() -> Self {
        Self {
            blocklist: Vec::new(),
            secret_ttl_hours: default_secret_ttl_hours(),
            retention_days: default_retention_days(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self, ConfigError> {
        Ok(Self::from_data_dir(data_dir()?))
    }

    pub fn from_data_dir(data_dir: PathBuf) -> Self {
        let db_path = data_dir.join("cliptrace.db");
        let config_path = data_dir.join("config.toml");
        Self {
            data_dir,
            db_path,
            config_path,
        }
    }

    pub fn ensure_data_dir(&self) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }
}

pub fn data_dir() -> Result<PathBuf, ConfigError> {
    if let Ok(dir) = std::env::var("CLIPTRACE_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let local = dirs::data_local_dir().ok_or(ConfigError::NoDataDir)?;
    Ok(local.join("Cliptrace"))
}

pub fn load_or_create(config_path: &Path) -> Result<Config, ConfigError> {
    if config_path.exists() {
        let raw = fs::read_to_string(config_path)?;
        return Ok(toml::from_str(&raw)?);
    }
    let config = Config::default();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config_path, toml::to_string_pretty(&config)?)?;
    Ok(config)
}
