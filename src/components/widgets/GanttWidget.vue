<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import type { WidgetConfig, DataSeries } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { CustomChart } from 'echarts/charts';
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components';
import VChart from 'vue-echarts';

use([CanvasRenderer, CustomChart, GridComponent, TooltipComponent, LegendComponent]);

const props = defineProps<{
    config: WidgetConfig;
    message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const history = ref<any[]>([]);

const seriesDefs = computed<DataSeries[]>(() => {
    if (props.config.settings?.series && props.config.settings.series.length > 0) {
        return props.config.settings.series;
    }
    return [{
        topic: '',
        key: props.config.settings?.valueKey,
        name: props.config.title || 'Status',
        color: undefined
    }];
});

const seriesStates = ref<Record<number, { value: string, startTime: number }>>({});

const processMessage = (msg: MqttMessage, topic: string) => {
    seriesDefs.value.forEach((s, idx) => {
        if ((s.topic || props.config.topic) === topic) {
            const rawVal = extractValue(msg.payload, s.key);
            const val = String(rawVal);
            const currentState = seriesStates.value[idx];
            const timestamp = msg.timestamp * 1000;

            if (currentState && currentState.value !== val) {
                // Close previous
                history.value.push({
                    index: idx,
                    name: s.name || s.key || `Row ${idx}`,
                    start: currentState.startTime,
                    end: timestamp,
                    value: currentState.value
                });
                // Start new
                seriesStates.value[idx] = { value: val, startTime: timestamp };

                // Prune history (older than 5 mins?)
                const cutoff = Date.now() - 300000;
                if (history.value.length > 0 && history.value[0].end < cutoff) {
                    history.value = history.value.filter(h => h.end >= cutoff);
                }

            } else if (!currentState) {
                seriesStates.value[idx] = { value: val, startTime: timestamp };
            }
        }
    });
};

watch(() => props.message, (msg) => {
    if (msg && props.config.topic) processMessage(msg, props.config.topic);
});

const extraTopics = computed(() => {
    const s = new Set<string>();
    seriesDefs.value.forEach(x => { if (x.topic && x.topic !== props.config.topic) s.add(x.topic); });
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

onMounted(() => updateNow());
onUnmounted(() => cancelAnimationFrame(animFrame));

const getColor = (str: string) => {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const c = (hash & 0x00FFFFFF).toString(16).toUpperCase();
    return '#' + '00000'.substring(0, 6 - c.length) + c;
};

const renderItem = (params: any, api: any) => {
    const categoryIndex = api.value(0);
    const start = api.coord([api.value(1), categoryIndex]);
    const end = api.coord([api.value(2), categoryIndex]);
    const height = api.size([0, 1])[1] * 0.6;

    const width = Math.max(end[0] - start[0], 1); // Minimum 1px

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
    Object.entries(seriesStates.value).forEach(([k, v]) => {
        const idx = Number(k);
        data.push({
            index: idx,
            start: v.startTime,
            end: now.value,
            value: v.value
        });
    });

    const echartsData = data.map(d => ({
        value: [d.index, d.start, d.end, d.value],
        itemStyle: { color: getColor(d.value) },
        name: d.value
    }));

    return {
        tooltip: {
            trigger: 'item',
            formatter: (p: any) => {
                const v = p.value;
                return `${seriesDefs.value[v[0]]?.name || 'Row ' + v[0]}: ${v[3]}<br/>${new Date(v[1]).toLocaleTimeString()} - ${new Date(v[2]).toLocaleTimeString()}`;
            }
        },
        xAxis: {
            type: 'time',
            min: now.value - 60000,
            max: now.value,
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
            data: echartsData
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
