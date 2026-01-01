export interface MqttMessage {
    topic: string;
    payload: string;
    timestamp: number;
    data_type?: string;
    value_num?: number;
}

export interface MqttStatus {
    status: string;
}
