<script setup lang="ts">
import { ref, watch, computed } from 'vue';
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
    clearToken?: number;
}>();

const mqttStore = useMqttStore();
const maxPoints = 200;
// Ring buffer for O(1) append/evict instead of O(n) shift()
const _ringBuffer: number[][] = new Array(maxPoints);
let _ringHead = 0; // next write index
let _ringCount = 0;
const history = ref<number[][]>([]);

const currentX = ref<number | null>(null);
const currentY = ref<number | null>(null);

watch(() => props.clearToken, (token, prev) => {
    if (token !== undefined && token !== prev) {
        _ringHead = 0;
        _ringCount = 0;
        history.value = [];
        currentX.value = null;
        currentY.value = null;
    }
});

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
            // Ring buffer: O(1) insert, overwrites oldest when full
            _ringBuffer[_ringHead] = [currentX.value, currentY.value, timestamp];
            _ringHead = (_ringHead + 1) % maxPoints;
            if (_ringCount < maxPoints) _ringCount++;

            // Rebuild display array from ring buffer in correct order
            const result: number[][] = [];
            const start = _ringCount < maxPoints ? 0 : _ringHead;
            for (let i = 0; i < _ringCount; i++) {
                result.push(_ringBuffer[(start + i) % maxPoints]);
            }
            history.value = result;
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
