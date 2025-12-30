<script setup lang="ts">
import { computed } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useMqttStore } from '../../stores/useMqttStore';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();

const onPayload = computed(() => props.config.settings?.onPayload ?? 'ON');
const offPayload = computed(() => props.config.settings?.offPayload ?? 'OFF');

const isOn = computed(() => props.message?.payload === onPayload.value);

const toggle = async () => {
  if (!props.config.topic) return;
  const payload = isOn.value ? offPayload.value : onPayload.value;
  try {
    await mqttStore.publishMessage(props.config.topic, payload);
  } catch (e) {
    console.error('Failed to toggle switch:', e);
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full">
    <input 
      type="checkbox" 
      class="toggle toggle-primary toggle-lg" 
      :checked="isOn" 
      @click.prevent="toggle"
    />
    <div v-if="message" class="text-xs opacity-50 mt-2">
      {{ message.payload }}
    </div>
  </div>
</template>
