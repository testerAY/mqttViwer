import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import { listen } from '@tauri-apps/api/event';

interface MqttMessage {
    topic: string;
    payload: string;
    timestamp: number;
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
    };

    return {
        dataMap,
        isConnected,
        setupListener,
    };
});
