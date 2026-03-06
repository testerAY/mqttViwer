<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, toRef } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import { useDashboardStore } from '../../stores/useDashboardStore';
import type { WidgetConfig, DataSeries } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { CustomChart } from 'echarts/charts';
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components';
import VChart from 'vue-echarts';
import { useWidgetData } from '../../composables/useWidgetData';

use([CanvasRenderer, CustomChart, GridComponent, TooltipComponent, LegendComponent]);

const props = defineProps<{
    config: WidgetConfig;
    message: MqttMessage | undefined;
    clearToken?: number;
}>();

const mqttStore = useMqttStore();
const dashboardStore = useDashboardStore();
const { topic: globalTopic, valueKey: globalValueKey } = useWidgetData(toRef(props, 'config'));

const history = ref<any[]>([]);

const seriesDefs = computed<DataSeries[]>(() => {
    if (props.config.settings?.series && props.config.settings.series.length > 0) {
        return props.config.settings.series;
    }
    return [{
        topic: '',
        key: '',
        name: props.config.title || 'Status',
        color: undefined
    }];
});

const seriesStates = ref<Record<number, { value: string, startTime: number }>>({});

watch(() => props.clearToken, (token, prev) => {
    if (token !== undefined && token !== prev) {
        history.value = [];
        seriesStates.value = {};
    }
});

const resolveSeriesData = (s: DataSeries) => {
    let topic = s.topic;
    let key = s.key;
    let valueType: string = 'json';

    if (s.mappingId) {
        const m = dashboardStore.getDataMappingById(s.mappingId);
        if (m) {
            topic = m.topic;
            valueType = m.valueType || 'json';
            if (valueType === 'json' && !key) key = m.valueKey;
        }
    }

    if (!topic) topic = globalTopic.value;
    if (!key && topic === globalTopic.value) key = globalValueKey.value;

    return { topic, key, valueType };
};

const processMessage = (msg: MqttMessage, topic: string) => {
    seriesDefs.value.forEach((s, idx) => {
        const { topic: targetTopic, key: targetKey, valueType: targetValueType } = resolveSeriesData(s);

        if (targetTopic === topic) {
            const rawVal = targetValueType === 'value' ? msg.payload : extractValue(msg.payload, targetKey);
            const val = String(rawVal);
            const currentState = seriesStates.value[idx];
            const timestamp = msg.timestamp * 1000;

            if (currentState && currentState.value !== val) {
                history.value.push({
                    index: idx,
                    name: s.name || s.key || `Row ${idx}`,
                    start: currentState.startTime,
                    end: timestamp,
                    value: currentState.value
                });
                seriesStates.value[idx] = { value: val, startTime: timestamp };

                const window = (props.config.settings?.timeWindow || 60) * 1000;
                const cutoff = Date.now() - window - 10000;
                const startOfToday = new Date().setHours(0, 0, 0, 0);
                const effectiveCutoff = Math.max(cutoff, startOfToday);

                if (history.value.length > 0 && history.value[0].end < effectiveCutoff) {
                    history.value = history.value.filter(h => h.end >= effectiveCutoff);
                }

            } else if (!currentState) {
                seriesStates.value[idx] = { value: val, startTime: timestamp };
            }
        }
    });
};

watch(() => props.message, (msg) => {
    if (msg && globalTopic.value) processMessage(msg, globalTopic.value);
});

const extraTopics = computed(() => {
    const s = new Set<string>();
    seriesDefs.value.forEach(x => {
        const { topic } = resolveSeriesData(x);
        if (topic && topic !== globalTopic.value) s.add(topic);
    });
    return Array.from(s);
});

watch(() => extraTopics.value.map(t => mqttStore.lastMessages[t]), (newMsgs, oldMsgs) => {
    newMsgs.forEach((msg, i) => {
        const t = extraTopics.value[i];
        const old = oldMsgs?.[i];
        if (msg && (!old || msg.timestamp !== old.timestamp)) processMessage(msg, t);
    });
});

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

onUnmounted(() => cancelAnimationFrame(animFrame));

const getColor = (str: string) => {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const c = (hash & 0x00FFFFFF).toString(16).toUpperCase();
    return '#' + '00000'.substring(0, 6 - c.length) + c;
};

const renderItem = (_params: any, api: any) => {
    const categoryIndex = api.value(0);
    const start = api.coord([api.value(1), categoryIndex]);
    const end = api.coord([api.value(2), categoryIndex]);
    const height = api.size([0, 1])[1] * 0.6;

    const width = Math.max(end[0] - start[0], 1);

    return {
        type: 'rect',
        shape: {
            x: start[0],
            y: start[1] - height / 2,
            width: width,
            height: height
        },
        style: api.style()
    };
};

const option = computed(() => {
    const data = [...history.value];
    let minTime, maxTime;
    const nowTs = now.value;

    if (props.config.settings?.timeMode === 'absolute' && props.config.settings?.startTime && props.config.settings?.endTime) {
        const baseDate = new Date();
        const [startH, startM, startS] = props.config.settings.startTime.split(':').map(Number);
        const [endH, endM, endS] = props.config.settings.endTime.split(':').map(Number);
        minTime = new Date(baseDate).setHours(startH, startM, startS || 0);
        maxTime = new Date(baseDate).setHours(endH, endM, endS || 0);
    } else {
        const window = (props.config.settings?.timeWindow || 60) * 1000;
        maxTime = nowTs;
        minTime = maxTime - window;
    }

    const startOfToday = new Date().setHours(0, 0, 0, 0);

    Object.entries(seriesStates.value).forEach(([k, v]) => {
        const idx = Number(k);
        data.push({
            index: idx,
            start: Math.max(v.startTime, startOfToday),
            end: now.value,
            value: v.value
        });
    });

    const echartsData = data.map(d => ({
        value: [d.index, d.start, d.end, d.value],
        itemStyle: { color: getColor(d.value) },
        name: d.value
    }));

    const interval = props.config.settings?.tickInterval ? props.config.settings.tickInterval * 1000 : 'auto';

    return {
        animation: false,
        tooltip: {
            trigger: 'item',
            formatter: (p: any) => {
                const v = p.value;
                return `${seriesDefs.value[v[0]]?.name || 'Row ' + v[0]}: ${v[3]}<br/>${new Date(v[1]).toLocaleTimeString()} - ${new Date(v[2]).toLocaleTimeString()}`;
            }
        },
        xAxis: {
            type: 'time',
            min: minTime,
            max: maxTime,
            interval: interval,
            splitLine: { show: true }
        },
        yAxis: {
            type: 'category',
            data: seriesDefs.value.map((s, i) => s.name || `Row ${i}`)
        },
        series: [{
            type: 'custom',
            renderItem: renderItem,
            encode: { x: [1, 2], y: 0 },
            data: echartsData,
            clip: true
        }]
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
