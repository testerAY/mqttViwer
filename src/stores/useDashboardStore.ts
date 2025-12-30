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
