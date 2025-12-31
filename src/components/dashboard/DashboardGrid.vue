<script setup lang="ts">
import { ref } from 'vue';
import { GridLayout, GridItem } from 'grid-layout-plus';
import { storeToRefs } from 'pinia';
import { useDashboardStore } from '../../stores/useDashboardStore';
import type { WidgetConfig } from '../../types/dashboard';
import WidgetHost from './WidgetHost.vue';
import WidgetSettingsModal from './WidgetSettingsModal.vue';

const dashboardStore = useDashboardStore();
// isDraggingNewWidget を確実に取得
const { layout, isEditing, isDraggingNewWidget } = storeToRefs(dashboardStore);

const settingsModalOpen = ref(false);
const currentWidgetId = ref<string | null>(null);
const gridContainer = ref<HTMLElement | null>(null);

const handleEditWidget = (id: string) => {
  currentWidgetId.value = id;
  settingsModalOpen.value = true;
};

// ★ オーバーレイ上でのドラッグオーバー処理
const handleDragOver = (event: DragEvent) => {
  if (!isEditing.value) return;
  // ドロップを許可するために必須
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }
};

// ★ オーバーレイ上でのドロップ処理
const handleDrop = (event: DragEvent) => {
  if (!isEditing.value) return;
  
  event.preventDefault();
  event.stopPropagation();
  
  const type = event.dataTransfer?.getData('widget-type') || event.dataTransfer?.getData('text/plain');
  
  // gridContainer (親div) を基準に座標計算
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

    dashboardStore.addWidget(type as WidgetConfig['type'], safeX, safeY);
    
    // ドロップ完了後、ドラッグ状態を強制解除（念のため）
    dashboardStore.setDraggingNewWidget(false);
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
  >
    <div :style="{ opacity: isDraggingNewWidget ? 0.5 : 1, transition: 'opacity 0.2s' }">
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
    </div>

    <div 
      v-if="isDraggingNewWidget"
      class="drop-overlay"
      @dragover="handleDragOver"
      @drop="handleDrop"
    >
      <div class="drop-message">
        <span class="text-xl font-bold">+ Drop Widget Here</span>
      </div>
    </div>

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
  min-height: calc(100vh - 100px); 
  position: relative; /* オーバーレイの絶対配置の基準点 */
}

/* ドロップ専用オーバーレイ */
.drop-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 9999; /* 最前面に表示 */
  background-color: rgba(var(--p), 0.1); /* Primary color with transparency */
  border: 4px dashed rgba(var(--p), 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: auto; /* イベントを確実に受け取る */
}

.drop-message {
  background: rgba(0, 0, 0, 0.7);
  color: white;
  padding: 1rem 2rem;
  border-radius: 1rem;
  pointer-events: none; /* 文字自体が邪魔しないように */
}

.dashboard-item {
  transition: box-shadow 0.2s;
}

.dashboard-item.editing {
  border: 1px dashed rgba(255, 255, 255, 0.2);
  background-color: rgba(255, 255, 255, 0.05);
  cursor: move;
}

:deep(.vgl-item--placeholder) {
  background: rgba(255, 255, 255, 0.2) !important;
  border-radius: 0.5rem;
  opacity: 0.5;
}
</style>
