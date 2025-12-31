import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import type { DashboardItem } from '../types/dashboard';

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
  const isEditing = ref(false);
  const currentLayoutPath = ref<string | null>(null);

  const toggleEditMode = () => {
    isEditing.value = !isEditing.value;
  };

  const loadLastLayout = async () => {
    try {
      const path = await invoke<string | null>('get_last_layout_path');
      if (path) {
        const savedLayout = await invoke<DashboardItem[]>('load_layout', { path });
        if (savedLayout && savedLayout.length > 0) {
          layout.value = savedLayout;
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
        await invoke('save_layout', { path, layout: layout.value });
        currentLayoutPath.value = path;
      }
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
        const savedLayout = await invoke<DashboardItem[]>('load_layout', { path });
        if (savedLayout && savedLayout.length > 0) {
          layout.value = savedLayout;
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

  // Initialize layout
  loadLastLayout();

  return {
    layout,
    isEditing,
    currentLayoutPath,
    toggleEditMode,
    updateLayout,
    saveLayoutAs,
    openLayout,
  };
});
