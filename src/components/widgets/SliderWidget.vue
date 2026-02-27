<script setup lang="ts">
import { ref, computed, watch, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useMqttStore } from '../../stores/useMqttStore';
import { useWidgetData } from '../../composables/useWidgetData';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();

const { topic } = useWidgetData(toRef(props, 'config'));

const min = computed(() => props.config.settings?.min ?? 0);
const max = computed(() => props.config.settings?.max ?? 100);
const step = computed(() => props.config.settings?.step ?? 1);
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const currentValue = ref(props.message?.payload ?? min.value);

watch(() => props.message, (newMessage) => {
  if (newMessage) {
    const numValue = parseFloat(newMessage.payload);
    if (!isNaN(numValue)) {
      currentValue.value = numValue;
    }
  }
});

const publishValue = async () => {
  if (!topic.value) return;
  try {
    await mqttStore.publishMessage(topic.value, String(currentValue.value), qos.value, retain.value);
  } catch (e) {
    console.error('Failed to publish slider value:', e);
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full">
    <input type="range" :min="min" :max="max" :step="step" v-model.number="currentValue" @change="publishValue"
      class="range range-primary" />
    <div class="text-lg font-bold mt-2">
      {{ currentValue }}
    </div>
  </div>
</template>
