<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { ScatterChart } from 'echarts/charts';
import { GridComponent, TooltipComponent } from 'echarts/components';
import VChart from 'vue-echarts';

use([CanvasRenderer, ScatterChart, GridComponent, TooltipComponent]);

const props = defineProps<{
    config: WidgetConfig;
    message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const history = ref<number[][]>([]);
const maxPoints = 200;

const currentX = ref<number | null>(null);
const currentY = ref<number | null>(null);

const xTopic = computed(() => props.config.settings?.xTopic || props.config.topic);
const yTopic = computed(() => props.config.settings?.yTopic || props.config.topic);
const xKey = computed(() => props.config.settings?.xKey);
// Use valueKey as fallback for Y if yKey not present
const yKey = computed(() => props.config.settings?.yKey || props.config.settings?.valueKey);

const processMessage = (msg: MqttMessage, topic: string) => {
    let updated = false;

    if (topic === xTopic.value) {
        const val = parseFloat(extractValue(msg.payload, xKey.value));
        if (!isNaN(val)) {
            currentX.value = val;
            updated = true;
        }
    }

    if (topic === yTopic.value) {
        const val = parseFloat(extractValue(msg.payload, yKey.value));
        if (!isNaN(val)) {
            currentY.value = val;
            updated = true;
        }
    }

    // Only push if we have both values and at least one updated
    if (updated && currentX.value !== null && currentY.value !== null) {
        const timestamp = msg.timestamp * 1000;
        const startOfToday = new Date().setHours(0, 0, 0, 0);

        if (timestamp >= startOfToday) {
            history.value.push([currentX.value, currentY.value, timestamp]);

            // Prune old data (older than today)
            if (history.value.length > 0 && history.value[0][2] < startOfToday) {
                history.value = history.value.filter(p => p[2] >= startOfToday);
            }

            if (history.value.length > maxPoints) history.value.shift();
        }
    }
};

watch(() => props.message, (msg) => {
    if (msg && props.config.topic) processMessage(msg, props.config.topic);
});

const extraTopics = computed(() => {
    const s = new Set<string>();
    if (xTopic.value && xTopic.value !== props.config.topic) s.add(xTopic.value);
    if (yTopic.value && yTopic.value !== props.config.topic) s.add(yTopic.value);
    return Array.from(s);
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

const option = computed(() => ({
    grid: { left: 40, right: 40, top: 20, bottom: 30, containLabel: true },
    xAxis: {
        type: 'value',
        scale: true,
        name: props.config.settings?.xLabel || 'X',
        nameLocation: 'middle',
        nameGap: 25
    },
    yAxis: {
        type: 'value',
        scale: true,
        name: props.config.settings?.yLabel || 'Y',
    },
    tooltip: {
        trigger: 'item',
        formatter: (params: any) => `X: ${params.data[0]}<br/>Y: ${params.data[1]}`
    },
    series: [{
        type: 'scatter',
        data: history.value,
        symbolSize: 10,
        itemStyle: { color: props.config.settings?.color || '#3b82f6' }
    }]
}));
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
