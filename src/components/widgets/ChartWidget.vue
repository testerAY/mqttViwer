<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
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

const history = ref<Array<{time: string, value: number}>>([]);
const maxPoints = 50;

const addDataPoint = (msg: MqttMessage) => {
    const val = parseFloat(msg.payload);
    if (!isNaN(val)) {
        const now = new Date(msg.timestamp * 1000);
        const timeStr = now.toLocaleTimeString();
        
        // Avoid duplicate timestamps if possible, or just push
        history.value.push({
            time: timeStr,
            value: val
        });

        if (history.value.length > maxPoints) {
            history.value.shift();
        }
    }
};

watch(() => props.message, (newMsg) => {
  if (newMsg) {
    addDataPoint(newMsg);
  }
});

onMounted(() => {
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
    data: history.value.map(h => h.time)
  },
  yAxis: {
    type: 'value',
    scale: true // Auto scale y axis based on values
  },
  series: [
    {
      name: props.config.title,
      type: 'line',
      data: history.value.map(h => h.value),
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
