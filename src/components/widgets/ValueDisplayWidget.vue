<script setup lang="ts">
import { computed } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const displayValue = computed(() => {
  if (!props.message) return '';
  const val = extractValue(props.message.payload, props.config.settings?.valueKey);
  
  if (val === undefined) return 'N/A';
  if (typeof val === 'object') return JSON.stringify(val);
  return val;
});
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full">
    <div v-if="message" class="text-center w-full">
      <div class="text-3xl font-bold truncate">
        {{ displayValue }}
        <span v-if="config.settings?.unit" class="text-lg font-normal opacity-70 ml-1">
          {{ config.settings.unit }}
        </span>
      </div>
      <div class="text-xs opacity-50 mt-1">
        {{ new Date(message.timestamp * 1000).toLocaleTimeString() }}
      </div>
    </div>
    <div v-else class="text-center opacity-50">
      Waiting for data...
    </div>
  </div>
</template>
