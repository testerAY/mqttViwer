export type WidgetType = 'value-display' | 'switch' | 'chart' | 'gauge' | 'slider' | 'multi-slider' | 'multi-switch' | '3d-plotter' | string;

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
  mappingId?: string; // New field
  key?: string;
  name?: string;
  color?: string;
  yAxisIndex?: number;
}

export interface DataMapping {
  id: string;
  name: string;
  type: 'sub' | 'pub' | 'both';
  topic: string;
  valueType?: 'value' | 'json';
  valueKey?: string;
  description?: string;
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
