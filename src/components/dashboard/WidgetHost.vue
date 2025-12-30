<script setup lang="ts">
import { computed, defineAsyncComponent } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';

const props = defineProps<{
  widget: WidgetConfig;
}>();

const mqttStore = useMqttStore();

const message = computed(() => {
  if (!props.widget.topic) return undefined;
  return mqttStore.dataMap.get(props.widget.topic);
});

const ValueDisplayWidget = defineAsyncComponent(() => import('../widgets/ValueDisplayWidget.vue'));
const SwitchWidget = defineAsyncComponent(() => import('../widgets/SwitchWidget.vue'));
const ChartWidget = defineAsyncComponent(() => import('../widgets/ChartWidget.vue'));
const GaugeWidget = defineAsyncComponent(() => import('../widgets/GaugeWidget.vue'));

const widgetComponent = computed(() => {
  switch (props.widget.type) {
    case 'value-display':
      return ValueDisplayWidget;
    case 'switch':
      return SwitchWidget;
    case 'chart':
      return ChartWidget;
    case 'gauge':
      return GaugeWidget;
    default:
      return null;
  }
});
</script>

<template>
  <div class="card bg-base-100 w-full h-full shadow-md overflow-hidden flex flex-col">
    <div class="card-body p-4 flex-grow-0">
      <h3 class="card-title text-sm">{{ widget.title }}</h3>
      <div v-if="widget.topic" class="badge badge-ghost badge-xs">{{ widget.topic }}</div>
    </div>
    
    <div class="flex-1 overflow-hidden relative">
      <component 
        v-if="widgetComponent"
        :is="widgetComponent"
        :config="widget"
        :message="message"
      />
      <div v-else class="flex items-center justify-center h-full opacity-50">
        Widget type "{{ widget.type }}" not implemented yet
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  height: 100%;
}
</style>
