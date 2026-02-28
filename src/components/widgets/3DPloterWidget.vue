<script setup lang="ts">
import { ref, computed, watch, toRef } from 'vue';
import { use } from 'echarts/core';
// @ts-ignore - echarts-gl has no official TypeScript declarations
import { Scatter3DChart } from 'echarts-gl/charts';
// @ts-ignore
import { Grid3DComponent } from 'echarts-gl/components';
import { TooltipComponent, LegendComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import VChart from 'vue-echarts';
import type { WidgetConfig } from '../../types/dashboard';
import { useWidgetData } from '../../composables/useWidgetData';
import { extractValue } from '../../utils/jsonExtractor';

// echarts-gl コンポーネントの登録
// Note: Tooltip and Legend are from standard echarts, Grid3D is from echarts-gl
use([Scatter3DChart, Grid3DComponent, TooltipComponent, LegendComponent, CanvasRenderer]);

const props = defineProps<{
    config: WidgetConfig;
}>();

// MQTTデータ取得用の既存Composableを使用
const { lastMessage } = useWidgetData(toRef(props, 'config'));

// プロットデータの蓄積用状態
interface PlotPoint {
    x: number;
    y: number;
    z: number;
    label: string;
}
const dataPoints = ref<PlotPoint[]>([]);

// メッセージ受信時のデータ抽出処理
watch(() => lastMessage.value, (msg) => {
    if (!msg) return;
    try {
        const settings = props.config.settings || {};

        // 設定されたキーから値を抽出
        const x = extractValue(msg.payload, settings.xKey || 'x');
        const y = extractValue(msg.payload, settings.yKey || 'y');
        const z = extractValue(msg.payload, settings.zKey || 'z');
        const label = extractValue(msg.payload, settings.labelKey || 'label') || 'default';

        if (x !== undefined && y !== undefined && z !== undefined) {
            dataPoints.value.push({ x: Number(x), y: Number(y), z: Number(z), label: String(label) });

            // 最大ポイント数の制限
            const maxPoints = settings.maxPoints || 100;
            if (dataPoints.value.length > maxPoints) {
                dataPoints.value.shift();
            }
        }
    } catch (e) {
        console.warn('Failed to parse 3D Plotter data', e);
    }
}, { deep: true });

// ECharts のオプション生成
const chartOption = computed(() => {
    // データをラベルごとにグループ化する（色分けのため）
    const seriesMap = new Map<string, number[][]>();

    dataPoints.value.forEach(pt => {
        if (!seriesMap.has(pt.label)) {
            seriesMap.set(pt.label, []);
        }
        seriesMap.get(pt.label)!.push([pt.x, pt.y, pt.z]);
    });

    const series = Array.from(seriesMap.entries()).map(([label, data]) => ({
        name: label,
        type: 'scatter3D',
        symbolSize: 8,
        data: data,
        itemStyle: {
            opacity: 0.8
        }
    }));

    return {
        tooltip: {
            formatter: (params: any) => {
                return `Label: ${params.seriesName}<br/>X: ${params.value[0]}<br/>Y: ${params.value[1]}<br/>Z: ${params.value[2]}`;
            }
        },
        legend: {
            show: true,
            textStyle: { color: '#ccc' }
        },
        grid3D: {
            viewControl: {
                projection: 'perspective'
            },
            axisLine: { lineStyle: { color: '#888' } },
            axisPointer: { lineStyle: { color: '#aaa' } }
        },
        xAxis3D: { type: 'value', name: 'X' },
        yAxis3D: { type: 'value', name: 'Y' },
        zAxis3D: { type: 'value', name: 'Z' },
        series: series
    };
});
</script>

<template>
    <div class="w-full h-full p-2 flex flex-col">
        <div class="text-sm font-bold opacity-70 mb-1 shrink-0">{{ config.title }}</div>
        <div class="flex-1 min-h-0 relative">
            <v-chart class="chart w-full h-full" :option="chartOption" autoresize />
        </div>
    </div>
</template>