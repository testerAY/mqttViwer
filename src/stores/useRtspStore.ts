import { defineStore } from 'pinia';
import { reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { RtspWidgetSettings } from '../types/dashboard';

export interface RtspStreamState {
  url: string | null;
  isConnecting: boolean;
  isStreaming: boolean;
  isRecording: boolean;
  recordingPath: string | null;
  error: string | null;
}

export interface MjpegServerStatus {
  running: boolean;
  port: number;
  error: string | null;
}

export const useRtspStore = defineStore('rtsp', () => {
  const streams = reactive<Record<string, RtspStreamState>>({});
  const serverStatus = reactive<MjpegServerStatus>({
    running: false,
    port: 0,
    error: null,
  });

  function getOrCreate(widgetId: string): RtspStreamState {
    if (!streams[widgetId]) {
      streams[widgetId] = {
        url: null,
        isConnecting: false,
        isStreaming: false,
        isRecording: false,
        recordingPath: null,
        error: null,
      };
    }
    return streams[widgetId];
  }

  async function startStream(widgetId: string, settings: RtspWidgetSettings): Promise<string> {
    const state = getOrCreate(widgetId);
    state.isConnecting = true;
    state.error = null;

    try {
      const url = await invoke<string>('rtsp_start_stream', {
        widgetId,
        params: {
          rtspUrl: settings.rtspUrl,
          mode: settings.mode,
          width: settings.width,
          height: settings.height,
          fps: settings.fps,
          bitrate: settings.bitrate,
          quality: settings.quality,
          rtspTransport: settings.rtspTransport,
        }
      });
      state.url = url;
      state.isStreaming = true;
      state.isConnecting = false;
      return url;
    } catch (e: any) {
      state.error = String(e);
      state.url = null;
      state.isStreaming = false;
      state.isConnecting = false;
      throw e;
    }
  }

  async function stopStream(widgetId: string): Promise<void> {
    try {
      await invoke('rtsp_stop_stream', { widgetId });
    } catch (e) {
      // Ignore errors on stop (stream may already be gone)
    }
    const state = streams[widgetId];
    if (state) {
      state.url = null;
      state.isStreaming = false;
      state.isConnecting = false;
      state.isRecording = false;
      state.recordingPath = null;
    }
  }

  async function startRecording(widgetId: string, outputPath?: string): Promise<string> {
    const state = getOrCreate(widgetId);
    try {
      const path = await invoke<string>('rtsp_start_recording', {
        widgetId,
        outputPath: outputPath || null,
      });
      state.isRecording = true;
      state.recordingPath = path;
      return path;
    } catch (e: any) {
      state.error = String(e);
      throw e;
    }
  }

  async function stopRecording(widgetId: string): Promise<void> {
    try {
      await invoke('rtsp_stop_recording', { widgetId });
    } catch (e) {
      // Ignore
    }
    const state = streams[widgetId];
    if (state) {
      state.isRecording = false;
      state.recordingPath = null;
    }
  }

  async function takeSnapshot(
    widgetId: string,
    rtspUrl: string,
    rtspTransport: string,
    outputPath?: string,
  ): Promise<string> {
    return invoke<string>('rtsp_take_snapshot', {
      widgetId,
      rtspUrl,
      rtspTransport,
      outputPath: outputPath || null,
    });
  }

  async function checkFfmpeg(): Promise<string> {
    return invoke<string>('rtsp_check_ffmpeg');
  }

  function cleanup(widgetId: string): void {
    delete streams[widgetId];
  }

  // Listen for MJPEG server status events
  listen<MjpegServerStatus>('mjpeg-server-status', (event) => {
    console.log('[RTSP] MJPEG server status:', event.payload);
    serverStatus.running = event.payload.running;
    serverStatus.port = event.payload.port;
    serverStatus.error = event.payload.error;
  });

  // Listen for backend event when ffmpeg process dies unexpectedly
  listen<string>('rtsp-stream-died', (event) => {
    const widgetId = event.payload;
    const state = streams[widgetId];
    if (state) {
      state.url = null;
      state.isStreaming = false;
      state.isConnecting = false;
      state.isRecording = false;
      state.recordingPath = null;
      state.error = 'Stream disconnected (ffmpeg exited)';
    }
  });

  return {
    streams,
    serverStatus,
    startStream,
    stopStream,
    startRecording,
    stopRecording,
    takeSnapshot,
    checkFfmpeg,
    cleanup,
    getOrCreate,
  };
});
