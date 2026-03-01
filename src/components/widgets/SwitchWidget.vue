<script setup lang="ts">
import { computed, ref, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useMqttStore } from '../../stores/useMqttStore';
import { useToastStore } from '../../stores/useToastStore';
import { useWidgetData } from '../../composables/useWidgetData';
import { extractValue, buildValue } from '../../utils/jsonExtractor';
import { encodeProtoPayload } from '../../utils/protoUtils';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const toastStore = useToastStore();
const isPublishing = ref(false);

const { topic, valueKey, valueType, fieldName, mapping } = useWidgetData(toRef(props, 'config'));

const onPayload = computed(() => props.config.settings?.onPayload ?? '1');
const offPayload = computed(() => props.config.settings?.offPayload ?? '0');
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const isOn = computed(() => {
  if (!props.message) return false;
  let val: any = props.message.payload;
  if (valueType.value === 'json' && valueKey.value) {
    val = extractValue(props.message.payload, valueKey.value);
  }
  return String(val) === onPayload.value;
});

const buildPayload = (val: string): string => {
  if (valueType.value !== 'json' || !valueKey.value) return val;
  return JSON.stringify(buildValue(valueKey.value, val));
};

const toggle = async () => {
  if (!topic.value || isPublishing.value) return;
  const rawPayload = isOn.value ? offPayload.value : onPayload.value;
  isPublishing.value = true;
  try {
    if (valueType.value === 'binary' && mapping.value?.protoSchema && fieldName.value) {
      const base64 = encodeProtoPayload(mapping.value.protoSchema, { [fieldName.value]: rawPayload });
      await mqttStore.publishBinaryMessage(topic.value, base64, qos.value, retain.value);
    } else {
      await mqttStore.publishMessage(topic.value, buildPayload(rawPayload), qos.value, retain.value);
    }
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
      {{ isOn ? 'ON' : 'OFF' }}
    </div>
  </div>
</template>
