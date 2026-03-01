export type WidgetType = 'value-display' | 'switch' | 'chart' | 'gauge' | 'slider' | 'multi-slider' | 'multi-switch' | '3d-plotter' | 'rtsp-camera' | string;

export interface RtspWidgetSettings {
  rtspUrl: string;
  mode: 'passthrough' | 'relay';
  width: number;
  height: number;
  fps: number;
  bitrate: string;
  quality: number;
  rtspTransport: 'tcp' | 'udp';
  reconnectDelaySecs: number;
  showControls: boolean;
  autoStart: boolean;
}

export interface MultiSliderItem {
  key: string;
  label: string;
  min: number;
  max: number;
  step: number;
  defaultValue: number;
}

export interface MultiSwitchItem {
  key: string;
  label: string;
}

export interface DataSeries {
  topic: string;
  mappingId?: string;
  key?: string;
  fieldName?: string;
  name?: string;
  color?: string;
  yAxisIndex?: number;
}

export type ProtoFieldType =
  | 'double' | 'float'
  | 'int32' | 'int64' | 'uint32' | 'uint64'
  | 'sint32' | 'sint64'
  | 'fixed32' | 'fixed64' | 'sfixed32' | 'sfixed64'
  | 'bool' | 'string' | 'bytes';

export type ProtoFieldRule = 'required' | 'optional' | 'repeated';

export interface ProtoField {
  id: number;
  name: string;
  type: ProtoFieldType;
  rule: ProtoFieldRule;
}

export interface ProtoMessage {
  name: string;
  fields: ProtoField[];
}

export interface ProtoSchema {
  packageName?: string;
  rootMessage: string;
  messages: ProtoMessage[];
}

export interface DataMapping {
  id: string;
  name: string;
  type: 'sub' | 'pub' | 'both';
  topic: string;
  valueType?: 'value' | 'json' | 'binary';
  valueKey?: string;
  description?: string;
  protoSchema?: ProtoSchema;
}

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  topic?: string;
  mappingId?: string;
  updateInterval?: number; // ms
  settings?: Record<string, any>;
}

export interface DashboardItem {
  x: number;
  y: number;
  w: number;
  h: number;
  i: string;
  widget: WidgetConfig;
}

export interface DashboardLayout {
  items: DashboardItem[];
  dataMappings: DataMapping[];
}
