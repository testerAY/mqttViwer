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
    // settings.valueKey is an explicit override - always takes priority
    if (config.value.settings?.valueKey) return config.value.settings.valueKey;
    if (mapping.value) return mapping.value.valueKey || '';
    return '';
  });

  const resolvedValueType = computed(() => {
    if (mapping.value) return mapping.value.valueType || 'json';
    return 'json';
  });

  const currentValue = computed(() => {
    if (!resolvedTopic.value) return null;

    const msg = mqttStore.lastMessages[resolvedTopic.value];
    if (!msg) return null;

    if (resolvedValueType.value === 'value') {
        return msg.payload;
    }

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
    valueType: resolvedValueType,
    value: currentValue,
    lastMessage
  };
}
