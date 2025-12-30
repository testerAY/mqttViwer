<script setup lang="ts">
import { computed } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';

const props = defineProps<{
  widget: WidgetConfig;
}>();

const mqttStore = useMqttStore();

const message = computed(() => {
  if (!props.widget.topic) return null;
  return mqttStore.dataMap.get(props.widget.topic);
});
</script>

<template>
  <div class="card bg-base-100 w-full h-full shadow-md overflow-hidden flex flex-col">
    <div class="card-body p-4 flex-grow-0">
      <h3 class="card-title text-sm">{{ widget.title }}</h3>
      <div v-if="widget.topic" class="badge badge-ghost badge-xs">{{ widget.topic }}</div>
    </div>
    
    <div class="flex-1 flex items-center justify-center bg-base-200 p-4">
      <div v-if="message" class="text-center">
        <div class="text-3xl font-bold">{{ message.payload }}</div>
        <div class="text-xs opacity-50 mt-1">
          {{ new Date(message.timestamp * 1000).toLocaleTimeString() }}
        </div>
      </div>
      <div v-else class="text-center opacity-50">
        Waiting for data...
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  height: 100%;
}
</style>
