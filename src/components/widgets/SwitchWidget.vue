<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useMqttStore } from '../../stores/useMqttStore';
import { useToastStore } from '../../stores/useToastStore';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const toastStore = useToastStore();
const isPublishing = ref(false);

const onPayload = computed(() => props.config.settings?.onPayload ?? 'ON');
const offPayload = computed(() => props.config.settings?.offPayload ?? 'OFF');
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const isOn = computed(() => props.message?.payload === onPayload.value);

const toggle = async () => {
  if (!props.config.topic || isPublishing.value) return;
  const payload = isOn.value ? offPayload.value : onPayload.value;
  isPublishing.value = true;
  try {
    await mqttStore.publishMessage(props.config.topic, payload, qos.value, retain.value);
  } catch (e) {
    console.error('Failed to toggle switch:', e);
    toastStore.addToast('Failed to publish message', 'error');
  } finally {
    isPublishing.value = false;
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-4 w-full h-full">
    <input type="checkbox" class="toggle toggle-primary toggle-lg" :class="{ 'opacity-50': isPublishing }"
      :checked="isOn" :disabled="isPublishing" @click.prevent="toggle" />
    <div v-if="message" class="text-xs opacity-50 mt-2">
      {{ message.payload }}
    </div>
  </div>
</template>
