import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { DashboardItem } from '../types/dashboard';

export const useDashboardStore = defineStore('dashboard', () => {
  const layout = ref<DashboardItem[]>([
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
  ]);

  const isEditing = ref(false);

  const toggleEditMode = () => {
    isEditing.value = !isEditing.value;
  };

  const updateLayout = (newLayout: DashboardItem[]) => {
    // grid-layout-plus modifies the array in place, so we might not need to reassign if binding directly.
    // However, explicit update can be good for persistence later.
    layout.value = newLayout;
  };

  return {
    layout,
    isEditing,
    toggleEditMode,
    updateLayout,
  };
});
