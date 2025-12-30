<script setup lang="ts">
import { GridLayout, GridItem } from 'grid-layout-plus';
import { storeToRefs } from 'pinia';
import { useDashboardStore } from '../../stores/useDashboardStore';
import WidgetHost from './WidgetHost.vue';

const dashboardStore = useDashboardStore();
const { layout, isEditing } = storeToRefs(dashboardStore);
</script>

<template>
  <div class="dashboard-grid">
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
        <WidgetHost :widget="item.widget" />
      </GridItem>
    </GridLayout>
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
