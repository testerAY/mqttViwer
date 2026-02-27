use crate::config;
use crate::mqtt::MqttMessage;
use crate::LayoutFileLock;
use rumqttc::{AsyncClient, QoS};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct DataMapping {
    id: String,
    name: String,
    #[serde(rename = "type")]
    mapping_type: String, // "sub" | "pub"
    topic: String,
    #[serde(rename = "valueKey")]
    value_key: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardLayout {
    items: Vec<DashboardItem>,
    #[serde(rename = "dataMappings")]
    data_mappings: Vec<DataMapping>,
}

fn update_app_config(app: &tauri::AppHandle, layout_path: String) -> Result<(), String> {
    let mut config = config::load_config(app)?;
    config.last_layout_path = Some(layout_path);
    config::save_config(app, &config)
}

#[tauri::command]
pub async fn get_last_layout_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let config = config::load_config(&app)?;
    Ok(config.last_layout_path)
}

#[tauri::command]
pub async fn get_app_settings(app: tauri::AppHandle) -> Result<config::AppConfig, String> {
    config::load_config(&app)
}

#[tauri::command]
pub async fn save_app_settings(
    app: tauri::AppHandle,
    config: config::AppConfig,
) -> Result<(), String> {
    config::save_config(&app, &config)
}

fn save_layout_impl(path: &Path, layout: &DashboardLayout) -> Result<(), String> {
    let json = serde_json::to_string_pretty(layout).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_layout_impl(path: &Path) -> Result<DashboardLayout, String> {
    if !path.exists() {
        return Ok(DashboardLayout {
            items: vec![],
            data_mappings: vec![],
        });
    }
    let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let layout: DashboardLayout = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(layout)
}

#[tauri::command]
pub async fn save_layout(
    app: tauri::AppHandle,
    lock: tauri::State<'_, LayoutFileLock>,
    path: String,
    layout: DashboardLayout,
) -> Result<(), String> {
    println!(
        "Backend: save_layout called. Path: {}, Items: {}, Mappings: {}",
        path,
        layout.items.len(),
        layout.data_mappings.len()
    );
    // Acquire lock to prevent race conditions
    let _guard = lock.0.lock().await;

    let layout_path = PathBuf::from(&path);
    save_layout_impl(&layout_path, &layout)?;
    update_app_config(&app, path)?;
    Ok(())
}

#[tauri::command]
pub async fn load_layout(app: tauri::AppHandle, path: String) -> Result<DashboardLayout, String> {
    println!("Backend: load_layout called. Path: {}", path);
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

        let layout = DashboardLayout {
            items: vec![DashboardItem {
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
            }],
            data_mappings: vec![DataMapping {
                id: "map1".to_string(),
                name: "Test Map".to_string(),
                mapping_type: "sub".to_string(),
                topic: "test/topic".to_string(),
                value_key: None,
                description: None,
            }],
        };

        // Save
        let result = save_layout_impl(&file_path, &layout);
        assert!(result.is_ok(), "Failed to save layout");

        // Load
        let loaded_result = load_layout_impl(&file_path);
        assert!(loaded_result.is_ok(), "Failed to load layout");
        let loaded_layout = loaded_result.unwrap();

        assert_eq!(loaded_layout.items.len(), 1);
        assert_eq!(loaded_layout.items[0].i, "1");
        assert_eq!(loaded_layout.items[0].widget.title, "Test Widget");
        assert_eq!(loaded_layout.data_mappings.len(), 1);
        assert_eq!(loaded_layout.data_mappings[0].id, "map1");

        // Clean up
        let _ = fs::remove_file(file_path);
    }
}

#[tauri::command]
pub async fn publish_message(
    client: tauri::State<'_, AsyncClient>,
    topic: String,
    payload: String,
    qos: u8,
    retain: bool,
) -> Result<(), String> {
    let qos_level = match qos {
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtMostOnce,
    };

    client
        .publish(topic, qos_level, retain, payload.as_bytes())
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

    let messages = sqlx::query_as::<_, MqttMessage>(
        "SELECT topic, payload, timestamp, data_type, value_num FROM messages WHERE topic LIKE ? ORDER BY timestamp DESC LIMIT ?"
    )
    .bind(filter)
    .bind(limit)
    .fetch_all(p)
    .await
    .map_err(|e| e.to_string())?;

    Ok(messages)
}

#[tauri::command]
pub async fn export_widget_data_as_csv(
    pool: tauri::State<'_, Option<SqlitePool>>,
    topic: String,
) -> Result<String, String> {
    let p = match pool.inner() {
        Some(pool) => pool,
        None => return Err("Database not connected".to_string()),
    };

    let messages = sqlx::query_as::<_, MqttMessage>(
        "SELECT topic, payload, timestamp, data_type, value_num FROM messages WHERE topic = ? ORDER BY timestamp ASC"
    )
    .bind(&topic)
    .fetch_all(p)
    .await
    .map_err(|e| e.to_string())?;

    if messages.is_empty() {
        return Ok("".to_string());
    }

    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);

    wtr.write_record(&["timestamp", "topic", "payload", "data_type", "value_num"])
        .map_err(|e| e.to_string())?;

    for msg in messages {
        let timestamp_str = msg.timestamp.to_string();
        let value_num_str = msg.value_num.map(|v| v.to_string()).unwrap_or_default();
        let data_type_str = msg.data_type.as_deref().unwrap_or_default();
        wtr.write_record(&[
            &timestamp_str,
            &msg.topic,
            &msg.payload,
            data_type_str,
            &value_num_str,
        ])
        .map_err(|e| e.to_string())?;
    }

    let csv_data = String::from_utf8(wtr.into_inner().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(csv_data)
}

#[derive(FromRow)]
struct TopicRow {
    topic: String,
}

#[tauri::command]
pub async fn get_distinct_topics(
    pool: tauri::State<'_, Option<SqlitePool>>,
) -> Result<Vec<String>, String> {
    let p = match pool.inner() {
        Some(pool) => pool,
        None => return Ok(vec![]),
    };

    let rows =
        sqlx::query_as::<_, TopicRow>("SELECT DISTINCT topic FROM messages ORDER BY topic ASC")
            .fetch_all(p)
            .await
            .map_err(|e| e.to_string())?;

    let topics = rows.into_iter().map(|row| row.topic).collect();
    Ok(topics)
}
