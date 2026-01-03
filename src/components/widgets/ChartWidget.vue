<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import type { WidgetConfig, DataSeries } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { LineChart, BarChart } from 'echarts/charts';
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent
} from 'echarts/components';
import VChart from 'vue-echarts';

use([
  CanvasRenderer,
  LineChart,
  BarChart,
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent
]);

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const maxPoints = 50;

interface ChartPoint {
  value: number;
  timestamp: number; // ms
}

// Store history per series index
const seriesHistory = ref<Record<number, ChartPoint[]>>({});

// Computed Normalized Series Definitions
const seriesDefs = computed<DataSeries[]>(() => {
  if (props.config.settings?.series && props.config.settings.series.length > 0) {
    return props.config.settings.series;
  }
  // Fallback / Backward Compatibility
  return [{
    topic: '', // Global
    key: props.config.settings?.valueKey,
    name: props.config.title,
    color: undefined
  }];
});

const getPointFromMessage = (msg: MqttMessage, valueKey?: string, xKey?: string): ChartPoint | null => {
  // Extract Y Value
  const rawVal = extractValue(msg.payload, valueKey || props.config.settings?.yKey);
  const val = parseFloat(rawVal);
  if (isNaN(val)) return null;

  // Extract X Value (Time)
  let timestamp = msg.timestamp * 1000; // Default to message timestamp (ms)

  if (xKey) {
    const rawTime = extractValue(msg.payload, xKey);
    if (rawTime !== undefined) {
      const dateObj = new Date(rawTime);
      if (!isNaN(dateObj.getTime())) {
        // It's a valid date/time
        // Check if it's a numeric timestamp (seconds vs ms)
        const numVal = Number(rawTime);
        if (!isNaN(numVal)) {
          // Heuristic: if < 10 billion, assume seconds, else ms
          if (numVal < 10000000000) {
            timestamp = numVal * 1000;
          } else {
            timestamp = numVal;
          }
        } else {
          timestamp = dateObj.getTime();
        }
      }
    }
  }

  return {
    value: val,
    timestamp: timestamp
  };
};

const processMessage = (msg: MqttMessage, topic: string) => {
  seriesDefs.value.forEach((s, idx) => {
    // Determine target topic for this series
    const targetTopic = s.topic || props.config.topic;

    if (targetTopic === topic) {
      const pt = getPointFromMessage(msg, s.key, props.config.settings?.xKey);
      if (pt) {
        if (!seriesHistory.value[idx]) seriesHistory.value[idx] = [];
        const arr = seriesHistory.value[idx];
        arr.push(pt);
        // Keep buffer size limited
        if (arr.length > maxPoints) {
          seriesHistory.value[idx] = arr.slice(-maxPoints);
        }
      }
    }
  });
};

// Watch Global Topic (Throttled via props)
watch(() => props.message, (newMsg) => {
  if (newMsg && props.config.topic) {
    processMessage(newMsg, props.config.topic);
  }
});

// Watch Extra Topics (Directly from Store)
const extraTopics = computed(() => {
  const set = new Set<string>();
  seriesDefs.value.forEach(s => {
    if (s.topic && s.topic !== props.config.topic) {
      set.add(s.topic);
    }
  });
  return Array.from(set);
});

watch(() => extraTopics.value.map(t => mqttStore.lastMessages[t]), (newMsgs, oldMsgs) => {
  newMsgs.forEach((msg, i) => {
    const topic = extraTopics.value[i];
    const oldMsg = oldMsgs ? oldMsgs[i] : undefined;

    // Only process if it's a new message
    if (msg && (!oldMsg || msg.timestamp !== oldMsg.timestamp)) {
      processMessage(msg, topic);
    }
  });
});

// Load Initial History
onMounted(async () => {
  const topicsToFetch = new Set<string>();
  if (props.config.topic) topicsToFetch.add(props.config.topic);
  extraTopics.value.forEach(t => topicsToFetch.add(t));

  for (const topic of topicsToFetch) {
    if (!topic) continue;
    try {
      const msgs = await mqttStore.getHistory(topic, maxPoints);
      // History comes Newest First (DESC). Reverse to Chronological.
      [...msgs].reverse().forEach(msg => processMessage(msg, topic));
    } catch (e) {
      console.error(`Failed to load history for ${topic}:`, e);
    }
  }
});

// Chart Option
const option = computed(() => {
  const chartType = props.config.settings?.chartType || 'line';

  const seriesOptions = seriesDefs.value.map((s, idx) => {
    const data = seriesHistory.value[idx] || [];
    return {
      name: s.name || s.key || `Series ${idx + 1}`,
      type: chartType,
      // Map to [x, y] format for 'time' axis
      data: data.map(d => [d.timestamp, d.value]),
      itemStyle: s.color ? { color: s.color } : undefined,
      showSymbol: false,
      smooth: chartType === 'line',
      areaStyle: chartType === 'line' ? { opacity: 0.1 } : undefined,
      barMaxWidth: chartType === 'bar' ? 20 : undefined,
    };
  });

  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross' }
    },
    legend: {
      show: true,
      top: 0
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      top: '15%', // Make room for legend
      containLabel: true
    },
    xAxis: {
      type: 'time',
      boundaryGap: chartType === 'bar', // Bars need gap
      splitLine: { show: false }
    },
    yAxis: {
      type: 'value',
      scale: true,
      splitLine: { show: true, lineStyle: { type: 'dashed' } }
    },
    series: seriesOptions,
    animationDuration: 300
  };
});
</script>

<template>
  <div class="w-full h-full p-2">
    <v-chart class="chart" :option="option" autoresize />
  </div>
</template>

<style scoped>
.chart {
  height: 100%;
  width: 100%;
}
</style>
