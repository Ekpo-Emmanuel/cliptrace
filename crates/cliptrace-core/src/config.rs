use std::fs;
use std::path::PathBuf;

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
    /// Empty or omitted: next to the exe, then `<data dir>\\runtime\\onnxruntime.dll`.
    #[serde(default)]
    pub onnxruntime_path: String,
    /// Global popup hotkey, e.g. `Ctrl+Alt+V`.
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// Launch at Windows sign-in (HKCU Run). Mirrors the registry value we set.
    #[serde(default)]
    pub autostart: bool,
    /// Whether the first-run welcome screen has been dismissed. A config file written before
    /// this field existed counts as welcomed, so upgrading users are not greeted again.
    #[serde(default = "default_welcomed")]
    pub welcomed: bool,
}

fn default_welcomed() -> bool {
    true
}

fn default_secret_ttl_hours() -> u32 {
    24
}

fn default_retention_days() -> u32 {
    90
}

fn default_hotkey() -> String {
    "Ctrl+Alt+V".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            blocklist: Vec::new(),
            secret_ttl_hours: default_secret_ttl_hours(),
            retention_days: default_retention_days(),
            onnxruntime_path: String::new(),
            hotkey: default_hotkey(),
            autostart: false,
            welcomed: false,
        }
    }
}

impl Config {
    /// Where to load `onnxruntime.dll` from: the configured path if that file exists, then
    /// next to the exe (how a download ships it), then `<data dir>\runtime`.
    pub fn resolved_onnxruntime_path(&self, paths: &Paths) -> PathBuf {
        let trimmed = self.onnxruntime_path.trim();
        if !trimmed.is_empty() {
            let configured = PathBuf::from(trimmed);
            if configured.exists() {
                return configured;
            }
        }
        if let Some(beside_exe) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("onnxruntime.dll")))
        {
            if beside_exe.exists() {
                return beside_exe;
            }
        }
        paths.onnxruntime_dll()
    }
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
    pub models_dir: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self, ConfigError> {
        Ok(Self::from_data_dir(data_dir()?))
    }

    pub fn from_data_dir(data_dir: PathBuf) -> Self {
        let db_path = data_dir.join("cliptrace.db");
        let config_path = data_dir.join("config.toml");
        let models_dir = data_dir.join("models");
        Self {
            data_dir,
            db_path,
            config_path,
            models_dir,
        }
    }

    pub fn onnxruntime_dll(&self) -> PathBuf {
        self.data_dir.join("runtime").join("onnxruntime.dll")
    }

    pub fn ensure_data_dir(&self) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.models_dir)?;
        fs::create_dir_all(self.data_dir.join("runtime"))?;
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

pub fn load_or_create(paths: &Paths) -> Result<Config, ConfigError> {
    if paths.config_path.exists() {
        let raw = fs::read_to_string(&paths.config_path)?;
        return Ok(toml::from_str(&raw)?);
    }
    let config = Config::default();
    if let Some(parent) = paths.config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&paths.config_path, toml::to_string_pretty(&config)?)?;
    Ok(config)
}

pub fn save(paths: &Paths, config: &Config) -> Result<(), ConfigError> {
    fs::write(&paths.config_path, toml::to_string_pretty(config)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths_in(name: &str) -> Paths {
        let dir = std::env::temp_dir().join(format!("cliptrace-cfg-{name}-{}", std::process::id()));
        Paths::from_data_dir(dir)
    }

    #[test]
    fn onnxruntime_uses_configured_path_when_the_file_exists() {
        let paths = paths_in("configured");
        fs::create_dir_all(&paths.data_dir).unwrap();
        let dll = paths.data_dir.join("custom.dll");
        fs::write(&dll, b"x").unwrap();
        let config = Config {
            onnxruntime_path: dll.to_string_lossy().into_owned(),
            ..Config::default()
        };
        assert_eq!(config.resolved_onnxruntime_path(&paths), dll);
        let _ = fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn onnxruntime_falls_back_to_the_data_dir_when_nothing_else_exists() {
        let paths = paths_in("fallback");
        let config = Config {
            onnxruntime_path: r"C:\does\not\exist\onnxruntime.dll".into(),
            ..Config::default()
        };
        assert_eq!(config.resolved_onnxruntime_path(&paths), paths.onnxruntime_dll());
    }

    #[test]
    fn a_new_install_is_not_welcomed_but_an_older_config_file_is() {
        assert!(!Config::default().welcomed);
        let old: Config = toml::from_str("retention_days = 30\n").unwrap();
        assert!(old.welcomed);
    }

    #[test]
    fn a_fresh_config_file_records_that_the_welcome_is_pending() {
        let paths = paths_in("welcome");
        let created = load_or_create(&paths).unwrap();
        assert!(!created.welcomed);
        let reread = load_or_create(&paths).unwrap();
        assert!(!reread.welcomed);
        let _ = fs::remove_dir_all(&paths.data_dir);
    }
}
