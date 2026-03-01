use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use tauri::Manager;

use crate::rtsp_server::RtspServerState;

/// Parameters for starting an RTSP stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamParams {
    pub rtsp_url: String,
    pub mode: String, // "passthrough" or "relay"
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtspStreamInfo {
    pub widget_id: String,
    pub rtsp_url: String,
    pub is_recording: bool,
}

/// A single active stream entry.
struct StreamEntry {
    params: StreamParams,
    ffmpeg_process: Child,
    reader_handle: JoinHandle<()>,
    recording_process: Option<Child>,
    recording_path: Option<PathBuf>,
}

/// Manages all active RTSP streams and their ffmpeg processes.
pub struct RtspManager {
    streams: Mutex<HashMap<String, StreamEntry>>,
    ffmpeg_path: Mutex<Option<PathBuf>>,
}

impl RtspManager {
    pub fn new() -> Self {
        Self {
            streams: Mutex::new(HashMap::new()),
            ffmpeg_path: Mutex::new(None),
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
        server_state: &RtspServerState,
        server_port: u16,
    ) -> Result<String, String> {
        let mut streams = self.streams.lock().await;

        // Stop existing stream if any
        if let Some(mut entry) = streams.remove(widget_id) {
            let _ = entry.ffmpeg_process.kill().await;
            entry.reader_handle.abort();
            server_state.remove_stream(widget_id).await;
        }

        // Build ffmpeg command
        let mut args: Vec<String> = vec![
            "-rtsp_transport".into(),
            params.rtsp_transport.clone(),
            "-i".into(),
            params.rtsp_url.clone(),
        ];

        // Video filter for resolution and fps
        let vf = format!("scale={}:{},fps={}", params.width, params.height, params.fps);
        args.extend(["-vf".into(), vf]);

        if params.mode == "relay" {
            // Re-encode with bitrate control
            args.extend([
                "-c:v".into(),
                "mjpeg".into(),
                "-b:v".into(),
                params.bitrate.clone(),
            ]);
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
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture ffmpeg stdout".to_string())?;

        // Register the stream in the server and get the broadcast sender
        let frame_capacity = (params.fps as usize).max(5);
        let tx = server_state.add_stream(widget_id, frame_capacity).await;

        // Spawn a task that reads JPEG frames from ffmpeg stdout
        let wid = widget_id.to_string();
        let reader_handle = tokio::spawn(async move {
            read_mjpeg_frames(stdout, tx, &wid).await;
        });

        let url = format!("http://127.0.0.1:{}/stream/{}", server_port, widget_id);

        streams.insert(
            widget_id.to_string(),
            StreamEntry {
                params,
                ffmpeg_process: child,
                reader_handle,
                recording_process: None,
                recording_path: None,
            },
        );

        Ok(url)
    }

    /// Stop a stream and clean up.
    pub async fn stop_stream(
        &self,
        widget_id: &str,
        server_state: &RtspServerState,
    ) -> Result<(), String> {
        let mut streams = self.streams.lock().await;
        if let Some(mut entry) = streams.remove(widget_id) {
            let _ = entry.ffmpeg_process.kill().await;
            entry.reader_handle.abort();
            if let Some(mut rec) = entry.recording_process.take() {
                let _ = rec.kill().await;
            }
            server_state.remove_stream(widget_id).await;
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
            })
            .collect()
    }

    /// Stop all streams (for app shutdown).
    pub async fn stop_all(&self, server_state: &RtspServerState) {
        let mut streams = self.streams.lock().await;
        for (id, mut entry) in streams.drain() {
            let _ = entry.ffmpeg_process.kill().await;
            entry.reader_handle.abort();
            if let Some(mut rec) = entry.recording_process.take() {
                let _ = rec.kill().await;
            }
            server_state.remove_stream(&id).await;
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
async fn read_mjpeg_frames(
    mut stdout: tokio::process::ChildStdout,
    tx: broadcast::Sender<Bytes>,
    widget_id: &str,
) {
    let mut buf = vec![0u8; 65536];
    let mut frame_buf: Vec<u8> = Vec::with_capacity(65536);
    let mut in_frame = false;

    loop {
        match stdout.read(&mut buf).await {
            Ok(0) => {
                tracing::info!("ffmpeg stdout closed for widget {}", widget_id);
                break;
            }
            Ok(n) => {
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
                                let frame = Bytes::from(frame_buf.clone());
                                let _ = tx.send(frame); // ignore if no receivers
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
