<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useMqttStore } from '../../stores/useMqttStore';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();

const min = computed(() => props.config.settings?.min ?? 0);
const max = computed(() => props.config.settings?.max ?? 100);
const step = computed(() => props.config.settings?.step ?? 1);

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
  if (!props.config.topic) return;
  try {
    await mqttStore.publishMessage(props.config.topic, String(currentValue.value));
  } catch (e) {
    console.error('Failed to publish slider value:', e);
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full">
    <input 
      type="range" 
      :min="min" 
      :max="max" 
      :step="step" 
      v-model.number="currentValue"
      @change="publishValue"
      class="range range-primary" 
    />
    <div class="text-lg font-bold mt-2">
      {{ currentValue }}
    </div>
  </div>
</template>
