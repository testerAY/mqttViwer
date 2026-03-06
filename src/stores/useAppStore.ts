import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface BrokerConfig {
  mode: 'internal' | 'external';
  host: string;
  port: number;
  username?: string;
  password?: string;
  subscription_topic: string;
}

export interface RetentionConfig {
  enabled: boolean;
  days: number;
}

export interface RtspAppConfig {
  ffmpeg_path: string | null;
  server_port: number;
  recordings_dir: string | null;
  snapshots_dir: string | null;
}

export interface AppConfig {
  last_layout_path: string | null;
  broker: BrokerConfig;
  theme: string;
  retention: RetentionConfig;
  rtsp: RtspAppConfig;
}

export const useAppStore = defineStore('app', () => {
  const settings = ref<AppConfig>({
    last_layout_path: null,
    broker: { mode: 'internal', host: '127.0.0.1', port: 9883, subscription_topic: '#', username: undefined, password: undefined },
    theme: 'dark',
    retention: { enabled: true, days: 7 },
    rtsp: {
      ffmpeg_path: null,
      server_port: 18801,
      recordings_dir: null,
      snapshots_dir: null,
    },
  });

  const loadSettings = async () => {
    try {
      const config = await invoke<AppConfig>('get_app_settings');
      settings.value = config;
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
  };

  const saveSettings = async (newSettings: AppConfig) => {
    try {
      await invoke('save_app_settings', { config: newSettings });
      settings.value = newSettings;
    } catch (error) {
      console.error('Failed to save settings:', error);
      throw error;
    }
  };

  return {
    settings,
    loadSettings,
    saveSettings
  };
});
