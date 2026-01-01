<script setup lang="ts">
import { ref } from 'vue';
import { GridLayout, GridItem } from 'grid-layout-plus';
import { storeToRefs } from 'pinia';
import { useDashboardStore } from '../../stores/useDashboardStore';
import type { WidgetConfig } from '../../types/dashboard';
import WidgetHost from './WidgetHost.vue';
import WidgetSettingsModal from './WidgetSettingsModal.vue';
import { ask } from '@tauri-apps/plugin-dialog';

const dashboardStore = useDashboardStore();
// isDraggingNewWidget を確実に取得
const { layout, isEditing, isDraggingNewWidget } = storeToRefs(dashboardStore);

const settingsModalOpen = ref(false);
const currentWidgetId = ref<string | null>(null);
const gridContainer = ref<HTMLElement | null>(null);
const isDragOver = ref(false); // ドラッグ中フラグを追加

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
const handleDrop = async (event: DragEvent) => {
  if (!isEditing.value) return;
  
  event.preventDefault();
  event.stopPropagation();

  try {
    const type = event.dataTransfer?.getData('widget-type') || event.dataTransfer?.getData('text/plain');
    console.log('Drop event detected. Type:', type);
    
    if (type && gridContainer.value) {
      const rect = gridContainer.value.getBoundingClientRect();
      const x = event.clientX - rect.left;
      const y = event.clientY - rect.top;
      
      const colNum = 12; // Grid Layoutの列数に合わせる
      const colWidth = rect.width / colNum;
      const rowHeight = 60; // GridItemのrow-heightに合わせる
      
      const gridX = Math.floor(x / colWidth);
      const gridY = Math.floor(y / rowHeight);

      console.log(`Drop coordinates: client(${event.clientX}, ${event.clientY}) -> local(${x}, ${y}) -> grid(${gridX}, ${gridY})`);

      // ★ ウィジェットタイプごとの幅を定義（Storeの設定と合わせる）
      let widgetWidth = 4; // デフォルト
      if (type === 'chart') {
        widgetWidth = 8;
      }
      // switchなどは4でOK、必要ならcase追加

      // ★ はみ出さない最大X座標を計算 (12 - 幅)
      const maxX = colNum - widgetWidth;
      
      // safeX を「0 ～ maxX」の範囲に収める
      const safeX = Math.max(0, Math.min(maxX, gridX));
      const safeY = Math.max(0, gridY);

      console.log(`Adding widget at: ${safeX}, ${safeY} (Width: ${widgetWidth})`);
      await dashboardStore.addWidget(type as WidgetConfig['type'], safeX, safeY);
      console.log('Widget added via store');
      
      dashboardStore.setDraggingNewWidget(false);
    } else {
      console.warn('Drop failed: Missing type or gridContainer');
    }
  } catch (error) {
    console.error('Error in handleDrop:', error);
    dashboardStore.setDraggingNewWidget(false);
  }
};

const handleRemoveWidget = async (id: string) => {
  const yes = await ask('Are you sure you want to remove this widget?',{
    title: 'Confirm Removal',
    kind: 'warning'
  });
  if (yes) {
    dashboardStore.removeWidget(id);
  }
};

// ドラッグがコンテナに入った時
const handleDragEnter = (event: DragEvent) => {
  if (!isEditing.value) return;
  event.preventDefault(); // WebView対策：必須
  isDragOver.value = true;
};

// ドラッグがコンテナから出た時
const handleDragLeave = (event: DragEvent) => {
  if (!isEditing.value) return;
  
  // 関連ターゲット（移動先）がコンテナ内であればフラグを維持
  if (gridContainer.value && gridContainer.value.contains(event.relatedTarget as Node)) {
    return;
  }
  isDragOver.value = false;
};
</script>

<template>
  <div 
    ref="gridContainer"
    class="dashboard-grid"
    @dragover="handleDragOver"
    @drop="handleDrop"
    @dragenter="handleDragEnter"
    @dragleave="handleDragLeave"
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
  height: 100%; 
  position: relative; /* オーバーレイの絶対配置の基準点 */
}

/* ドロップ専用オーバーレイ */
.drop-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999; /* 最前面に表示 */
  background-color: rgba(var(--p), 0.1); /* Primary color with transparency */
  border: 4px dashed rgba(var(--p), 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none; /* イベントを親に透過させる */
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
