<script setup lang="ts">
import { computed, defineAsyncComponent } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';

const props = defineProps<{
  widget: WidgetConfig;
}>();

const emit = defineEmits<{
  (e: 'edit', id: string): void;
  (e: 'remove', id: string): void;
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
  <div class="card bg-base-100 w-full h-full shadow-md overflow-hidden flex flex-col group">
    <div class="card-body p-2 flex-grow-0 flex-row justify-between items-start min-h-[3rem]">
      <div class="overflow-hidden">
        <h3 class="card-title text-sm truncate" :title="widget.title">{{ widget.title }}</h3>
        <div v-if="widget.topic" class="badge badge-ghost badge-xs truncate max-w-full" :title="widget.topic">{{ widget.topic }}</div>
      </div>
      
      <div class="dropdown dropdown-end opacity-0 group-hover:opacity-100 transition-opacity">
        <div tabindex="0" role="button" class="btn btn-ghost btn-xs btn-circle">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="inline-block w-4 h-4 stroke-current"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"></path></svg>
        </div>
        <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
          <li><a @click="emit('edit', widget.id)">Settings</a></li>
          <li><a>Export CSV (Coming Soon)</a></li>
          <li><a @click="emit('remove', widget.id)" class="text-error">Remove</a></li>
        </ul>
      </div>
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
