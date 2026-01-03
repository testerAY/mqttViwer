use crate::config::AppConfig;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

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

fn generate_client_id() -> String {
    format!("rumqtt-client-{}", Uuid::new_v4())
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

async fn write_batch_to_db(pool: &SqlitePool, messages: &[MqttMessage]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for msg in messages {
        let (data_type, value_num) = analyze_payload(&msg.payload);

        sqlx::query(
            "INSERT INTO messages (topic, payload, timestamp, data_type, value_num) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&msg.topic)
        .bind(&msg.payload)
        .bind(msg.timestamp)
        .bind(data_type)
        .bind(value_num)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub fn init<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    config: &AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let client_id = generate_client_id();
    let mut mqttoptions = MqttOptions::new(client_id, &config.broker.host, config.broker.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    if let (Some(u), Some(p)) = (&config.broker.username, &config.broker.password) {
        if !u.is_empty() {
            mqttoptions.set_credentials(u, p);
        }
    }

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
            let mut buffer: Vec<MqttMessage> = Vec::with_capacity(100);
            loop {
                // Use a timeout to process the buffer periodically
                let recv_result = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;

                match recv_result {
                    // Message received, add to buffer
                    Ok(Some(msg)) => {
                        buffer.push(msg);
                        if buffer.len() >= 100 {
                            // Buffer is full, write to DB
                            match write_batch_to_db(&pool, &buffer).await {
                                Ok(_) => buffer.clear(),
                                Err(e) => {
                                    error!("Failed to write batch to database: {}", e);
                                    if buffer.len() >= 1000 {
                                        buffer.drain(0..100);
                                        warn!("Buffer full (1000), dropping oldest 100 messages");
                                    }
                                }
                            }
                        }
                    }
                    // Channel closed
                    Ok(None) => {
                        // Write any remaining messages before exiting
                        if !buffer.is_empty() {
                            if let Err(e) = write_batch_to_db(&pool, &buffer).await {
                                error!("Failed to write final batch to database: {}", e);
                            }
                            buffer.clear();
                        }
                        break; // Exit loop
                    }
                    // Timeout elapsed
                    Err(_) => {
                        // Timeout, write buffer to DB if not empty
                        if !buffer.is_empty() {
                            match write_batch_to_db(&pool, &buffer).await {
                                Ok(_) => buffer.clear(),
                                Err(e) => {
                                    error!("Failed to write batch to database on timeout: {}", e);
                                    if buffer.len() >= 1000 {
                                        buffer.drain(0..100);
                                        warn!("Buffer full (1000), dropping oldest 100 messages");
                                    }
                                }
                            }
                        }
                    }
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
                    .await
                {
                    Ok(result) => info!(
                        "Retention policy applied. Deleted {} rows.",
                        result.rows_affected()
                    ),
                    Err(e) => error!("Failed to apply retention policy: {}", e),
                }
            }
        });
    }

    let app_handle = app.clone();
    let client_clone = client.clone();
    let config_clone = config.clone();

    tauri::async_runtime::spawn(async move {
        // Wait for broker to start up completely
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Subscribe to the configured topic
        let topic = &config_clone.broker.subscription_topic;
        let qos = match config_clone.broker.qos {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        };
        info!("Subscribing to topic: {} with QoS: {:?}", topic, qos);
        if let Err(e) = client_clone.subscribe(topic, qos).await {
            error!("Failed to subscribe to topic {}: {}", topic, e);
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
    fn test_client_id_generation() {
        let id1 = generate_client_id();
        let id2 = generate_client_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("rumqtt-client-"));
    }

    #[tokio::test]
    async fn test_buffer_retention_logic() {
        // Simulate buffer
        let mut buffer: Vec<MqttMessage> = Vec::new();
        let max_size = 1000;
        let drop_size = 100;

        // Helper to simulate write failure
        // We use a simple closure that always returns error
        let mock_write_fail = |_buf: &[MqttMessage]| async { Err::<(), &str>("DB Error") };

        // Fill buffer to max
        for i in 0..max_size {
            buffer.push(MqttMessage {
                topic: "test".to_string(),
                payload: i.to_string(),
                timestamp: 0,
                data_type: None,
                value_num: None,
            });
        }

        assert_eq!(buffer.len(), 1000);

        // Trigger logic: Failed write
        if let Err(_) = mock_write_fail(&buffer).await {
            if buffer.len() >= max_size {
                buffer.drain(0..drop_size);
            }
        }

        assert_eq!(buffer.len(), 900);
        // Verify first item is now "100" (since 0-99 were dropped)
        assert_eq!(buffer[0].payload, "100");
    }

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
