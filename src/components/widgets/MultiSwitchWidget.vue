<script setup lang="ts">
import { computed, ref, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import type { MultiSwitchItem } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';
import { useToastStore } from '../../stores/useToastStore';
import { useWidgetData } from '../../composables/useWidgetData';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const toastStore = useToastStore();
const { topic } = useWidgetData(toRef(props, 'config'));

const switches = computed<MultiSwitchItem[]>(() => props.config.settings?.switches ?? []);
const qos = computed(() => props.config.settings?.qos ?? 0);
const retain = computed(() => props.config.settings?.retain ?? false);

const isPublishing = ref(false);

// SwitchWidget と同様に props.message から表示状態を computed で導出（ローカル状態を持たない）
const displayStates = computed<Record<string, boolean>>(() => {
  const result: Record<string, boolean> = {};
  switches.value.forEach(s => { result[s.key] = false; });

  if (!props.message) return result;

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

  // 現在の表示状態を元に新しいペイロードを構築（指定キーだけ反転）
  const payload: Record<string, number> = {};
  switches.value.forEach(s => {
    const current = displayStates.value[s.key] ?? false;
    payload[s.key] = (s.key === key ? !current : current) ? 1 : 0;
  });

  isPublishing.value = true;
  try {
    await mqttStore.publishMessage(topic.value, JSON.stringify(payload), qos.value, retain.value);
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
      キーが未設定です。<br />右上メニューの「Configure Keys」から設定してください。
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
