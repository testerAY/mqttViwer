import { defineStore } from 'pinia';
import { ref, shallowRef, triggerRef } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

import type { MqttMessage, MqttStatus } from '../types/mqtt';

export const useMqttStore = defineStore('mqtt', () => {
    const dataMap = shallowRef<Map<string, MqttMessage>>(new Map());
    const isConnected = ref(false);
    const lastMessages = ref<Record<string, MqttMessage>>({});

    // B1: Connection quality tracking
    const messageRate = ref(0);        // msgs/sec
    const lastMessageTime = ref(0);    // timestamp ms
    const connectionStale = ref(false); // true if no message for N seconds
    let _msgCount = 0;
    const STALE_THRESHOLD_MS = 5000; // 5 seconds without message = stale

    const _startConnectionTracking = () => {
        // Calculate message rate every second
        setInterval(() => {
            messageRate.value = _msgCount;
            _msgCount = 0;
        }, 1000);
        // Check for stale connection every second
        setInterval(() => {
            if (lastMessageTime.value > 0 && isConnected.value) {
                connectionStale.value = (Date.now() - lastMessageTime.value) > STALE_THRESHOLD_MS;
            } else {
                connectionStale.value = false;
            }
        }, 1000);
    };

    const setupListener = async () => {
        _startConnectionTracking();

        await listen<MqttMessage>('mqtt-message', (event) => {
            const newMessage = event.payload;

            // B1: Track reception
            _msgCount++;
            lastMessageTime.value = Date.now();

            // Mutate existing Map and trigger shallow reactivity (avoids full Map copy)
            dataMap.value.set(newMessage.topic, newMessage);
            triggerRef(dataMap);

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
            dataMap.value.set(topic, msg);
            triggerRef(dataMap);
            lastMessages.value[topic] = msg;
        }
    };

    const publishBinaryMessage = async (topic: string, base64Payload: string, qos: number = 0, retain: boolean = false) => {
        try {
            await invoke('publish_binary_message', { topic, base64Payload, qos, retain });
        } catch (error) {
            console.error('Failed to publish binary message:', error);
            console.log(`[Simulation] Published binary to ${topic} (QoS: ${qos}, Retain: ${retain})`);
            // Update local state for simulation
            const msg: MqttMessage = {
                topic,
                payload: base64Payload,
                timestamp: Date.now() / 1000,
                payload_encoding: 'base64',
            };
            dataMap.value.set(topic, msg);
            triggerRef(dataMap);
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
            _msgCount++;
            lastMessageTime.value = Date.now();
            dataMap.value.set(topic, msg);
            triggerRef(dataMap);
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
        // B1: Connection quality
        messageRate,
        lastMessageTime,
        connectionStale,
        setupListener,
        publishMessage,
        publishBinaryMessage,
        startSimulation,
        getHistory,
    };
});
