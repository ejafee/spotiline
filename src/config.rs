use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_PORT: u16 = 47836;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_audio_backend")]
    pub audio_backend: String,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_bg")]
    pub color_bg: String,
    #[serde(default = "default_highlight")]
    pub color_highlight: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            audio_backend: "auto".to_string(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color_bg: "#121212".to_string(),
            color_highlight: "#1DB954".to_string(),
        }
    }
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn default_audio_backend() -> String {
    "auto".to_string()
}

fn default_bg() -> String {
    "#121212".to_string()
}

fn default_highlight() -> String {
    "#1DB954".to_string()
}

pub fn load_config() -> Config {
    let path = get_config_path();

    if !path.exists() {
        return Config::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(_) => Config::default(),
        },
        Err(_) => Config::default(),
    }
}

fn get_config_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        dirs::config_dir()
            .map(|p| p.join("spotiline").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    } else {
        dirs::config_dir()
            .map(|p| p.join("spotiline").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("~/.config/spotiline/config.toml"))
    }
}
