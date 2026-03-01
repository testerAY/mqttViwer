import { computed, type Ref } from 'vue';
import { useDashboardStore } from '../stores/useDashboardStore';
import { useMqttStore } from '../stores/useMqttStore';
import type { WidgetConfig } from '../types/dashboard';
import { extractValue } from '../utils/jsonExtractor';
import { decodeProtoPayload } from '../utils/protoUtils';

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

  const resolvedFieldName = computed(() => {
    if (resolvedValueType.value !== 'binary') return '';
    if (config.value.settings?.fieldName) return config.value.settings.fieldName;
    return '';
  });

  // Memoize protobuf decode to avoid redundant work on every computed access
  let lastDecodedPayload = '';
  let lastDecodedResult: Record<string, any> | null = null;

  const currentValue = computed(() => {
    if (!resolvedTopic.value) return null;

    const msg = mqttStore.lastMessages[resolvedTopic.value];
    if (!msg) return null;

    if (resolvedValueType.value === 'value') {
      return msg.payload;
    }

    if (resolvedValueType.value === 'binary') {
      const schema = mapping.value?.protoSchema;
      if (!schema) return null;

      // If payload is not base64 (e.g. simulation mode), fall back to JSON path
      if (msg.payload_encoding !== 'base64') {
        try {
          return extractValue(JSON.parse(msg.payload), resolvedValueKey.value);
        } catch {
          return msg.payload;
        }
      }

      try {
        if (msg.payload !== lastDecodedPayload) {
          lastDecodedResult = decodeProtoPayload(msg.payload, schema);
          lastDecodedPayload = msg.payload;
        }
        if (!lastDecodedResult) return null;
        const key = resolvedFieldName.value || resolvedValueKey.value;
        return extractValue(lastDecodedResult, key);
      } catch (e) {
        console.warn('[useWidgetData] Protobuf decode error:', e);
        return null;
      }
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
    fieldName: resolvedFieldName,
    valueType: resolvedValueType,
    value: currentValue,
    lastMessage,
  };
}
