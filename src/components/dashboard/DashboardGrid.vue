<script setup lang="ts">
import { ref } from 'vue';
import { GridLayout, GridItem } from 'grid-layout-plus';
import { storeToRefs } from 'pinia';
import { useDashboardStore } from '../../stores/useDashboardStore';
import type { WidgetConfig } from '../../types/dashboard';
import WidgetHost from './WidgetHost.vue';
import WidgetSettingsModal from './WidgetSettingsModal.vue';

const dashboardStore = useDashboardStore();
const { layout, isEditing } = storeToRefs(dashboardStore);

const settingsModalOpen = ref(false);
const currentWidgetId = ref<string | null>(null);
const gridContainer = ref<HTMLElement | null>(null);

const handleEditWidget = (id: string) => {
  currentWidgetId.value = id;
  settingsModalOpen.value = true;
};

const handleDragOver = (event: DragEvent) => {
  if (!isEditing.value) return;
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }
};

const handleDrop = (event: DragEvent) => {
  if (!isEditing.value) return;
  
  // Stop propagation to prevent multiple drops if handled by both container and grid
  event.preventDefault();
  event.stopPropagation();
  
  console.log('Drop event detected');
  const type = event.dataTransfer?.getData('widget-type');
  console.log('Widget type:', type);
  
  if (type && gridContainer.value) {
    const rect = gridContainer.value.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    // Grid calculation
    const colNum = 12;
    const colWidth = rect.width / colNum;
    const rowHeight = 70; // 60px height + 10px margin
    
    const gridX = Math.floor(x / colWidth);
    const gridY = Math.floor(y / rowHeight);
    
    // Clamp values
    const safeX = Math.max(0, Math.min(colNum - 1, gridX));
    const safeY = Math.max(0, gridY);

    console.log(`Adding widget at: ${safeX}, ${safeY}`);
    dashboardStore.addWidget(type as WidgetConfig['type'], safeX, safeY);
  }
};

const handleRemoveWidget = (id: string) => {
  if (confirm('Are you sure you want to remove this widget?')) {
    dashboardStore.removeWidget(id);
  }
};
</script>

<template>
  <div 
    ref="gridContainer"
    class="dashboard-grid"
    @dragover="handleDragOver"
    @drop="handleDrop"
  >
    <GridLayout
      v-model:layout="layout"
      :col-num="12"
      :row-height="60"
      :is-draggable="isEditing"
      :is-resizable="isEditing"
      :vertical-compact="true"
      :use-css-transforms="true"
    >
      <GridItem
        v-for="item in layout"
        :key="item.i"
        :x="item.x"
        :y="item.y"
        :w="item.w"
        :h="item.h"
        :i="item.i"
        class="dashboard-item"
        :class="{ 'editing': isEditing }"
      >
        <WidgetHost 
          :widget="item.widget" 
          @edit="handleEditWidget"
          @remove="handleRemoveWidget"
        />
      </GridItem>
    </GridLayout>

    <WidgetSettingsModal
      :open="settingsModalOpen"
      :widget-id="currentWidgetId"
      @close="settingsModalOpen = false"
    />
  </div>
</template>

<style scoped>
.dashboard-grid {
  width: 100%;
  /* Ensure minimum height for drop area */
  min-height: calc(100vh - 100px); 
}

.dashboard-item {
  /* Add transition for smooth movement */
  transition: box-shadow 0.2s;
}

.dashboard-item.editing {
  /* Visual cue for edit mode */
  border: 1px dashed rgba(255, 255, 255, 0.2);
  background-color: rgba(255, 255, 255, 0.05);
  cursor: move;
}

/* Customizing placeholder */
:deep(.vgl-item--placeholder) {
  background: rgba(255, 255, 255, 0.2) !important;
  border-radius: 0.5rem;
  opacity: 0.5;
}
</style>
