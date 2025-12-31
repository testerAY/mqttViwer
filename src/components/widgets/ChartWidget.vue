<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue';
import { useMqttStore } from '../../stores/useMqttStore';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { extractValue } from '../../utils/jsonExtractor';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { LineChart } from 'echarts/charts';
import {
  TitleComponent,
  TooltipComponent,
  GridComponent
} from 'echarts/components';
import VChart from 'vue-echarts';

use([
  CanvasRenderer,
  LineChart,
  TitleComponent,
  TooltipComponent,
  GridComponent
]);

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const mqttStore = useMqttStore();
const history = ref<MqttMessage[]>([]);
const maxPoints = 50;

const getPointFromMessage = (msg: MqttMessage) => {
    // Extract Y value (default to payload if no key)
    const rawVal = extractValue(msg.payload, props.config.settings?.yKey);
    const val = parseFloat(rawVal);
    
    if (isNaN(val)) return null;

    // Extract X value (Time)
    let timeStr = '';
    let rawTime = undefined;

    if (props.config.settings?.xKey) {
        rawTime = extractValue(msg.payload, props.config.settings.xKey);
    }

    if (rawTime !== undefined) {
        let treatAsDate = false;
        const dateObj = new Date(rawTime);
        
        // Check if it can be a valid date
        if (!isNaN(dateObj.getTime())) {
            // Check if it's a number-like value
            // Use Number() to handle both number type and numeric strings
            const numVal = Number(rawTime);
            
            if (!isNaN(numVal)) {
                // If it's a number, check magnitude to distinguish "value" from "timestamp"
                // Threshold: 1,000,000,000
                if (numVal > 1000000000) {
                    treatAsDate = true;
                }
            } else {
                // Not a number (e.g. "2023-01-01"), so it's a date string
                treatAsDate = true;
            }
        }

        if (treatAsDate) {
            timeStr = dateObj.toLocaleTimeString();
        } else {
            timeStr = String(rawTime);
        }
    } else {
        // Default to message timestamp
        const now = new Date(msg.timestamp * 1000);
        timeStr = now.toLocaleTimeString();
    }
    
    return {
        time: timeStr,
        value: val
    };
};

const chartData = computed(() => {
    return history.value
        .map(msg => getPointFromMessage(msg))
        .filter((pt): pt is {time: string, value: number} => pt !== null);
});

const addDataPoint = (msg: MqttMessage) => {
    history.value.push(msg);
    if (history.value.length > maxPoints) {
        history.value.shift();
    }
};

watch(() => props.message, (newMsg) => {
  if (newMsg) {
    addDataPoint(newMsg);
  }
});

onMounted(async () => {
    // Load history data from backend
    if (props.config.topic) {
        try {
            const messages = await mqttStore.getHistory(props.config.topic, maxPoints);
            // Messages come in DESC order (newest first), reverse to get chronological order
            messages.reverse().forEach(msg => {
                addDataPoint(msg);
            });
        } catch (e) {
            console.error('Failed to load chart history:', e);
        }
    }

    if (props.message) {
        addDataPoint(props.message);
    }
});

const option = computed(() => ({
  tooltip: {
    trigger: 'axis'
  },
  grid: {
    left: '3%',
    right: '4%',
    bottom: '3%',
    top: '10%',
    containLabel: true
  },
  xAxis: {
    type: 'category',
    boundaryGap: false,
    data: chartData.value.map(h => h.time)
  },
  yAxis: {
    type: 'value',
    scale: true // Auto scale y axis based on values
  },
  series: [
    {
      name: props.config.title,
      type: 'line',
      data: chartData.value.map(h => h.value),
      smooth: true,
      areaStyle: {
          opacity: 0.3
      },
      lineStyle: {
          width: 2
      },
      showSymbol: false,
    }
  ]
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
