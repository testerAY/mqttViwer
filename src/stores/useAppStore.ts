import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface BrokerConfig {
  mode: 'internal' | 'external';
  host: string;
  port: number;
}

export interface RetentionConfig {
  enabled: boolean;
  days: number;
}

export interface AppConfig {
  last_layout_path: string | null;
  broker: BrokerConfig;
  theme: string;
  retention: RetentionConfig;
}

export const useAppStore = defineStore('app', () => {
  const settings = ref<AppConfig>({
    last_layout_path: null,
    broker: { mode: 'internal', host: '127.0.0.1', port: 9883 },
    theme: 'dark',
    retention: { enabled: true, days: 7 }
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
