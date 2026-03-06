use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Shared state for the MJPEG HTTP server.
/// Maps widget_id -> broadcast sender of JPEG frames.
#[derive(Clone)]
pub struct RtspServerState {
    pub streams: Arc<RwLock<HashMap<String, broadcast::Sender<Bytes>>>>,
    pub server_running: Arc<AtomicBool>,
    /// The actual port the server is listening on (may differ from config if fallback was used).
    pub actual_port: Arc<AtomicU16>,
}

impl RtspServerState {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            server_running: Arc::new(AtomicBool::new(false)),
            actual_port: Arc::new(AtomicU16::new(0)),
        }
    }

    /// Check if the MJPEG server is running.
    pub fn is_running(&self) -> bool {
        self.server_running.load(Ordering::Relaxed)
    }

    /// Get the actual port the MJPEG server is listening on.
    pub fn get_port(&self) -> u16 {
        self.actual_port.load(Ordering::Relaxed)
    }

    /// Register a new stream and return the broadcast sender for pushing frames.
    pub async fn add_stream(&self, widget_id: &str, capacity: usize) -> broadcast::Sender<Bytes> {
        let (tx, _) = broadcast::channel(capacity);
        let tx_clone = tx.clone();
        self.streams.write().await.insert(widget_id.to_string(), tx);
        tx_clone
    }

    /// Remove a stream entry.
    pub async fn remove_stream(&self, widget_id: &str) {
        self.streams.write().await.remove(widget_id);
    }
}

/// Start the Axum MJPEG HTTP server on the given port with retry and fallback logic.
/// First retries the configured port up to 3 times (1-second intervals).
/// If all retries fail, tries up to 5 alternative ports (port+1, port+2, ...).
/// Emits `mjpeg-server-status` event to notify the frontend.
pub async fn start(state: Arc<RtspServerState>, port: u16, app_handle: tauri::AppHandle) {
    use tauri::Emitter;

    eprintln!("[DIAG] MJPEG server task executing, binding port {}...", port);
    tracing::info!("MJPEG server starting, attempting to bind port {}...", port);

    let app = Router::new()
        .route("/stream/:widget_id", get(mjpeg_stream_handler))
        .with_state(state.clone());

    // Try the configured port first with retries, then fallback to alternative ports
    let mut listener = None;
    let mut bound_port = port;

    // Try binding with SO_REUSEADDR to avoid issues with lingering sockets from previous runs
    let ports_to_try: Vec<u16> = std::iter::once(port)
        .chain((1..=5u16).map(|offset| port.wrapping_add(offset)))
        .collect();

    for (idx, try_port) in ports_to_try.iter().enumerate() {
        match bind_with_reuseaddr(*try_port).await {
            Ok(l) => {
                if *try_port == port {
                    eprintln!("[DIAG] MJPEG server bound on 0.0.0.0:{}", try_port);
                    tracing::info!("MJPEG server bound on 0.0.0.0:{}", try_port);
                } else {
                    eprintln!("[DIAG] MJPEG server bound on fallback port {} (original: {})", try_port, port);
                    tracing::info!(
                        "MJPEG server bound on fallback port {} (original: {})",
                        try_port,
                        port
                    );
                }
                bound_port = *try_port;
                listener = Some(l);
                break;
            }
            Err(e) => {
                eprintln!("[DIAG] Failed to bind MJPEG on port {}: {}", try_port, e);
                if idx == 0 {
                    tracing::warn!(
                        "Failed to bind MJPEG server on 0.0.0.0:{}, trying alternatives: {}",
                        try_port,
                        e
                    );
                } else {
                    tracing::warn!("Fallback port {} also unavailable: {}", try_port, e);
                }
            }
        }
    }

    let listener = match listener {
        Some(l) => l,
        None => {
            tracing::error!(
                "MJPEG server failed to start on port {} and fallback ports {}-{}",
                port,
                port + 3,
                port + 5
            );
            let _ = app_handle.emit(
                "mjpeg-server-status",
                serde_json::json!({
                    "running": false,
                    "port": port,
                    "error": format!("Failed to bind port {} and fallback ports {}-{}", port, port + 1, port + 5),
                }),
            );
            return;
        }
    };

    // Mark server as running and notify frontend with actual port
    state.server_running.store(true, Ordering::Relaxed);
    state.actual_port.store(bound_port, Ordering::Relaxed);
    let _ = app_handle.emit(
        "mjpeg-server-status",
        serde_json::json!({
            "running": true,
            "port": bound_port,
            "error": null,
        }),
    );

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("MJPEG server error: {}", e);
        state.server_running.store(false, Ordering::Relaxed);
        let _ = app_handle.emit(
            "mjpeg-server-status",
            serde_json::json!({
                "running": false,
                "port": bound_port,
                "error": format!("Server stopped: {}", e),
            }),
        );
    }
}

/// Bind a TCP listener with SO_REUSEADDR to allow reuse of ports from recently closed sockets.
async fn bind_with_reuseaddr(port: u16) -> std::io::Result<tokio::net::TcpListener> {
    let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    socket.listen(1024)
}

/// Handler for GET /stream/{widget_id}
/// Returns a multipart/x-mixed-replace MJPEG stream.
async fn mjpeg_stream_handler(
    Path(widget_id): Path<String>,
    State(state): State<Arc<RtspServerState>>,
) -> impl IntoResponse {
    // Wait up to 5 seconds for the stream to become available.
    // This handles the race condition where the browser requests the URL
    // before ffmpeg has started producing frames and the stream is registered.
    let tx = {
        let mut found = None;
        for _ in 0..50 {
            let streams = state.streams.read().await;
            if let Some(t) = streams.get(&widget_id) {
                found = Some(t.clone());
                break;
            }
            drop(streams);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        match found {
            Some(tx) => tx,
            None => {
                let current_streams = state.streams.read().await;
                let registered: Vec<&String> = current_streams.keys().collect();
                tracing::warn!(
                    "Stream not found for widget_id={} after 5s wait. \
                     Currently registered streams: {:?}",
                    widget_id,
                    registered
                );
                drop(current_streams);
                return Err((StatusCode::NOT_FOUND, "Stream not found"));
            }
        }
    };
    let mut rx = tx.subscribe();

    let boundary = "mjpegboundary";

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(frame) => {
                    let header_part = format!(
                        "--{}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                        boundary,
                        frame.len()
                    );
                    yield Ok::<Bytes, std::io::Error>(Bytes::from(header_part));
                    yield Ok(frame);
                    yield Ok(Bytes::from("\r\n"));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("MJPEG client for {} lagged, skipped {} frames", widget_id, n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    let body = axum::body::Body::from_stream(stream);

    Ok((
        [
            (
                header::CONTENT_TYPE,
                format!("multipart/x-mixed-replace;boundary={}", boundary),
            ),
            (header::CACHE_CONTROL, "no-cache".to_string()),
            (header::CONNECTION, "keep-alive".to_string()),
        ],
        body,
    ))
}
