import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import type { DashboardItem, WidgetConfig, DashboardLayout, DataMapping } from '../types/dashboard';

export const useDashboardStore = defineStore('dashboard', () => {
  const defaultLayout: DashboardItem[] = [
    {
      i: '1',
      x: 0,
      y: 0,
      w: 4,
      h: 4,
      widget: {
        id: '1',
        type: 'value-display',
        title: 'Temperature',
        topic: 'sensors/temp',
        settings: { unit: '°C' }
      },
    },
    {
      i: '2',
      x: 4,
      y: 0,
      w: 4,
      h: 4,
      widget: {
        id: '2',
        type: 'value-display',
        title: 'Humidity',
        topic: 'sensors/humidity',
        settings: { unit: '%' }
      },
    },
    {
      i: '3',
      x: 8,
      y: 0,
      w: 4,
      h: 3,
      widget: {
        id: '3',
        type: 'switch',
        title: 'Living Room Light',
        topic: 'home/light/living',
        settings: { onPayload: 'ON', offPayload: 'OFF' }
      },
    },
    {
      i: '4',
      x: 0,
      y: 4,
      w: 8,
      h: 6,
      widget: {
        id: '4',
        type: 'chart',
        title: 'Temperature History',
        topic: 'sensors/temp'
      },
    },
    {
      i: '5',
      x: 8,
      y: 3,
      w: 4,
      h: 4,
      widget: {
        id: '5',
        type: 'gauge',
        title: 'CPU Load',
        topic: 'system/cpu',
        settings: { min: 0, max: 100, unit: '%' }
      },
    },
  ];

  const layout = ref<DashboardItem[]>([...defaultLayout]);
  const dataMappings = ref<DataMapping[]>([]);
  const isEditing = ref(false);
  const currentLayoutPath = ref<string | null>(null);

  const toggleEditMode = () => {
    isEditing.value = !isEditing.value;
  };

  const loadLastLayout = async () => {
    try {
      const path = await invoke<string | null>('get_last_layout_path');
      if (path) {
        const savedData = await invoke<DashboardLayout>('load_layout', { path });
        if (savedData) {
          layout.value = savedData.items || [];
          dataMappings.value = savedData.dataMappings || [];
          currentLayoutPath.value = path;
        }
      }
    } catch (error) {
      console.error('Failed to load last layout:', error);
    }
  };

  const saveLayoutAs = async () => {
    try {
      const path = await save({
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }]
      });
      
      if (path) {
        const fullLayout: DashboardLayout = {
          items: layout.value,
          dataMappings: dataMappings.value
        };
        await invoke('save_layout', { path, layout: fullLayout });
        currentLayoutPath.value = path;
      }
    } catch (error) {
      console.error('Failed to save layout:', error);
    }
  };

  const saveLayout = async () => {
    if (!currentLayoutPath.value) {
      console.warn('No layout path set, cannot auto-save.');
      return;
    }
    try {
      console.log('Saving layout to:', currentLayoutPath.value);
      const fullLayout: DashboardLayout = {
        items: layout.value,
        dataMappings: dataMappings.value
      };
      await invoke('save_layout', { 
        path: currentLayoutPath.value, 
        layout: fullLayout 
      });
      console.log('Layout saved successfully');
    } catch (error) {
      console.error('Failed to save layout:', error);
    }
  };

  const openLayout = async () => {
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }]
      });
      
      if (path) {
        const savedData = await invoke<DashboardLayout>('load_layout', { path });
        if (savedData) {
          layout.value = savedData.items || [];
          dataMappings.value = savedData.dataMappings || [];
          currentLayoutPath.value = path as string;
        }
      }
    } catch (error) {
      console.error('Failed to open layout:', error);
    }
  };

  const updateLayout = (newLayout: DashboardItem[]) => {
    // Just update the state, no auto-save to file
    layout.value = newLayout;
  };

  const updateWidget = (id: string, updates: Partial<WidgetConfig>) => {
    const item = layout.value.find(item => item.widget.id === id);
    if (item) {
      item.widget = { ...item.widget, ...updates };
    }
  };

  const updateLayoutItem = (id: string, updates: Partial<Omit<DashboardItem, 'widget'>>) => {
    const item = layout.value.find(item => item.widget.id === id);
    if (item) {
      if (updates.x !== undefined) item.x = updates.x;
      if (updates.y !== undefined) item.y = updates.y;
      if (updates.w !== undefined) item.w = updates.w;
      if (updates.h !== undefined) item.h = updates.h;
    }
  };

  const removeWidget = (id: string) => {
    layout.value = layout.value.filter(item => item.widget.id !== id);
  };

  const addWidget = async (type: WidgetConfig['type'], x: number, y: number) => {
    const id = crypto.randomUUID();
    let width = 4;
    let height = 4;
    let title = 'New Widget';
    let topic = 'test/topic';
    let settings = {};

    switch (type) {
      case 'value-display':
        title = 'Value Display';
        settings = { unit: '' };
        height = 4;
        break;
      case 'chart':
        title = 'Chart';
        settings = { chartType: 'line', timeMode: 'relative', timeWindow: 60 };
        width = 8;
        height = 6;
        break;
      case 'gauge':
        title = 'Gauge';
        settings = { min: 0, max: 100, unit: '%' };
        break;
      case 'switch':
        title = 'Switch';
        settings = { onPayload: 'ON', offPayload: 'OFF' };
        height = 3;
        break;
      case 'slider':
        title = 'Slider';
        settings = { min: 0, max: 100, step: 1 };
        break;
      case 'plotter':
        title = 'Plotter';
        settings = { timeMode: 'relative', timeWindow: 60 };
        break;
      case 'gantt':
        title = 'Gantt';
        settings = { timeMode: 'relative', timeWindow: 60 };
        break;
      case 'scatter':
        title = 'Scatter';
        settings = {};
        break;
      case 'attitude':
        title = 'Attitude';
        width = 3;
        height = 5;
        settings = { rollKey: 'roll', pitchKey: 'pitch', yawKey: 'yaw' };
        break;
      case 'rtsp-camera':
        title = 'Camera';
        width = 6;
        height = 6;
        topic = undefined as any;
        settings = {
          rtspUrl: '',
          mode: 'passthrough',
          width: 640,
          height: 480,
          fps: 15,
          bitrate: '500k',
          quality: 5,
          rtspTransport: 'tcp',
          reconnectDelaySecs: 3,
          showControls: true,
          autoStart: false,
        };
        break;
    }

    const newItem: DashboardItem = {
      i: id,
      x,
      y,
      w: width,
      h: height,
      widget: {
        id,
        type,
        title,
        topic,
        settings
      }
    };

    console.log('Adding new widget item:', newItem);
    // 配列の再代入を行うことで、変更検知を確実にする
    layout.value = [...layout.value, newItem];
    await saveLayout();
  };

  // Data Mapping Actions
  const addDataMapping = (mapping: Omit<DataMapping, 'id'>) => {
    const id = crypto.randomUUID();
    dataMappings.value.push({ ...mapping, id });
  };
  
  const updateDataMapping = (id: string, updates: Partial<DataMapping>) => {
    const index = dataMappings.value.findIndex(m => m.id === id);
    if (index !== -1) {
      dataMappings.value[index] = { ...dataMappings.value[index], ...updates };
    }
  };

  const removeDataMapping = (id: string) => {
    dataMappings.value = dataMappings.value.filter(m => m.id !== id);
  };
  
  const getDataMappingById = (id: string) => {
    return dataMappings.value.find(m => m.id === id);
  };

  // ★追加: 新規ウィジェットドラッグ中のフラグ
  const isDraggingNewWidget = ref(false);

  // ★追加: フラグ操作用のアクション
  const setDraggingNewWidget = (isDragging: boolean) => {
    isDraggingNewWidget.value = isDragging;
  };

  // Initialize layout
  loadLastLayout();

  return {
    layout,
    dataMappings,
    isEditing,
    currentLayoutPath,
    toggleEditMode,
    updateLayout,
    saveLayoutAs,
    saveLayout,
    openLayout,
    updateWidget,
    updateLayoutItem,
    removeWidget,
    addWidget,
    addDataMapping,
    updateDataMapping,
    removeDataMapping,
    getDataMappingById,
    isDraggingNewWidget,
    setDraggingNewWidget,
  };
});
