export interface MqttViewerSDK {
  subscribe(topic: string, callback: (topic: string, message: string) => void): void;
  unsubscribe(topic: string, callback: (topic: string, message: string) => void): void;
  publish(topic: string, message: string): Promise<void>;
  getTheme(): 'light' | 'dark' | 'system';
}

declare global {
  interface Window {
    MqttViewerSDK: MqttViewerSDK;
  }
}
