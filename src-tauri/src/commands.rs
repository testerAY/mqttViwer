use rumqttc::{AsyncClient, QoS};

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
