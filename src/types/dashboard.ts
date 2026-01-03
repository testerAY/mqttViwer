export type WidgetType = 'value-display' | 'switch' | 'chart' | 'gauge' | 'slider' | string;

export interface DataSeries {
  topic: string;
  key?: string;
  name?: string;
  color?: string;
  yAxisIndex?: number;
}

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  topic?: string;
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
}
