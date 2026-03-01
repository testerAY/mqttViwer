<script setup lang="ts">
import { computed, ref, watch, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useWidgetData } from '../../composables/useWidgetData';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const { value: dataValue, lastMessage } = useWidgetData(toRef(props, 'config'));

const lastValue = ref<string | number | null>(null);
const lastTimestamp = ref<number | null>(null);

watch(dataValue, (newVal) => {
  if (newVal !== null && newVal !== undefined) {
    lastValue.value = typeof newVal === 'object' ? JSON.stringify(newVal) : newVal;
    lastTimestamp.value = lastMessage.value?.timestamp ?? null;
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
