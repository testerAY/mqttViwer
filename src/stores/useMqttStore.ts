import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

import type { MqttMessage, MqttStatus } from '../types/mqtt';

export const useMqttStore = defineStore('mqtt', () => {
    const dataMap = shallowRef<Map<string, MqttMessage>>(new Map());
    const isConnected = ref(false);
    const lastMessages = ref<Record<string, MqttMessage>>({});

    const setupListener = async () => {
        await listen<MqttMessage>('mqtt-message', (event) => {
            const newMessage = event.payload;
            
            // For reactivity in DashboardGrid
            const newMap = new Map(dataMap.value);
            newMap.set(newMessage.topic, newMessage);
            dataMap.value = newMap;
            
            // For preview in settings
            lastMessages.value[newMessage.topic] = newMessage;
        });

        await listen<MqttStatus>('mqtt-status', (event) => {
            console.log('MQTT Status:', event.payload.status);
            isConnected.value = event.payload.status === 'connected';
        });
    };

    const publishMessage = async (topic: string, payload: string, qos: number = 0, retain: boolean = false) => {
        try {
            await invoke('publish_message', { topic, payload, qos, retain });
        } catch (error) {
            console.error('Failed to publish message:', error);
            // fallback for simulation
            console.log(`[Simulation] Published to ${topic}: ${payload} (QoS: ${qos}, Retain: ${retain})`);
            
            // Update local state for simulation
            const msg: MqttMessage = {
                topic,
                payload,
                timestamp: Date.now() / 1000
            };
            const newMap = new Map(dataMap.value);
            newMap.set(topic, msg);
            dataMap.value = newMap;
            lastMessages.value[topic] = msg;
        }
    };

    const startSimulation = () => {
        console.log('Starting simulation...');
        const topics = ['sensors/temp', 'sensors/humidity', 'system/cpu'];
        setInterval(() => {
            const topic = topics[Math.floor(Math.random() * topics.length)];
            let value = 0;
            if (topic.includes('temp')) value = 20 + Math.random() * 10;
            else if (topic.includes('humidity')) value = 40 + Math.random() * 20;
            else if (topic.includes('cpu')) value = Math.random() * 100;
            
            const msg: MqttMessage = {
                topic,
                payload: value.toFixed(1),
                timestamp: Date.now() / 1000
            };
            
            console.log(`[Simulation] Update ${topic}: ${msg.payload}`);
            const newMap = new Map(dataMap.value);
            newMap.set(topic, msg);
            dataMap.value = newMap;
            lastMessages.value[topic] = msg;
        }, 1000);
    };

    const getHistory = async (topicFilter?: string, limit?: number): Promise<MqttMessage[]> => {
        try {
            return await invoke<MqttMessage[]>('get_history', { topicFilter, limit });
        } catch (error) {
            console.error('Failed to get history:', error);
            return [];
        }
    };

    return {
        dataMap,
        isConnected,
        lastMessages,
        setupListener,
        publishMessage,
        startSimulation,
        getHistory,
    };
});
