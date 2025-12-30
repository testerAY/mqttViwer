use sqlx::sqlite::{SqlitePool, SqliteConnectOptions};
use std::str::FromStr;
use tauri::Manager;
use tracing::error;

mod broker;
mod mqtt;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            // Start Broker asynchronously
            tauri::async_runtime::spawn(async {
                broker::start_broker().await;
            });

            // App Data ディレクトリ配下に保存
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            // ディレクトリがなければ作成
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
            }
            let db_path = app_data_dir.join("app_data.db");
            let db_url = format!("sqlite:{}", db_path.to_string_lossy());

            let options = SqliteConnectOptions::from_str(&db_url)
                .unwrap()
                .create_if_missing(true);

            let pool = tauri::async_runtime::block_on(async {
                match SqlitePool::connect_with(options).await {
                    Ok(pool) => {
                        // Create table if not exists
                        if let Err(e) = sqlx::query("CREATE TABLE IF NOT EXISTS messages (
                            id INTEGER PRIMARY KEY AUTOINCREMENT,
                            topic TEXT NOT NULL,
                            payload TEXT NOT NULL,
                            timestamp INTEGER NOT NULL
                        )")
                        .execute(&pool)
                        .await {
                            error!("Failed to create tables: {}", e);
                            Some(pool)
                        } else {
                            Some(pool)
                        }
                    }
                    Err(e) => {
                        error!("Failed to connect to DB: {}", e);
                        None
                    }
                }
            });
            app.manage(pool);

            let handle = app.handle().clone();
            mqtt::init(&handle).expect("Failed to initialize MQTT client");
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::publish_message,
            commands::get_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
