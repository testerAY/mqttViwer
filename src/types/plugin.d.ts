export interface MqttViewerSDK {
  subscribe(topic: string, callback: (topic: string, message: string) => void): void;
  unsubscribe(topic: string, callback: (topic: string, message: string) => void): void;
  publish(topic: string, message: string): Promise<void>;
  getTheme(): 'light' | 'dark' | 'system';
}

export interface PluginConfigField {
  name: string;
  label: string;
  type: 'text' | 'number' | 'checkbox' | 'select' | 'color';
  defaultValue?: any;
  options?: { label: string; value: any }[];
  min?: number;
  max?: number;
  description?: string;
}

export interface PluginCapabilities {
  requiresTopic?: boolean;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description?: string;
  main: string;
  tagName: string;
  capabilities?: PluginCapabilities;
  configSchema?: PluginConfigField[];
}

declare global {
  interface Window {
    MqttViewerSDK: MqttViewerSDK;
  }
}
