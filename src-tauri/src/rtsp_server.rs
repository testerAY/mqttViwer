use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Shared state for the MJPEG HTTP server.
/// Maps widget_id -> broadcast sender of JPEG frames.
#[derive(Clone)]
pub struct RtspServerState {
    pub streams: Arc<RwLock<HashMap<String, broadcast::Sender<Bytes>>>>,
}

impl RtspServerState {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new stream and return the broadcast sender for pushing frames.
    pub async fn add_stream(&self, widget_id: &str, capacity: usize) -> broadcast::Sender<Bytes> {
        let (tx, _) = broadcast::channel(capacity);
        let tx_clone = tx.clone();
        self.streams
            .write()
            .await
            .insert(widget_id.to_string(), tx);
        tx_clone
    }

    /// Remove a stream entry.
    pub async fn remove_stream(&self, widget_id: &str) {
        self.streams.write().await.remove(widget_id);
    }
}

/// Start the Axum MJPEG HTTP server on the given port.
pub async fn start(state: RtspServerState, port: u16) {
    let app = Router::new()
        .route("/stream/{widget_id}", get(mjpeg_stream_handler))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    tracing::info!("RTSP MJPEG server starting on http://{}", addr);

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind MJPEG server on {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("MJPEG server error: {}", e);
    }
}

/// Handler for GET /stream/{widget_id}
/// Returns a multipart/x-mixed-replace MJPEG stream.
async fn mjpeg_stream_handler(
    Path(widget_id): Path<String>,
    State(state): State<RtspServerState>,
) -> impl IntoResponse {
    let streams = state.streams.read().await;
    let tx = match streams.get(&widget_id) {
        Some(tx) => tx.clone(),
        None => {
            return Err((StatusCode::NOT_FOUND, "Stream not found"));
        }
    };
    let mut rx = tx.subscribe();
    drop(streams);

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
