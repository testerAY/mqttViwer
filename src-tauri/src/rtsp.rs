use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use tauri::{Emitter, Manager};

use crate::rtsp_server::RtspServerState;

/// Parameters for starting an RTSP stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamParams {
    pub rtsp_url: String, // In PUSH mode, this acts as the bind address / port. In PULL mode, it's the target URL.
    pub mode: String,     // "pull" (default/legacy) or "push"
    pub width: u32,
    pub height: u32,
    pub fps: u8,
    pub bitrate: String,
    pub quality: u8,
    pub rtsp_transport: String, // "tcp" or "udp"
}

/// Status information for a running stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtspStreamStatus {
    pub widget_id: String,
    pub is_streaming: bool,
    pub is_recording: bool,
    pub recording_path: Option<String>,
    pub rtsp_url: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtspStreamInfo {
    pub widget_id: String,
    pub rtsp_url: String,
    pub is_recording: bool,
    pub mode: String,
}

/// A single active stream entry.
struct StreamEntry {
    params: StreamParams,
    ffmpeg_process: Child,
    reader_handle: JoinHandle<()>,
    stderr_handle: JoinHandle<()>,
    monitor_handle: JoinHandle<()>,
    recording_process: Option<Child>,
    recording_path: Option<PathBuf>,
}

/// Manages all active RTSP streams and their ffmpeg processes.
pub struct RtspServerManager {
    streams: Mutex<HashMap<String, StreamEntry>>,
    ffmpeg_path: Mutex<Option<PathBuf>>,
    server_state: std::sync::Arc<RtspServerState>,
}

impl RtspServerManager {
    pub fn new(server_state: std::sync::Arc<RtspServerState>) -> Self {
        Self {
            streams: Mutex::new(HashMap::new()),
            ffmpeg_path: Mutex::new(None),
            server_state,
        }
    }

    /// Find and cache the ffmpeg binary path.
    pub async fn find_ffmpeg(
        &self,
        config_path: Option<&str>,
        app: &tauri::AppHandle,
    ) -> Result<PathBuf, String> {
        // Check cache first
        {
            let cached = self.ffmpeg_path.lock().await;
            if let Some(ref p) = *cached {
                return Ok(p.clone());
            }
        }

        let path = find_ffmpeg_impl(config_path, app)?;

        // Cache it
        *self.ffmpeg_path.lock().await = Some(path.clone());
        Ok(path)
    }

    /// Start streaming RTSP for a widget. Returns the local MJPEG URL.
    pub async fn start_stream(
        &self,
        widget_id: &str,
        params: StreamParams,
        ffmpeg_path: &Path,
        app_handle: &tauri::AppHandle,
    ) -> Result<String, String> {
        // Check that the MJPEG server is running before starting a stream
        if !self.server_state.is_running() {
            return Err(
                "MJPEG server is not running. Check if port is available and restart the app."
                    .to_string(),
            );
        }

        let mut streams = self.streams.lock().await;

        // Stop existing stream if any
        if let Some(mut entry) = streams.remove(widget_id) {
            let _ = entry.ffmpeg_process.kill().await;
            entry.reader_handle.abort();
            entry.monitor_handle.abort();
            self.server_state.remove_stream(widget_id).await;
        }

        let mut args: Vec<String> = Vec::new();

        if params.mode == "push" {
            // In push mode, ffmpeg acts as an RTSP server, waiting for the camera to push.
            // Use verbose logging to capture RTSP negotiation details for diagnostics.
            args.extend([
                "-loglevel".into(),
                "debug".into(),
                "-rtsp_flags".into(),
                "listen".into(),
                "-listen_timeout".into(),
                "0".into(), // Wait indefinitely for the camera to connect
                "-rtsp_transport".into(),
                params.rtsp_transport.clone(),
                "-i".into(),
                params.rtsp_url.clone(), // e.g. "rtsp://0.0.0.0:18802/stream"
            ]);
        } else {
            // In pull mode, ffmpeg acts as a client
            args.extend([
                "-rtsp_transport".into(),
                params.rtsp_transport.clone(),
                "-i".into(),
                params.rtsp_url.clone(),
            ]);
        }

        // Video filter for resolution and fps
        let vf = format!(
            "scale={}:{},fps={}",
            params.width, params.height, params.fps
        );
        args.extend(["-vf".into(), vf]);

        // Always specify output codec as mjpeg
        args.extend(["-c:v".into(), "mjpeg".into()]);

        if params.mode == "relay" {
            // Re-encode with bitrate control
            args.extend(["-b:v".into(), params.bitrate.clone()]);
        }

        // MJPEG quality and output to stdout
        args.extend([
            "-q:v".into(),
            params.quality.to_string(),
            "-f".into(),
            "mjpeg".into(),
            "-an".into(), // no audio
            "pipe:1".into(),
        ]);

        tracing::info!(
            "Starting ffmpeg for widget {}: {} {:?}",
            widget_id,
            ffmpeg_path.display(),
            args
        );

        let mut child = Command::new(ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture ffmpeg stdout".to_string())?;

        // Capture stderr for diagnostics (shared buffer for error reporting)
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture ffmpeg stderr".to_string())?;

        let stderr_lines = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let stderr_lines_clone = stderr_lines.clone();
        let wid_stderr = widget_id.to_string();
        let stderr_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!("[ffmpeg:{}] {}", wid_stderr, line);
                let mut buf = stderr_lines_clone.lock().await;
                if buf.len() < 100 {
                    buf.push(line);
                }
            }
        });

        // Register the stream in the server and get the broadcast sender
        let frame_capacity = (params.fps as usize).max(5);
        let tx = self
            .server_state
            .add_stream(widget_id, frame_capacity)
            .await;

        // Create a oneshot channel to be notified when the first frame arrives
        let (first_frame_tx, first_frame_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn a task that reads JPEG frames from ffmpeg stdout
        let wid = widget_id.to_string();
        let reader_handle = tokio::spawn(async move {
            read_mjpeg_frames(stdout, tx, &wid, Some(first_frame_tx)).await;
        });

        let is_push = params.mode == "push";

        // Release the streams lock while waiting so other operations aren't blocked.
        drop(streams);

        if is_push {
            // In push mode, give ffmpeg a moment to fail fast (e.g., port already in use).
            // If it exits within 500ms, it almost certainly failed to bind.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    // ffmpeg already exited - this is a startup failure
                    reader_handle.abort();
                    stderr_handle.abort();
                    // Give stderr task a moment to collect output
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    self.server_state.remove_stream(widget_id).await;
                    let stderr_buf = stderr_lines.lock().await;
                    let detail = if stderr_buf.is_empty() {
                        format!(
                            "ffmpeg exited immediately with status {} in push mode. \
                             Port in {} may already be in use.",
                            exit_status, params.rtsp_url
                        )
                    } else {
                        format!(
                            "ffmpeg failed to start in push mode (status {}):\n{}",
                            exit_status,
                            stderr_buf.join("\n")
                        )
                    };
                    return Err(detail);
                }
                Ok(None) => {
                    // Still running - ffmpeg is listening for incoming RTSP push
                    tracing::info!(
                        "Push mode: ffmpeg alive and listening for widget {} (camera will connect later)",
                        widget_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Push mode: could not check ffmpeg status for widget {}: {}",
                        widget_id, e
                    );
                    // Continue anyway - best effort
                }
            }
        } else {
            // In pull mode, wait for the first frame to confirm ffmpeg is working.
            let timeout_secs = 10u64;
            let wait_result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                first_frame_rx,
            )
            .await;

            // Re-acquire the lock for cleanup
            let streams = self.streams.lock().await;

            match wait_result {
                Ok(Ok(())) => {
                    tracing::info!("Stream ready for widget {}", widget_id);
                }
                Ok(Err(_)) => {
                    reader_handle.abort();
                    stderr_handle.abort();
                    self.server_state.remove_stream(widget_id).await;
                    let stderr_buf = stderr_lines.lock().await;
                    let detail = if stderr_buf.is_empty() {
                        "ffmpeg exited without producing any frames".to_string()
                    } else {
                        format!(
                            "ffmpeg exited without producing frames:\n{}",
                            stderr_buf.join("\n")
                        )
                    };
                    return Err(detail);
                }
                Err(_) => {
                    reader_handle.abort();
                    stderr_handle.abort();
                    self.server_state.remove_stream(widget_id).await;
                    let stderr_buf = stderr_lines.lock().await;
                    let detail = if stderr_buf.is_empty() {
                        format!(
                            "Timed out waiting for first frame ({}s). Check RTSP URL and network.",
                            timeout_secs
                        )
                    } else {
                        format!(
                            "Timed out ({}s) waiting for first frame:\n{}",
                            timeout_secs,
                            stderr_buf.join("\n")
                        )
                    };
                    return Err(detail);
                }
            }

            // Release lock again before proceeding
            drop(streams);
        }

        // Use the actual port the MJPEG server is running on (may differ from config)
        let actual_port = self.server_state.get_port();
        let url = format!(
            "http://127.0.0.1:{}/stream/{}?t={}",
            actual_port,
            widget_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        // Spawn a monitor task that waits for the reader to finish (ffmpeg exit),
        // then cleans up the server state and notifies the frontend.
        let monitor_wid = widget_id.to_string();
        let monitor_server_state = self.server_state.clone();
        let monitor_app = app_handle.clone();
        let monitor_reader = reader_handle.abort_handle();
        let monitor_is_push = is_push;
        let monitor_handle = tokio::spawn(async move {
            // Wait for the reader task to complete (i.e., ffmpeg stdout closed)
            // We poll the abort handle; when the reader completes, we act.
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if monitor_reader.is_finished() {
                    // Check if stream was already removed by stop_stream (intentional stop)
                    let still_registered = monitor_server_state
                        .streams
                        .read()
                        .await
                        .contains_key(&monitor_wid);
                    if still_registered {
                        if monitor_is_push {
                            tracing::warn!(
                                "ffmpeg reader finished for push-mode widget {} \
                                 (camera may not have connected or disconnected)",
                                monitor_wid
                            );
                        } else {
                            tracing::warn!(
                                "ffmpeg exited unexpectedly for widget {}, cleaning up",
                                monitor_wid
                            );
                        }
                        monitor_server_state.remove_stream(&monitor_wid).await;
                        let _ = monitor_app.emit("rtsp-stream-died", &monitor_wid);
                    }
                    break;
                }
            }
        });

        let mut streams = self.streams.lock().await;
        streams.insert(
            widget_id.to_string(),
            StreamEntry {
                params,
                ffmpeg_process: child,
                reader_handle,
                stderr_handle,
                monitor_handle,
                recording_process: None,
                recording_path: None,
            },
        );

        Ok(url)
    }

    /// Stop a stream and clean up.
    pub async fn stop_stream(&self, widget_id: &str) -> Result<(), String> {
        let mut streams = self.streams.lock().await;
        if let Some(mut entry) = streams.remove(widget_id) {
            let _ = entry.ffmpeg_process.kill().await;
            entry.reader_handle.abort();
            entry.stderr_handle.abort();
            entry.monitor_handle.abort();
            if let Some(mut rec) = entry.recording_process.take() {
                let _ = rec.kill().await;
            }
            self.server_state.remove_stream(widget_id).await;
            Ok(())
        } else {
            Err("Stream not found".to_string())
        }
    }

    /// Start recording to a file (separate ffmpeg process).
    pub async fn start_recording(
        &self,
        widget_id: &str,
        output_path: PathBuf,
        ffmpeg_path: &Path,
    ) -> Result<String, String> {
        let mut streams = self.streams.lock().await;
        let entry = streams
            .get_mut(widget_id)
            .ok_or_else(|| "Stream not found".to_string())?;

        if entry.recording_process.is_some() {
            return Err("Already recording".to_string());
        }

        // If it's a PUSH stream, we cannot start a secondary process to pull from the same URL,
        // because the source is pushing. In that case, ffmpeg needs to be configured differently
        // or we need to record the MJPEG stream. For now, we warn or return an error if mode is push.
        if entry.params.mode == "push" {
            return Err("Direct recording is not supported for PUSH mode streams yet.".to_string());
        }

        // Spawn a separate ffmpeg to record to MP4
        let args = vec![
            "-rtsp_transport".to_string(),
            entry.params.rtsp_transport.clone(),
            "-i".to_string(),
            entry.params.rtsp_url.clone(),
            "-c:v".to_string(),
            "copy".to_string(),
            "-an".to_string(),
            "-y".to_string(),
            output_path.to_string_lossy().to_string(),
        ];

        let child = Command::new(ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn recording ffmpeg: {}", e))?;

        let path_str = output_path.to_string_lossy().to_string();
        entry.recording_process = Some(child);
        entry.recording_path = Some(output_path);

        Ok(path_str)
    }

    /// Stop recording.
    pub async fn stop_recording(&self, widget_id: &str) -> Result<(), String> {
        let mut streams = self.streams.lock().await;
        let entry = streams
            .get_mut(widget_id)
            .ok_or_else(|| "Stream not found".to_string())?;

        if let Some(mut rec) = entry.recording_process.take() {
            // Send 'q' to gracefully stop ffmpeg recording (allows proper file finalization)
            let _ = rec.kill().await;
            let _ = rec.wait().await;
        }
        entry.recording_path = None;
        Ok(())
    }

    /// Get status of a specific stream.
    pub async fn get_status(&self, widget_id: &str) -> Option<RtspStreamStatus> {
        let streams = self.streams.lock().await;
        streams.get(widget_id).map(|entry| RtspStreamStatus {
            widget_id: widget_id.to_string(),
            is_streaming: true,
            is_recording: entry.recording_process.is_some(),
            recording_path: entry
                .recording_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            rtsp_url: entry.params.rtsp_url.clone(),
            mode: entry.params.mode.clone(),
        })
    }

    /// List all active streams.
    pub async fn list_streams(&self) -> Vec<RtspStreamInfo> {
        let streams = self.streams.lock().await;
        streams
            .iter()
            .map(|(id, entry)| RtspStreamInfo {
                widget_id: id.clone(),
                rtsp_url: entry.params.rtsp_url.clone(),
                is_recording: entry.recording_process.is_some(),
                mode: entry.params.mode.clone(),
            })
            .collect()
    }

    /// Stop all streams (for app shutdown).
    pub async fn stop_all(&self) {
        let mut streams = self.streams.lock().await;
        for (id, mut entry) in streams.drain() {
            let _ = entry.ffmpeg_process.kill().await;
            entry.reader_handle.abort();
            entry.stderr_handle.abort();
            entry.monitor_handle.abort();
            if let Some(mut rec) = entry.recording_process.take() {
                let _ = rec.kill().await;
            }
            self.server_state.remove_stream(&id).await;
        }
    }

}

/// Take a snapshot (single frame) from an RTSP stream.
pub async fn take_snapshot(
    ffmpeg_path: &Path,
    rtsp_url: &str,
    rtsp_transport: &str,
    output_path: &Path,
) -> Result<(), String> {
    let status = Command::new(ffmpeg_path)
        .args([
            "-rtsp_transport",
            rtsp_transport,
            "-i",
            rtsp_url,
            "-frames:v",
            "1",
            "-q:v",
            "2",
            "-y",
            &output_path.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Failed to run ffmpeg snapshot: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "ffmpeg snapshot failed with exit code: {:?}",
            status.code()
        ))
    }
}

/// Find the ffmpeg binary.
fn find_ffmpeg_impl(config_path: Option<&str>, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 1. Explicit config path
    if let Some(p) = config_path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Bundled in Tauri resource dir
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled: PathBuf = resource_dir.join("ffmpeg").join("ffmpeg.exe");
        if bundled.exists() {
            return Ok(bundled);
        }
        // Also try without .exe for non-Windows
        let bundled: PathBuf = resource_dir.join("ffmpeg").join("ffmpeg");
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    // 3. System PATH
    which::which("ffmpeg").map_err(|_| {
        "ffmpeg not found. Please install ffmpeg or set the path in RTSP settings.".to_string()
    })
}

/// Read MJPEG frames from ffmpeg stdout by detecting JPEG SOI/EOI markers.
/// If `first_frame_tx` is provided, it will be sent once the first frame is successfully broadcast.
async fn read_mjpeg_frames(
    mut stdout: tokio::process::ChildStdout,
    tx: broadcast::Sender<Bytes>,
    widget_id: &str,
    mut first_frame_tx: Option<tokio::sync::oneshot::Sender<()>>,
) {
    let mut buf = vec![0u8; 65536];
    let mut frame_buf: Vec<u8> = Vec::with_capacity(65536);
    let mut in_frame = false;
    let mut frame_count: u64 = 0;
    let mut total_bytes: u64 = 0;

    loop {
        match stdout.read(&mut buf).await {
            Ok(0) => {
                tracing::info!(
                    "ffmpeg stdout closed for widget {} (total frames: {}, bytes: {})",
                    widget_id,
                    frame_count,
                    total_bytes
                );
                break;
            }
            Ok(n) => {
                total_bytes += n as u64;
                for i in 0..n {
                    let byte = buf[i];

                    if !in_frame {
                        // Look for JPEG SOI marker: FF D8
                        if frame_buf.len() == 1 && frame_buf[0] == 0xFF && byte == 0xD8 {
                            in_frame = true;
                            frame_buf.clear();
                            frame_buf.push(0xFF);
                            frame_buf.push(0xD8);
                        } else if byte == 0xFF {
                            frame_buf.clear();
                            frame_buf.push(0xFF);
                        } else {
                            frame_buf.clear();
                        }
                    } else {
                        frame_buf.push(byte);

                        // Check for JPEG EOI marker: FF D9
                        if frame_buf.len() >= 2 {
                            let last_two = &frame_buf[frame_buf.len() - 2..];
                            if last_two[0] == 0xFF && last_two[1] == 0xD9 {
                                // Complete frame
                                frame_count += 1;
                                if frame_count == 1 {
                                    tracing::info!(
                                        "First MJPEG frame received for widget {} ({} bytes)",
                                        widget_id,
                                        frame_buf.len()
                                    );
                                    // Notify that the first frame has been received
                                    if let Some(notifier) = first_frame_tx.take() {
                                        let _ = notifier.send(());
                                    }
                                } else if frame_count % 100 == 0 {
                                    tracing::debug!(
                                        "Widget {} frame count: {}",
                                        widget_id,
                                        frame_count
                                    );
                                }
                                let frame = Bytes::from(frame_buf.clone());
                                let _ = tx.send(frame);
                                frame_buf.clear();
                                in_frame = false;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error reading ffmpeg stdout for {}: {}", widget_id, e);
                break;
            }
        }
    }
}
