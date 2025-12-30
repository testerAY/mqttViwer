import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface MqttMessage {
    topic: string;
    payload: string;
    timestamp: number;
}

interface MqttStatus {
    status: string;
}

export const useMqttStore = defineStore('mqtt', () => {
    const dataMap = shallowRef<Map<string, MqttMessage>>(new Map());
    const isConnected = ref(false); // This will be updated later

    const setupListener = async () => {
        await listen<MqttMessage>('mqtt-message', (event) => {
            const newMessage = event.payload;
            const newMap = new Map(dataMap.value);
            newMap.set(newMessage.topic, newMessage);
            dataMap.value = newMap;
        });

        await listen<MqttStatus>('mqtt-status', (event) => {
            console.log('MQTT Status:', event.payload.status);
            isConnected.value = event.payload.status === 'connected';
        });
    };

    const publishMessage = async (topic: string, payload: string) => {
        try {
            await invoke('publish_message', { topic, payload });
        } catch (error) {
            console.error('Failed to publish message:', error);
            throw error;
        }
    };

    return {
        dataMap,
        isConnected,
        setupListener,
        publishMessage,
    };
});
