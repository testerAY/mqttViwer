use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use serde::{Serialize, Deserialize};
use tauri::{Emitter, Manager};
use std::time::Duration;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use crate::config::AppConfig;

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
    pub timestamp: i64,
    pub data_type: Option<String>,
    pub value_num: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MqttStatus {
    pub status: String,
}

fn analyze_payload(payload: &str) -> (Option<String>, Option<f64>) {
    let trimmed = payload.trim();
    if let Ok(num) = trimmed.parse::<f64>() {
        (Some("number".to_string()), Some(num))
    } else if serde_json::from_str::<serde_json::Value>(payload).is_ok() {
        (Some("json".to_string()), None)
    } else {
        (Some("text".to_string()), None)
    }
}

pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new("rumqtt-client", &config.broker.host, config.broker.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 50);

    // Register client to state so it can be accessed by commands
    app.manage(client.clone());

    // Get DB pool from state
    let pool_state = app.state::<Option<SqlitePool>>();
    let pool_opt = pool_state.inner().clone();
    
    // Create channel for DB writes
    let (tx, mut rx) = mpsc::channel::<MqttMessage>(100);

    // Spawn DB writer task
    if let Some(pool) = pool_opt.clone() {
        tauri::async_runtime::spawn(async move {
            while let Some(msg) = rx.recv().await {
                // Payload analysis
                let (data_type, value_num) = analyze_payload(&msg.payload);

                 let result = sqlx::query(
                    "INSERT INTO messages (topic, payload, timestamp, data_type, value_num) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&msg.topic)
                .bind(&msg.payload)
                .bind(msg.timestamp) 
                .bind(data_type)
                .bind(value_num)
                .execute(&pool)
                .await;

                if let Err(e) = result {
                    error!("Failed to insert message into database: {}", e);
                }
            }
        });
    } else {
        warn!("Database pool not available, messages will not be saved.");
    }

    // Retention Policy
    if config.retention.enabled {
        let pool_opt_retention = pool_opt.clone();
        let days = config.retention.days;
        tauri::async_runtime::spawn(async move {
            if let Some(pool) = pool_opt_retention {
                // Wait a bit for DB to be ready and system to stabilize
                tokio::time::sleep(Duration::from_secs(10)).await;
                
                let retention_seconds = days as i64 * 24 * 60 * 60;
                let threshold = chrono::Utc::now().timestamp() - retention_seconds;
                
                info!("Running retention policy: deleting messages older than {} days (timestamp < {})", days, threshold);
                
                match sqlx::query("DELETE FROM messages WHERE timestamp < ?")
                    .bind(threshold)
                    .execute(&pool)
                    .await {
                        Ok(result) => info!("Retention policy applied. Deleted {} rows.", result.rows_affected()),
                        Err(e) => error!("Failed to apply retention policy: {}", e),
                    }
            }
        });
    }

    let app_handle = app.clone();
    let client_clone = client.clone();

    tauri::async_runtime::spawn(async move {
        // Wait for broker to start up completely
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Subscribe to all topics
        if let Err(e) = client_clone.subscribe("#", QoS::AtMostOnce).await {
             error!("Failed to subscribe: {}", e);
             tokio::time::sleep(Duration::from_secs(1)).await;
        }

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let message = MqttMessage {
                        topic: publish.topic.to_string(),
                        payload: String::from_utf8_lossy(&publish.payload).to_string(),
                        timestamp: chrono::Utc::now().timestamp(),
                        data_type: None, // Filled in DB worker
                        value_num: None, // Filled in DB worker
                    };
                    info!("Received = {:?}", message);
                    
                    // Emit to frontend
                    if let Err(e) = app_handle.emit("mqtt-message", &message) {
                        error!("Failed to emit message: {}", e);
                    }
                    
                    // Send to DB writer
                    if let Err(e) = tx.send(message).await {
                        error!("Failed to send message to DB writer: {}", e);
                    }
                }
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    info!("MQTT Connected!");
                    let status = MqttStatus {
                        status: "connected".to_string(),
                    };
                    if let Err(e) = app_handle.emit("mqtt-status", &status) {
                        error!("Failed to emit status: {}", e);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Error = {:?}", e);
                    let status = MqttStatus {
                        status: "disconnected".to_string(),
                    };
                    if let Err(e) = app_handle.emit("mqtt-status", &status) {
                        error!("Failed to emit status: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }

            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_payload() {
        // Number check
        let (dt, val) = analyze_payload("123.45");
        assert_eq!(dt.as_deref(), Some("number"));
        assert_eq!(val, Some(123.45));

        // Integer as number
        let (dt, val) = analyze_payload("42");
        assert_eq!(dt.as_deref(), Some("number"));
        assert_eq!(val, Some(42.0));

        // JSON check
        let (dt, val) = analyze_payload(r#"{"key": "value"}"#);
        assert_eq!(dt.as_deref(), Some("json"));
        assert_eq!(val, None);

        // JSON Array check
        let (dt, val) = analyze_payload(r#"[1, 2, 3]"#);
        assert_eq!(dt.as_deref(), Some("json"));
        assert_eq!(val, None);

        // Text check
        let (dt, val) = analyze_payload("Hello World");
        assert_eq!(dt.as_deref(), Some("text"));
        assert_eq!(val, None);
        
        // Invalid JSON text
        let (dt, val) = analyze_payload("{key: value}"); // Invalid JSON (no quotes)
        assert_eq!(dt.as_deref(), Some("text"));
        assert_eq!(val, None);
    }
}
