<script setup lang="ts">
import { computed, ref, watch, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useWidgetData } from '../../composables/useWidgetData';
import { useToastStore } from '../../stores/useToastStore';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const { value: dataValue, lastMessage } = useWidgetData(toRef(props, 'config'));
const toastStore = useToastStore();

const lastValue = ref<string | number | null>(null);
const lastTimestamp = ref<number | null>(null);
// Track previous alert state to avoid repeated toasts
const prevAlertState = ref<'ok' | 'min' | 'max'>('ok');

watch(dataValue, (newVal) => {
  if (newVal !== null && newVal !== undefined) {
    lastValue.value = typeof newVal === 'object' ? JSON.stringify(newVal) : newVal;
    lastTimestamp.value = lastMessage.value?.timestamp ?? null;
  }
}, { immediate: true });

const displayValue = computed(() => lastValue.value ?? '');
const hasData = computed(() => lastValue.value !== null);

const numericValue = computed(() => {
  const n = parseFloat(String(lastValue.value));
  return isNaN(n) ? null : n;
});

const minThreshold = computed(() => {
  const v = props.config.settings?.minThreshold;
  return v !== undefined && v !== '' ? Number(v) : null;
});
const maxThreshold = computed(() => {
  const v = props.config.settings?.maxThreshold;
  return v !== undefined && v !== '' ? Number(v) : null;
});

const alertState = computed<'ok' | 'min' | 'max'>(() => {
  if (numericValue.value === null) return 'ok';
  if (minThreshold.value !== null && numericValue.value < minThreshold.value) return 'min';
  if (maxThreshold.value !== null && numericValue.value > maxThreshold.value) return 'max';
  return 'ok';
});

// Fire toast when threshold is first crossed (debounced to avoid spam)
let alertDebounceTimer: ReturnType<typeof setTimeout> | null = null;
const ALERT_DEBOUNCE_MS = 2000;

watch(alertState, (newState, oldState) => {
  if (newState === oldState) return;
  prevAlertState.value = newState;

  if (newState === 'ok') {
    // Clear pending alert if returning to ok
    if (alertDebounceTimer) { clearTimeout(alertDebounceTimer); alertDebounceTimer = null; }
    return;
  }

  // Debounce alert toasts to prevent rapid fire near threshold boundary
  if (alertDebounceTimer) clearTimeout(alertDebounceTimer);
  alertDebounceTimer = setTimeout(() => {
    alertDebounceTimer = null;
    // Re-check state after debounce (may have recovered)
    if (alertState.value === 'ok') return;
    const title = props.config.title || 'Widget';
    if (alertState.value === 'min') {
      toastStore.addToast(`${title}: ${displayValue.value} is below minimum (${minThreshold.value})`, 'error', 5000);
    } else if (alertState.value === 'max') {
      toastStore.addToast(`${title}: ${displayValue.value} exceeds maximum (${maxThreshold.value})`, 'error', 5000);
    }
  }, ALERT_DEBOUNCE_MS);
});
</script>

<template>
  <div
    class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full transition-colors duration-300"
    :class="{
      'bg-error/20 text-error-content rounded-lg': alertState !== 'ok'
    }"
  >
    <div v-if="hasData" class="text-center w-full">
      <div class="text-3xl font-bold truncate flex items-center justify-center gap-2">
        <span
          v-if="alertState !== 'ok'"
          class="text-error text-2xl"
          title="Threshold exceeded"
        >⚠</span>
        <span :class="alertState !== 'ok' ? 'text-error' : ''">
          {{ displayValue }}
        </span>
        <span v-if="config.settings?.unit" class="text-lg font-normal opacity-70 ml-1">
          {{ config.settings.unit }}
        </span>
      </div>
      <div v-if="alertState === 'min'" class="text-xs text-error mt-1 font-medium">
        Below minimum ({{ minThreshold }})
      </div>
      <div v-else-if="alertState === 'max'" class="text-xs text-error mt-1 font-medium">
        Exceeds maximum ({{ maxThreshold }})
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
