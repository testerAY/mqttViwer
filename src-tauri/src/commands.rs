use rumqttc::{AsyncClient, QoS};
use sqlx::SqlitePool;
use crate::mqtt::MqttMessage;

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
