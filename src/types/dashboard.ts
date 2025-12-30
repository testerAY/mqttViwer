export type WidgetType = 'value-display' | 'switch' | 'chart' | 'gauge';

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  topic?: string;
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
