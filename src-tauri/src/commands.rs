use rumqttc::{AsyncClient, QoS};
use sqlx::SqlitePool;
use crate::mqtt::MqttMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct WidgetConfig {
    id: String,
    #[serde(rename = "type")]
    widget_type: String,
    title: String,
    topic: Option<String>,
    settings: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardItem {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    i: String,
    widget: WidgetConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    last_layout_path: Option<String>,
}

fn update_app_config(app: &tauri::AppHandle, layout_path: String) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config_path = app_data_dir.join("config.json");
    
    let config = AppConfig {
        last_layout_path: Some(layout_path),
    };
    
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_last_layout_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config_path = app_data_dir.join("config.json");
    
    if !config_path.exists() {
        return Ok(None);
    }
    
    let json = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&json).unwrap_or_default();
    
    Ok(config.last_layout_path)
}

fn save_layout_impl(path: &Path, layout: &Vec<DashboardItem>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(layout).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_layout_impl(path: &Path) -> Result<Vec<DashboardItem>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let layout: Vec<DashboardItem> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(layout)
}

#[tauri::command]
pub async fn save_layout(
    app: tauri::AppHandle,
    path: String,
    layout: Vec<DashboardItem>,
) -> Result<(), String> {
    let layout_path = PathBuf::from(&path);
    save_layout_impl(&layout_path, &layout)?;
    update_app_config(&app, path)?;
    Ok(())
}

#[tauri::command]
pub async fn load_layout(app: tauri::AppHandle, path: String) -> Result<Vec<DashboardItem>, String> {
    let layout_path = PathBuf::from(&path);
    let layout = load_layout_impl(&layout_path)?;
    update_app_config(&app, path)?;
    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_save_and_load_layout() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("test_layout.json");

        let layout = vec![
            DashboardItem {
                i: "1".to_string(),
                x: 0,
                y: 0,
                w: 2,
                h: 2,
                widget: WidgetConfig {
                    id: "1".to_string(),
                    widget_type: "test".to_string(),
                    title: "Test Widget".to_string(),
                    topic: None,
                    settings: None,
                },
            }
        ];

        // Save
        let result = save_layout_impl(&file_path, &layout);
        assert!(result.is_ok(), "Failed to save layout");

        // Load
        let loaded_result = load_layout_impl(&file_path);
        assert!(loaded_result.is_ok(), "Failed to load layout");
        let loaded_layout = loaded_result.unwrap();

        assert_eq!(loaded_layout.len(), 1);
        assert_eq!(loaded_layout[0].i, "1");
        assert_eq!(loaded_layout[0].widget.title, "Test Widget");

        // Clean up
        let _ = fs::remove_file(file_path);
    }
}

#[tauri::command]
pub async fn publish_message(
    client: tauri::State<'_, AsyncClient>,
    topic: String,
    payload: String,
) -> Result<(), String> {
    client
        .publish(topic, QoS::AtLeastOnce, false, payload.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_history(
    pool: tauri::State<'_, Option<SqlitePool>>,
    topic_filter: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<MqttMessage>, String> {
    let p = match pool.inner() {
        Some(pool) => pool,
        None => return Ok(vec![]), // Return empty list if DB is not connected
    };

    let limit = limit.unwrap_or(100);
    let filter = topic_filter.unwrap_or("%".to_string());
    
    // sqlx::query_as! macro checks types at compile time.
    // We map the database fields to our struct.
    // SQLite stores integers as i64, but our struct expects u64 for timestamp.
    // We need to cast it manually or use query_as function (non-macro).
    // Using non-macro query_as for easier type casting.
    
    let messages = sqlx::query_as::<_, MqttMessage>(
        "SELECT topic, payload, timestamp FROM messages WHERE topic LIKE ? ORDER BY timestamp DESC LIMIT ?"
    )
    .bind(filter)
    .bind(limit)
    .fetch_all(p)
    .await
    .map_err(|e| e.to_string())?;

    Ok(messages)
}
