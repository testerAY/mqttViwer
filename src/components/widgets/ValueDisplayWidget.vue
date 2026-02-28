<script setup lang="ts">
import { computed, ref, watch, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { useWidgetData } from '../../composables/useWidgetData';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const { valueKey } = useWidgetData(toRef(props, 'config'));

const lastValue = ref<string | number | null>(null);
const lastTimestamp = ref<number | null>(null);

const parsePayload = (payload: string): string | number => {
  try {
    const json = JSON.parse(payload);
    const val = extractValue(json, valueKey.value);
    if (val === undefined) return 'N/A';
    if (typeof val === 'object') return JSON.stringify(val);
    return val;
  } catch (e) {
    if (!valueKey.value) return payload;
    return 'N/A (Parse Error)';
  }
};

watch(() => props.message, (msg) => {
  if (msg) {
    const parsed = parsePayload(msg.payload);
    if (parsed !== 'N/A' && parsed !== 'N/A (Parse Error)') {
      lastValue.value = parsed;
      lastTimestamp.value = msg.timestamp;
    }
  }
}, { immediate: true });

const displayValue = computed(() => lastValue.value ?? '');
const hasData = computed(() => lastValue.value !== null);
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full">
    <div v-if="hasData" class="text-center w-full">
      <div class="text-3xl font-bold truncate">
        {{ displayValue }}
        <span v-if="config.settings?.unit" class="text-lg font-normal opacity-70 ml-1">
          {{ config.settings.unit }}
        </span>
      </div>
      <div v-if="lastTimestamp" class="text-xs opacity-50 mt-1">
        {{ new Date(lastTimestamp * 1000).toLocaleTimeString() }}
      </div>
    </div>
    <div v-else class="text-center opacity-50">
      Waiting for data...
    </div>
  </div>
</template>
