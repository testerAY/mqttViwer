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
mod rtsp;
mod rtsp_server;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

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

                // Wait for the broker to be ready before connecting the MQTT client.
                // Use a simple port-listening check via std::net (non-async, no lingering connection)
                // to avoid rumqttd logging "connection closed by peer" from raw TCP probes.
                let broker_port = config.broker.port;
                let broker_host = config.broker.host.clone();
                let addr = format!("{}:{}", broker_host, broker_port);
                for attempt in 1..=20 {
                    match std::net::TcpStream::connect_timeout(
                        &addr.parse::<std::net::SocketAddr>().unwrap(),
                        std::time::Duration::from_millis(200),
                    ) {
                        Ok(stream) => {
                            // Shut down cleanly to minimize broker-side errors
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                            tracing::info!("Broker ready on {} (attempt {})", addr, attempt);
                            break;
                        }
                        Err(_) => {
                            std::thread::sleep(std::time::Duration::from_millis(250));
                            if attempt == 20 {
                                tracing::warn!("Broker may not be ready after 5s, proceeding anyway");
                            }
                        }
                    }
                }
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
            eprintln!("[DIAG] mqtt::init completed, proceeding to RTSP setup...");

            // Initialize RTSP MJPEG server state (shared between server and manager)
            let rtsp_server_state = std::sync::Arc::new(rtsp_server::RtspServerState::new());
            app.manage(rtsp_server_state.clone());

            // Initialize our new RTSP PUSH Server Manager
            let rtsp_server_manager = rtsp::RtspServerManager::new(rtsp_server_state.clone());
            app.manage(rtsp_server_manager);

            // Start the MJPEG HTTP server
            let mjpeg_port = config.rtsp.server_port;
            eprintln!("[DIAG] Spawning MJPEG server on port {}...", mjpeg_port);
            let mjpeg_server_state = rtsp_server_state.clone();
            let mjpeg_app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                rtsp_server::start(mjpeg_server_state, mjpeg_port, mjpeg_app_handle).await;
            });


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
            plugins::load_plugin_file,
            // Old RTSP commands are removed, new one will be added
            commands::rtsp_check_ffmpeg,
            commands::rtsp_start_stream,
            commands::rtsp_stop_stream,
            commands::rtsp_start_recording,
            commands::rtsp_stop_recording,
            commands::rtsp_take_snapshot,
            commands::rtsp_server_list_streams,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                // Clean up all ffmpeg processes on exit
                let manager = app.state::<rtsp::RtspServerManager>();
                tauri::async_runtime::block_on(async {
                    manager.stop_all().await;
                });
                tracing::info!("All RTSP streams stopped on exit");
            }
        });
}
