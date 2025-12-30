<script setup lang="ts">
import { computed } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { use } from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import { GaugeChart } from 'echarts/charts';
import VChart from 'vue-echarts';

use([
  CanvasRenderer,
  GaugeChart
]);

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const value = computed(() => {
  if (!props.message) return 0;
  const val = parseFloat(props.message.payload);
  return isNaN(val) ? 0 : val;
});

const min = computed(() => props.config.settings?.min ?? 0);
const max = computed(() => props.config.settings?.max ?? 100);
const unit = computed(() => props.config.settings?.unit ?? '');

const option = computed(() => ({
  series: [
    {
      type: 'gauge',
      min: min.value,
      max: max.value,
      progress: {
        show: true
      },
      detail: {
        valueAnimation: true,
        formatter: `{value}${unit.value}`,
        fontSize: 20
      },
      data: [
        {
          value: value.value,
          name: props.config.title
        }
      ],
      axisLine: {
          lineStyle: {
              width: 10
          }
      },
      axisLabel: {
          distance: 15,
          fontSize: 10
      },
      title: {
          fontSize: 12,
          offsetCenter: [0, '80%']
      }
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
