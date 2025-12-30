use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use serde::{Serialize, Deserialize};
use tauri::{Emitter, Manager};
use std::time::Duration;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MqttStatus {
    pub status: String,
}

pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new("rumqtt-client", "localhost", 9883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 50);

    // Register client to state so it can be accessed by commands
    app.manage(client.clone());

    // Get DB pool from state
    let pool_state = app.state::<Option<SqlitePool>>();
    let pool = pool_state.inner().clone();

    let app_handle = app.clone();
    let client_clone = client.clone();

    tauri::async_runtime::spawn(async move {
        // Wait for broker to start up completely
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Subscribe to all topics
        if let Err(e) = client_clone.subscribe("#", QoS::AtMostOnce).await {
             println!("Failed to subscribe: {}", e);
             tokio::time::sleep(Duration::from_secs(1)).await;
        }

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let message = MqttMessage {
                        topic: publish.topic.to_string(),
                        payload: String::from_utf8_lossy(&publish.payload).to_string(),
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    println!("Received = {:?}", message);
                    
                    // Emit to frontend
                    if let Err(e) = app_handle.emit("mqtt-message", &message) {
                        println!("Failed to emit message: {}", e);
                    }
                    
                    // Insert into database via sqlx
                    if let Some(p) = &pool {
                        let result = sqlx::query(
                            "INSERT INTO messages (topic, payload, timestamp) VALUES (?, ?, ?)"
                        )
                        .bind(&message.topic)
                        .bind(&message.payload)
                        .bind(message.timestamp) 
                        .execute(p)
                        .await;

                        if let Err(e) = result {
                            println!("Failed to insert message into database: {}", e);
                        }
                    }
                }
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    println!("MQTT Connected!");
                    let status = MqttStatus {
                        status: "connected".to_string(),
                    };
                    if let Err(e) = app_handle.emit("mqtt-status", &status) {
                        println!("Failed to emit status: {}", e);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    println!("Error = {:?}", e);
                    let status = MqttStatus {
                        status: "disconnected".to_string(),
                    };
                    if let Err(e) = app_handle.emit("mqtt-status", &status) {
                        println!("Failed to emit status: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }

            }
        }
    });

    Ok(())
}
