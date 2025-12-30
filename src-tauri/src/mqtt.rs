use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};
use serde::{Serialize, Deserialize};
use tauri::Emitter;
use std::time::Duration;
use sqlx::sqlite::{SqlitePool, SqliteConnectOptions};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
    pub timestamp: u64,
}

pub fn start_client<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        // Since we are accessing the same file, we can just connect to it.
        // Retry connection a few times if needed.
        let db_url = "sqlite:app_data.db";
        
        let mut pool = None;
        for _ in 0..5 {
             let options = SqliteConnectOptions::from_str(db_url)
                 .unwrap()
                 .create_if_missing(true);
                 
             match SqlitePool::connect_with(options).await {
                Ok(p) => {
                    pool = Some(p);
                    break;
                }
                Err(e) => {
                    println!("Failed to connect to DB: {}, retrying...", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
             }
        }
        
        if pool.is_none() {
             println!("Could not connect to database after retries. MQTT Client will not save data.");
             return; 
        }
        let pool = pool.unwrap();

        let mut mqttoptions = MqttOptions::new("rumqtt-client", "localhost", 9883);

        mqttoptions.set_keep_alive(Duration::from_secs(5));

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 50);
        
        // Retry subscription logic could be added here
        if let Err(e) = client.subscribe("#", QoS::AtMostOnce).await {
             println!("Failed to subscribe: {}", e);
             // ここで return せず、再接続ロジックを入れるのが理想ですが、
             // まずはログを出して終了でOK。ただしブローカー起動待ちを入れると安定します。
             tokio::time::sleep(Duration::from_secs(1)).await;
             return;
        }

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let message = MqttMessage {
                        topic: publish.topic.to_string(),
                        payload: String::from_utf8_lossy(&publish.payload).to_string(),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };
                    println!("Received = {:?}", message);
                    
                    // Emit to frontend
                    if let Err(e) = app.emit("mqtt-message", &message) {
                        println!("Failed to emit message: {}", e);
                    }
                    
                    // Insert into database via sqlx
                    let result = sqlx::query(
                        "INSERT INTO messages (topic, payload, timestamp) VALUES (?, ?, ?)"
                    )
                    .bind(&message.topic)
                    .bind(&message.payload)
                    .bind(message.timestamp as i64) // SQLite stores integers as signed
                    .execute(&pool)
                    .await;

                    if let Err(e) = result {
                        println!("Failed to insert message into database: {}", e);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    println!("Error = {:?}", e);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }

            }
        }
    });
}
