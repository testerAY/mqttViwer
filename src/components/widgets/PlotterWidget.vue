<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import type { WidgetConfig, DataSeries } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { LineChart } from 'echarts/charts';
import { GridComponent, TooltipComponent } from 'echarts/components';
import VChart from 'vue-echarts';

use([CanvasRenderer, LineChart, GridComponent, TooltipComponent]);

const props = defineProps<{
    config: WidgetConfig;
    message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const windowSeconds = 60; // TODO: Make configurable
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
        key: props.config.settings?.valueKey,
        name: props.config.title,
        color: undefined
    }];
});

const getPoint = (msg: MqttMessage, key?: string): PlotPoint | null => {
    const rawVal = extractValue(msg.payload, key || props.config.settings?.yKey);
    const val = parseFloat(rawVal);
    if (isNaN(val)) return null;
    return { value: val, timestamp: msg.timestamp * 1000 };
};

const processMessage = (msg: MqttMessage, topic: string) => {
    seriesDefs.value.forEach((s, idx) => {
        if ((s.topic || props.config.topic) === topic) {
            const pt = getPoint(msg, s.key);
            if (pt) {
                // 当日チェック
                const startOfToday = new Date().setHours(0, 0, 0, 0);
                if (pt.timestamp < startOfToday) return;

                if (!seriesHistory.value[idx]) seriesHistory.value[idx] = [];
                seriesHistory.value[idx].push(pt);

                // Prune data outside window + buffer
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
    if (newMsg && props.config.topic) processMessage(newMsg, props.config.topic);
});

// Watch Extra Topics
const extraTopics = computed(() => {
    const set = new Set<string>();
    seriesDefs.value.forEach(s => {
        if (s.topic && s.topic !== props.config.topic) set.add(s.topic);
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

    // Initial History
    const topicsToFetch = new Set<string>();
    if (props.config.topic) topicsToFetch.add(props.config.topic);
    extraTopics.value.forEach(t => topicsToFetch.add(t));

    for (const topic of topicsToFetch) {
        if (!topic) continue;
        try {
            const msgs = await mqttStore.getHistory(topic, 100); // Fetch enough for window
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
        // 今日の日付に設定された時刻を適用する簡易実装
        const baseDate = new Date();
        const [startH, startM, startS] = props.config.settings.startTime.split(':').map(Number);
        const [endH, endM, endS] = props.config.settings.endTime.split(':').map(Number);

        minTime = new Date(baseDate).setHours(startH, startM, startS || 0);
        maxTime = new Date(baseDate).setHours(endH, endM, endS || 0);
    } else {
        // Relative Mode
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
