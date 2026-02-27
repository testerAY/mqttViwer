<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, toRef } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import { useDashboardStore } from '../../stores/useDashboardStore';
import type { WidgetConfig, DataSeries } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent } from 'echarts/components';
import VChart from 'vue-echarts';
import { useWidgetData } from '../../composables/useWidgetData';

use([CanvasRenderer, LineChart, GridComponent, TooltipComponent]);

const props = defineProps<{
    config: WidgetConfig;
    message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const dashboardStore = useDashboardStore();
const { topic: globalTopic, valueKey: globalValueKey } = useWidgetData(toRef(props, 'config'));

const timeWindow = computed(() => (props.config.settings?.timeWindow || 60) * 1000);

interface PlotPoint {
    value: number;
    timestamp: number;
}
const seriesHistory = ref<Record<number, PlotPoint[]>>({});

// Normalized Series
const seriesDefs = computed<DataSeries[]>(() => {
    if (props.config.settings?.series && props.config.settings.series.length > 0) {
        return props.config.settings.series;
    }
    return [{
        topic: '',
        key: '',
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

    if (!topic) topic = globalTopic.value;
    if (!key && topic === globalTopic.value) key = globalValueKey.value;

    return { topic, key };
};

const getPoint = (msg: MqttMessage, key?: string): PlotPoint | null => {
    const rawVal = extractValue(msg.payload, key);
    const val = parseFloat(rawVal);
    if (isNaN(val)) return null;
    return { value: val, timestamp: msg.timestamp * 1000 };
};

const processMessage = (msg: MqttMessage, topic: string) => {
    seriesDefs.value.forEach((s, idx) => {
        const { topic: targetTopic, key: targetKey } = resolveSeriesData(s);

        if (targetTopic === topic) {
            const pt = getPoint(msg, targetKey);
            if (pt) {
                const startOfToday = new Date().setHours(0, 0, 0, 0);
                if (pt.timestamp < startOfToday) return;

                if (!seriesHistory.value[idx]) seriesHistory.value[idx] = [];
                seriesHistory.value[idx].push(pt);

                const cutoff = Date.now() - timeWindow.value - 10000;
                const effectiveCutoff = Math.max(cutoff, startOfToday);

                if (seriesHistory.value[idx].length > 0 && seriesHistory.value[idx][0].timestamp < effectiveCutoff) {
                    seriesHistory.value[idx] = seriesHistory.value[idx].filter(p => p.timestamp >= effectiveCutoff);
                }
            }
        }
    });
};

// Watch Global
watch(() => props.message, (newMsg) => {
    if (newMsg && globalTopic.value) processMessage(newMsg, globalTopic.value);
});

// Watch Extra Topics
const extraTopics = computed(() => {
    const set = new Set<string>();
    seriesDefs.value.forEach(s => {
        const { topic } = resolveSeriesData(s);
        if (topic && topic !== globalTopic.value) set.add(topic);
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
        } catch (e) { console.error(e); }
    }
});

onUnmounted(() => {
    cancelAnimationFrame(animFrame);
});

const option = computed(() => {
    const nowTs = now.value;
    let minTime, maxTime;

    if (props.config.settings?.timeMode === 'absolute' && props.config.settings?.startTime && props.config.settings?.endTime) {
        const baseDate = new Date();
        const [startH, startM, startS] = props.config.settings.startTime.split(':').map(Number);
        const [endH, endM, endS] = props.config.settings.endTime.split(':').map(Number);

        minTime = new Date(baseDate).setHours(startH, startM, startS || 0);
        maxTime = new Date(baseDate).setHours(endH, endM, endS || 0);
    } else {
        maxTime = nowTs;
        minTime = maxTime - timeWindow.value;
    }

    const interval = props.config.settings?.tickInterval ? props.config.settings.tickInterval * 1000 : 'auto';

    const series = seriesDefs.value.map((s, idx) => ({
        name: s.name || `Series ${idx + 1}`,
        type: 'line',
        data: (seriesHistory.value[idx] || []).map(p => [p.timestamp, p.value]),
        showSymbol: false,
        animation: false,
        itemStyle: s.color ? { color: s.color } : undefined,
        lineStyle: { width: 2 }
    }));

    return {
        animation: false,
        tooltip: { trigger: 'axis' },
        grid: { left: 40, right: 20, top: 10, bottom: 20, containLabel: true },
        xAxis: {
            type: 'time',
            min: minTime,
            max: maxTime,
            interval: interval,
            splitLine: { show: true }
        },
        yAxis: { type: 'value', scale: true, splitLine: { show: true } },
        series: series
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
