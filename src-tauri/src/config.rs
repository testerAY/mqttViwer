use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrokerConfig {
    pub mode: String, // "internal" or "external"
    pub host: String,
    pub port: u16,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            mode: "internal".to_string(),
            host: "127.0.0.1".to_string(),
            port: 9883,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetentionConfig {
    pub enabled: bool,
    pub days: u32,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            days: 7,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub last_layout_path: Option<String>,
    #[serde(default)]
    pub broker: BrokerConfig,
    #[serde(default)]
    pub theme: String, // "light", "dark", "system"
    #[serde(default)]
    pub retention: RetentionConfig,
}

pub fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    }
    Ok(app_data_dir.join("config.json"))
}

pub fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let config_path = get_config_path(app)?;
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }
    
    let json = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&json).unwrap_or_default();
    Ok(config)
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let config_path = get_config_path(app)?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(config_path, json).map_err(|e| e.to_string())?;
    Ok(())
}
