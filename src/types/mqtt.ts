export interface MqttMessage {
    topic: string;
    payload: string;
    timestamp: number;
}

export interface MqttStatus {
    status: string;
}
