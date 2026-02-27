<script setup lang="ts">
import { ref, watch, computed, onMounted, toRef, onUnmounted } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import { useDashboardStore } from '../../stores/useDashboardStore';
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
import { useWidgetData } from '../../composables/useWidgetData';

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
  clearToken?: number;
}>();

const mqttStore = useMqttStore();
const dashboardStore = useDashboardStore();

const { topic: globalTopic, valueKey: globalValueKey } = useWidgetData(toRef(props, 'config'));

interface ChartPoint {
  value: number;
  timestamp: number; // ms
}

const timeWindow = computed(() => (props.config.settings?.timeWindow || 60) * 1000);

// Store history per series index
const seriesHistory = ref<Record<number, ChartPoint[]>>({});

watch(() => props.clearToken, (token, prev) => {
  if (token !== undefined && token !== prev) seriesHistory.value = {};
});

// Computed Normalized Series Definitions
const seriesDefs = computed<DataSeries[]>(() => {
  if (props.config.settings?.series && props.config.settings.series.length > 0) {
    return props.config.settings.series;
  }
  // Fallback / Backward Compatibility
  return [{
    topic: '', // Will use global
    key: '', // Will use global
    name: props.config.title,
    color: undefined
  }];
});

const resolveSeriesData = (s: DataSeries) => {
  let topic = s.topic;
  let key = s.key;

  if (s.mappingId) {
    const m = dashboardStore.getDataMappingById(s.mappingId);
    if (m) {
      topic = m.topic;
      if (!key) key = m.valueKey;
    }
  }

  // If no specific topic, use global
  if (!topic) topic = globalTopic.value;
  // If no specific key and using global topic, use global key? 
  // Or if no key at all, rely on global key if available
  if (!key && topic === globalTopic.value) key = globalValueKey.value;

  return { topic, key };
};

const getPointFromMessage = (msg: MqttMessage, valueKey?: string): ChartPoint | null => {
  // Extract Y Value
  const rawVal = extractValue(msg.payload, valueKey);
  const val = parseFloat(rawVal);
  if (isNaN(val)) return null;

  // Extract X Value (Time)
  let timestamp = msg.timestamp * 1000;

  return {
    value: val,
    timestamp: timestamp
  };
};

const processMessage = (msg: MqttMessage, topic: string) => {
  seriesDefs.value.forEach((s, idx) => {
    const { topic: targetTopic, key: targetKey } = resolveSeriesData(s);

    if (targetTopic === topic) {
      const pt = getPointFromMessage(msg, targetKey);
      if (pt) {
        const startOfToday = new Date().setHours(0, 0, 0, 0);
        if (pt.timestamp < startOfToday) return;

        if (!seriesHistory.value[idx]) seriesHistory.value[idx] = [];
        const arr = seriesHistory.value[idx];
        const lastPt = arr[arr.length - 1];

        if (lastPt && lastPt.timestamp === pt.timestamp) {
          lastPt.value = pt.value;
        } else {
          arr.push(pt);
        }

        const cutoff = Date.now() - timeWindow.value - 10000;
        const effectiveCutoff = Math.max(cutoff, startOfToday);

        if (arr.length > 0 && arr[0].timestamp < effectiveCutoff) {
          seriesHistory.value[idx] = arr.filter(p => p.timestamp >= effectiveCutoff);
        }
      }
    }
  });
};

// Watch Global Topic (Throttled via props)
watch(() => props.message, (newMsg) => {
  // Check if global topic matches any series that uses global topic
  if (newMsg && globalTopic.value) {
    processMessage(newMsg, globalTopic.value);
  }
});

// Watch Extra Topics (Directly from Store)
const extraTopics = computed(() => {
  const set = new Set<string>();
  seriesDefs.value.forEach(s => {
    const { topic } = resolveSeriesData(s);
    if (topic && topic !== globalTopic.value) {
      set.add(topic);
    }
  });
  return Array.from(set);
});

watch(() => extraTopics.value.map(t => mqttStore.lastMessages[t]), (newMsgs, oldMsgs) => {
  newMsgs.forEach((msg, i) => {
    const topic = extraTopics.value[i];
    const oldMsg = oldMsgs ? oldMsgs[i] : undefined;

    if (msg && (!oldMsg || msg.timestamp !== oldMsg.timestamp)) {
      processMessage(msg, topic);
    }
  });
});

// Animation Loop
const now = ref(Date.now());
let animFrame: number;
const updateNow = () => {
  now.value = Date.now();
  animFrame = requestAnimationFrame(updateNow);
};

// Load Initial History
onMounted(async () => {
  updateNow();

  const topicsToFetch = new Set<string>();
  if (globalTopic.value) topicsToFetch.add(globalTopic.value);
  extraTopics.value.forEach(t => topicsToFetch.add(t));

  for (const topic of topicsToFetch) {
    if (!topic) continue;
    try {
      const msgs = await mqttStore.getHistory(topic, 100);
      [...msgs].reverse().forEach(msg => processMessage(msg, topic));
    } catch (e) {
      console.error(`Failed to load history for ${topic}:`, e);
    }
  }
});

onUnmounted(() => {
  cancelAnimationFrame(animFrame);
});

// Chart Option
const option = computed(() => {
  const chartType = props.config.settings?.chartType || 'line';
  const interval = props.config.settings?.tickInterval ? props.config.settings.tickInterval * 1000 : undefined;

  let minTime, maxTime;
  if (props.config.settings?.timeMode === 'absolute' && props.config.settings?.startTime && props.config.settings?.endTime) {
    const baseDate = new Date();
    const [startH, startM, startS] = props.config.settings.startTime.split(':').map(Number);
    const [endH, endM, endS] = props.config.settings.endTime.split(':').map(Number);
    minTime = new Date(baseDate).setHours(startH, startM, startS || 0);
    maxTime = new Date(baseDate).setHours(endH, endM, endS || 0);
  } else {
    maxTime = now.value;
    minTime = maxTime - timeWindow.value;
  }

  const seriesOptions = seriesDefs.value.map((s, idx) => {
    const data = seriesHistory.value[idx] || [];
    return {
      animation: false,
      name: s.name || s.key || `Series ${idx + 1}`,
      type: chartType,
      data: data.map(d => [d.timestamp, d.value]),
      itemStyle: s.color ? { color: s.color } : undefined,
      showSymbol: false,
      smooth: chartType === 'line',
      areaStyle: chartType === 'line' ? { opacity: 0.1 } : undefined,
      barMaxWidth: chartType === 'bar' ? 20 : undefined,
    };
  });

  return {
    animation: false,
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
      top: '15%',
      containLabel: true
    },
    xAxis: {
      type: 'time',
      boundaryGap: chartType === 'bar',
      splitLine: { show: false },
      minInterval: interval,
      maxInterval: interval,
      min: minTime,
      max: maxTime,
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
