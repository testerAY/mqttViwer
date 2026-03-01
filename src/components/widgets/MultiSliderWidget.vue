<script setup lang="ts">
import { computed, reactive, watch, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import type { MultiSliderItem } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';
import { useWidgetData } from '../../composables/useWidgetData';
import { encodeProtoPayload, decodeProtoPayload } from '../../utils/protoUtils';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const { topic, valueType, mapping } = useWidgetData(toRef(props, 'config'));

const sliders = computed<MultiSliderItem[]>(() => props.config.settings?.sliders ?? []);
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const values = reactive<Record<string, number>>({});

// スライダー定義変更時にvaluesを初期化（存在しないキーのみ）
watch(sliders, (newSliders) => {
  newSliders.forEach(s => {
    if (!(s.key in values)) {
      values[s.key] = s.defaultValue ?? s.min ?? 0;
    }
  });
  // 削除されたキーをvaluesから除去
  Object.keys(values).forEach(k => {
    if (!newSliders.some(s => s.key === k)) {
      delete values[k];
    }
  });
}, { immediate: true });

// 受信メッセージからvaluesを同期
watch(() => props.message, (msg) => {
  if (!msg) return;

  if (valueType.value === 'binary' && mapping.value?.protoSchema) {
    if (msg.payload_encoding !== 'base64') return;
    try {
      const decoded = decodeProtoPayload(msg.payload, mapping.value.protoSchema);
      sliders.value.forEach(s => {
        if (s.key in decoded) {
          const n = Number(decoded[s.key]);
          if (!isNaN(n)) values[s.key] = n;
        }
      });
    } catch {
      // ignore
    }
    return;
  }

  try {
    const parsed = JSON.parse(msg.payload);
    sliders.value.forEach(s => {
      if (s.key in parsed) {
        const n = Number(parsed[s.key]);
        if (!isNaN(n)) values[s.key] = n;
      }
    });
  } catch {
    // JSON以外は無視
  }
});

const publishAll = async () => {
  if (!topic.value) return;
  try {
    if (valueType.value === 'binary' && mapping.value?.protoSchema) {
      const fieldValues: Record<string, number> = {};
      sliders.value.forEach(s => {
        fieldValues[s.key] = values[s.key] ?? s.defaultValue ?? 0;
      });
      const base64 = encodeProtoPayload(mapping.value.protoSchema, fieldValues);
      await mqttStore.publishBinaryMessage(topic.value, base64, qos.value, retain.value);
    } else {
      const payload: Record<string, number> = {};
      sliders.value.forEach(s => {
        payload[s.key] = values[s.key] ?? s.defaultValue ?? 0;
      });
      await mqttStore.publishMessage(topic.value, JSON.stringify(payload), qos.value, retain.value);
    }
  } catch (e) {
    console.error('Failed to publish multi-slider values:', e);
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col gap-3 p-4 w-full h-full overflow-y-auto">
    <div v-if="sliders.length === 0" class="flex items-center justify-center h-full opacity-50 text-sm text-center">
      キーが未設定です。<br />右上メニューの「Configure Keys」から設定してください。
    </div>
    <div v-for="slider in sliders" :key="slider.key" class="flex flex-col gap-1">
      <div class="flex justify-between items-center text-sm">
        <span class="font-medium">{{ slider.label || slider.key }}</span>
        <span class="font-mono font-bold text-primary">{{ values[slider.key] }}</span>
      </div>
      <input
        type="range"
        :min="slider.min"
        :max="slider.max"
        :step="slider.step"
        v-model.number="values[slider.key]"
        @change="publishAll"
        class="range range-primary range-sm"
      />
      <div class="flex justify-between text-xs opacity-50">
        <span>{{ slider.min }}</span>
        <span>{{ slider.max }}</span>
      </div>
    </div>
  </div>
</template>
