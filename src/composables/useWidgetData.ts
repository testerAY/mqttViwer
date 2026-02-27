import { computed, type Ref } from 'vue';
import { useDashboardStore } from '../stores/useDashboardStore';
import { useMqttStore } from '../stores/useMqttStore';
import type { WidgetConfig } from '../types/dashboard';
import { extractValue } from '../utils/jsonExtractor';

export function useWidgetData(config: Ref<WidgetConfig>) {
  const dashboardStore = useDashboardStore();
  const mqttStore = useMqttStore();

  const mapping = computed(() => {
    if (config.value.mappingId) {
      return dashboardStore.getDataMappingById(config.value.mappingId);
    }
    return null;
  });

  const resolvedTopic = computed(() => {
    if (mapping.value) return mapping.value.topic;
    return config.value.topic || '';
  });

  const resolvedValueKey = computed(() => {
    if (mapping.value) return mapping.value.valueKey || '';
    // Fallback to settings.valueKey if present (legacy)
    return config.value.settings?.valueKey || '';
  });

  const currentValue = computed(() => {
    if (!resolvedTopic.value) return null;

    const msg = mqttStore.lastMessages[resolvedTopic.value];
    if (!msg) return null;

    try {
        const payloadJson = JSON.parse(msg.payload);
        return extractValue(payloadJson, resolvedValueKey.value);
    } catch (e) {
        return msg.payload; // Raw value if not JSON
    }
  });
  
  const lastMessage = computed(() => {
      if (!resolvedTopic.value) return null;
      return mqttStore.lastMessages[resolvedTopic.value];
  });

  return {
    mapping,
    topic: resolvedTopic,
    valueKey: resolvedValueKey,
    value: currentValue,
    lastMessage
  };
}
