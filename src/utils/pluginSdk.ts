import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { MqttMessage } from '../types/mqtt';
import type { MqttViewerSDK } from '../types/plugin';

export async function initPluginSdk() {
  const subscribers = new Map<string, Set<(topic: string, message: string) => void>>();

  // Listen to global MQTT messages
  await listen<MqttMessage>('mqtt-message', (event) => {
    const { topic, payload } = event.payload;
    const callbacks = subscribers.get(topic);
    if (callbacks) {
      callbacks.forEach(cb => {
        try {
          cb(topic, payload);
        } catch (e) {
          console.error(`Error in plugin callback for topic ${topic}:`, e);
        }
      });
    }
  });

  const sdk: MqttViewerSDK = {
    subscribe(topic: string, callback: (topic: string, message: string) => void) {
      if (!subscribers.has(topic)) {
        subscribers.set(topic, new Set());
      }
      subscribers.get(topic)!.add(callback);
    },
    unsubscribe(topic: string, callback: (topic: string, message: string) => void) {
      const callbacks = subscribers.get(topic);
      if (callbacks) {
        callbacks.delete(callback);
        if (callbacks.size === 0) {
          subscribers.delete(topic);
        }
      }
    },
    async publish(topic: string, message: string) {
      await invoke('publish_message', { topic, payload: message });
    },
    getTheme() {
      // Simple implementation for now
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
  };

  window.MqttViewerSDK = sdk;
}
