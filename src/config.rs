use anyhow::Result;
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
    #[serde(default)]
    pub credentials: CredentialsConfig,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CredentialsConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
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
            credentials: CredentialsConfig::default(),
            token: None,
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
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
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

pub fn get_credentials_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        dirs::config_dir()
            .map(|p| p.join("spotiline").join("credentials.toml"))
            .unwrap_or_else(|| PathBuf::from("credentials.toml"))
    } else {
        dirs::config_dir()
            .map(|p| p.join("spotiline").join("credentials.toml"))
            .unwrap_or_else(|| PathBuf::from("~/.config/spotiline/credentials.toml"))
    }
}

pub fn load_credentials() -> CredentialsConfig {
    let path = get_credentials_path();
    if !path.exists() {
        return CredentialsConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => CredentialsConfig::default(),
    }
}

pub fn save_credentials(creds: &CredentialsConfig) -> Result<()> {
    let path = get_credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(creds)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn load_token_from_config() -> Option<String> {
    let config = load_config();
    config.token
}

pub fn save_token_to_config(token: &str) -> Result<()> {
    let mut config = load_config();
    config.token = Some(token.to_string());
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(&config)?;
    std::fs::write(&path, content)?;
    Ok(())
}
