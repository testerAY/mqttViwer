<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, toRef } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import { useDashboardStore } from '../../stores/useDashboardStore';
import type { WidgetConfig, DataSeries } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { decodeProtoPayload } from '../../utils/protoUtils';
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
    clearToken?: number;
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

watch(() => props.clearToken, (token, prev) => {
    if (token !== undefined && token !== prev) seriesHistory.value = {};
});

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
    let fieldName = s.fieldName;
    let valueType: string = 'json';
    let mappingProtoSchema = undefined as any;

    if (s.mappingId) {
        const m = dashboardStore.getDataMappingById(s.mappingId);
        if (m) {
            topic = m.topic;
            valueType = m.valueType || 'json';
            if (valueType === 'json' && !key) key = m.valueKey;
            if (valueType === 'binary') mappingProtoSchema = m.protoSchema;
        }
    }

    if (!topic) topic = globalTopic.value;
    if (!key && topic === globalTopic.value) key = globalValueKey.value;

    return { topic, key, fieldName, valueType, mappingProtoSchema };
};

const getPoint = (msg: MqttMessage, key?: string, valueType?: string): PlotPoint | null => {
    const rawVal = valueType === 'value' ? msg.payload : extractValue(msg.payload, key);
    const val = parseFloat(rawVal);
    if (isNaN(val)) return null;
    return { value: val, timestamp: msg.timestamp * 1000 };
};

const processMessage = (msg: MqttMessage, topic: string) => {
    seriesDefs.value.forEach((s, idx) => {
        const { topic: targetTopic, key: targetKey, fieldName: targetFieldName, valueType: targetValueType, mappingProtoSchema } = resolveSeriesData(s);

        if (targetTopic === topic) {
            let pt: PlotPoint | null = null;
            if (targetValueType === 'binary' && mappingProtoSchema && targetFieldName && msg.payload_encoding === 'base64') {
                try {
                    const decoded = decodeProtoPayload(msg.payload, mappingProtoSchema);
                    const val = parseFloat(String(decoded[targetFieldName]));
                    if (!isNaN(val)) pt = { value: val, timestamp: msg.timestamp * 1000 };
                } catch { /* ignore */ }
            } else {
                pt = getPoint(msg, targetKey, targetValueType);
            }
            if (pt) {
                const startOfToday = new Date().setHours(0, 0, 0, 0);
                if (pt.timestamp < startOfToday) return;

                if (!seriesHistory.value[idx]) seriesHistory.value[idx] = [];
                seriesHistory.value[idx].push(pt);

                const cutoff = Date.now() - timeWindow.value - 10000;
                const effectiveCutoff = Math.max(cutoff, startOfToday);

                if (seriesHistory.value[idx].length > 0 && seriesHistory.value[idx][0].timestamp < effectiveCutoff) {
                    const arr = seriesHistory.value[idx];
                    let spliceEnd = 0;
                    while (spliceEnd < arr.length && arr[spliceEnd].timestamp < effectiveCutoff) spliceEnd++;
                    if (spliceEnd > 0) arr.splice(0, spliceEnd);
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

// Extra topics watcher with throttle
let extraThrottleTimer: ReturnType<typeof setTimeout> | null = null;
let lastExtraUpdate = 0;

watch(() => extraTopics.value.map(t => mqttStore.lastMessages[t]), (newMsgs, oldMsgs) => {
    const interval = props.config.updateInterval || 0;

    const doUpdate = () => {
        newMsgs.forEach((msg, i) => {
            const topic = extraTopics.value[i];
            const oldMsg = oldMsgs ? oldMsgs[i] : undefined;
            if (msg && (!oldMsg || msg.timestamp !== oldMsg.timestamp)) {
                processMessage(msg, topic);
            }
        });
        lastExtraUpdate = Date.now();
    };

    if (interval <= 0) { doUpdate(); return; }

    const elapsed = Date.now() - lastExtraUpdate;
    if (elapsed >= interval) {
        doUpdate();
        if (extraThrottleTimer) { clearTimeout(extraThrottleTimer); extraThrottleTimer = null; }
    } else {
        if (extraThrottleTimer) clearTimeout(extraThrottleTimer);
        extraThrottleTimer = setTimeout(doUpdate, interval - elapsed);
    }
});

// Animation Loop - throttled to ~150ms
const now = ref(Date.now());
let animFrame: number;
let lastNowUpdate = 0;
const NOW_UPDATE_INTERVAL = 150;
const updateNow = () => {
    const t = Date.now();
    if (t - lastNowUpdate >= NOW_UPDATE_INTERVAL) {
        now.value = t;
        lastNowUpdate = t;
    }
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
