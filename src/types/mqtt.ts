export interface MqttMessage {
    topic: string;
    payload: string;
    timestamp: number;
    data_type?: string;
    value_num?: number;
    payload_encoding?: 'utf8' | 'base64';
}

export interface MqttStatus {
    status: string;
}
