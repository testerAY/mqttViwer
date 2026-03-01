<script setup lang="ts">
import { ref, computed, watch, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useMqttStore } from '../../stores/useMqttStore';
import { useWidgetData } from '../../composables/useWidgetData';
import { extractValue, buildValue } from '../../utils/jsonExtractor';
import { encodeProtoPayload, decodeProtoPayload } from '../../utils/protoUtils';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();

const { topic, valueKey, valueType, fieldName, mapping } = useWidgetData(toRef(props, 'config'));

const min = computed(() => props.config.settings?.min ?? 0);
const max = computed(() => props.config.settings?.max ?? 100);
const step = computed(() => props.config.settings?.step ?? 1);
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const currentValue = ref(props.message?.payload ?? min.value);

watch(() => props.message, (newMessage) => {
  if (newMessage) {
    let rawVal: any = newMessage.payload;
    if (valueType.value === 'binary' && mapping.value?.protoSchema && fieldName.value) {
      if (newMessage.payload_encoding === 'base64') {
        try {
          const decoded = decodeProtoPayload(newMessage.payload, mapping.value.protoSchema);
          rawVal = decoded[fieldName.value];
        } catch {
          return;
        }
      }
    } else if (valueType.value === 'json' && valueKey.value) {
      rawVal = extractValue(newMessage.payload, valueKey.value);
    }
    const numValue = parseFloat(String(rawVal));
    if (!isNaN(numValue)) {
      currentValue.value = numValue;
    }
  }
});

const buildPayload = (val: any): string => {
  if (valueType.value !== 'json' || !valueKey.value) return String(val);
  return JSON.stringify(buildValue(valueKey.value, val));
};

const publishValue = async () => {
  if (!topic.value) return;
  try {
    if (valueType.value === 'binary' && mapping.value?.protoSchema && fieldName.value) {
      const base64 = encodeProtoPayload(mapping.value.protoSchema, { [fieldName.value]: currentValue.value });
      await mqttStore.publishBinaryMessage(topic.value, base64, qos.value, retain.value);
    } else {
      await mqttStore.publishMessage(topic.value, buildPayload(currentValue.value), qos.value, retain.value);
    }
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
