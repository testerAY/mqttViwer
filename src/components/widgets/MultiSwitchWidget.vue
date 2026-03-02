<script setup lang="ts">
import { computed, ref, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import type { MultiSwitchItem } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';
import { useToastStore } from '../../stores/useToastStore';
import { useWidgetData } from '../../composables/useWidgetData';
import { encodeProtoPayload, decodeProtoPayload } from '../../utils/protoUtils';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const toastStore = useToastStore();
const { topic, valueType, mapping } = useWidgetData(toRef(props, 'config'));

const switches = computed<MultiSwitchItem[]>(() => props.config.settings?.switches ?? []);
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const isPublishing = ref(false);

// SwitchWidget と同様に props.message から表示状態を computed で導出（ローカル状態を持たない）
const displayStates = computed<Record<string, boolean>>(() => {
  const result: Record<string, boolean> = {};
  switches.value.forEach(s => { result[s.key] = false; });

  if (!props.message) return result;

  if (valueType.value === 'binary' && mapping.value?.protoSchema) {
    if (props.message.payload_encoding !== 'base64') return result;
    try {
      const decoded = decodeProtoPayload(props.message.payload, mapping.value.protoSchema);
      switches.value.forEach(s => {
        if (s.key in decoded) result[s.key] = Boolean(decoded[s.key]);
      });
    } catch {
      // ignore
    }
    return result;
  }

  try {
    const parsed = JSON.parse(props.message.payload);
    switches.value.forEach(s => {
      if (s.key in parsed) {
        result[s.key] = parsed[s.key] === 1 || parsed[s.key] === true;
      }
    });
  } catch {
    // JSON以外は無視
  }
  return result;
});

const toggle = async (key: string) => {
  if (!topic.value || isPublishing.value) return;

  isPublishing.value = true;
  try {
    if (valueType.value === 'binary' && mapping.value?.protoSchema) {
      const fieldValues: Record<string, boolean> = {};
      switches.value.forEach(s => {
        const current = displayStates.value[s.key] ?? false;
        fieldValues[s.key] = s.key === key ? !current : current;
      });
      const base64 = encodeProtoPayload(mapping.value.protoSchema, fieldValues);
      await mqttStore.publishBinaryMessage(topic.value, base64, qos.value, retain.value);
    } else {
      // 現在の表示状態を元に新しいペイロードを構築（指定キーだけ反転）
      const payload: Record<string, number> = {};
      switches.value.forEach(s => {
        const current = displayStates.value[s.key] ?? false;
        payload[s.key] = (s.key === key ? !current : current) ? 1 : 0;
      });
      await mqttStore.publishMessage(topic.value, JSON.stringify(payload), qos.value, retain.value);
    }
  } catch (e) {
    console.error('Failed to publish multi-switch values:', e);
    toastStore.addToast('Failed to publish message', 'error');
  } finally {
    isPublishing.value = false;
  }
};
</script>

<template>
  <div class="flex-1 flex flex-col gap-3 p-4 w-full h-full overflow-y-auto">
    <div v-if="switches.length === 0" class="flex items-center justify-center h-full opacity-50 text-sm text-center">
      No keys configured.<br />Use "Configure Keys" from the ⋮ menu (top-right).
    </div>
    <div
      v-for="sw in switches"
      :key="sw.key"
      class="flex items-center justify-between gap-4"
    >
      <span class="font-medium text-sm">{{ sw.label || sw.key }}</span>
      <div class="flex items-center gap-2">
        <span class="text-xs opacity-50">{{ displayStates[sw.key] ? 'ON' : 'OFF' }}</span>
        <input
          type="checkbox"
          class="toggle toggle-primary"
          :class="{ 'opacity-50': isPublishing }"
          :checked="displayStates[sw.key]"
          :disabled="isPublishing"
          @click.prevent="toggle(sw.key)"
        />
      </div>
    </div>
  </div>
</template>
