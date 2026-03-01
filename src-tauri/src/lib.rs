use sqlx::sqlite::SqlitePool;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::error;

// Layout file lock to prevent race conditions
pub struct LayoutFileLock(pub Mutex<()>);

// Add use statements for plugins
use tauri_plugin_dialog;
use tauri_plugin_fs;
use tauri_plugin_opener;

mod broker;
mod commands;
mod config;
mod database;
mod mqtt;
mod plugins;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            // Load config
            let config = config::load_config(app.handle()).unwrap_or_else(|e| {
                error!("Failed to load config: {}, using defaults", e);
                config::AppConfig::default()
            });

            // Start Broker asynchronously if mode is internal
            if config.broker.mode == "internal" {
                let port = config.broker.port;
                tauri::async_runtime::spawn(async move {
                    broker::start_broker(port).await;
                });
            }

            // App Data ディレクトリ配下に保存
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            // ディレクトリがなければ作成
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
            }
            let db_path = app_data_dir.join("app_data.db");

            let pool = tauri::async_runtime::block_on(async {
                database::init(app.handle(), db_path).await
            });

            match pool {
                Ok(pool) => {
                    app.manage(Some(pool));
                }
                Err(e) => {
                    error!("Failed to initialize database: {}", e);
                    app.manage::<Option<SqlitePool>>(None);
                }
            }

            // Initialize layout file lock
            app.manage(LayoutFileLock(Mutex::new(())));

            let handle = app.handle().clone();
            mqtt::init(&handle, &config).expect("Failed to initialize MQTT client");
            Ok(())
        })
        .register_uri_scheme_protocol("plugin", plugins::plugin_protocol_handler)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::publish_message,
            commands::publish_binary_message,
            commands::get_history,
            commands::save_layout,
            commands::load_layout,
            commands::get_last_layout_path,
            commands::get_app_settings,
            commands::save_app_settings,
            commands::export_widget_data_as_csv,
            commands::get_distinct_topics,
            commands::get_message_counts,
            commands::delete_messages,
            commands::save_proto_file,
            plugins::get_plugin_list,
            plugins::load_plugin_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
