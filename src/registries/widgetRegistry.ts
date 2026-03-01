import { defineAsyncComponent, type Component, markRaw } from 'vue';

// コンポーネントを格納するMap
// markRawを使って、Vueのリアクティブシステムがコンポーネント定義自体を監視しないようにします（パフォーマンス対策）
const registry = new Map<string, Component>();

/**
 * ウィジェットを登録する関数
 * @param id ウィジェットの識別子 (例: 'chart', 'my-custom-plugin')
 * @param component Vueコンポーネント定義
 */
export function registerWidget(id: string, component: Component) {
  if (registry.has(id)) {
    console.warn(`Widget "${id}" is already registered. Overwriting.`);
  }
  registry.set(id, markRaw(component));
  console.log(`Widget registered: ${id}`);
}

/**
 * IDからウィジェットコンポーネントを取得する関数
 */
export function getWidgetComponent(id: string): Component | undefined {
  return registry.get(id);
}

/**
 * 標準ウィジェットを一括登録する関数
 * defineAsyncComponent を使用して、実際に使用されるまでロードを遅延させます
 */
export function registerCoreWidgets() {
  registerWidget('value-display', defineAsyncComponent(() => import('../components/widgets/ValueDisplayWidget.vue')));
  registerWidget('chart', defineAsyncComponent(() => import('../components/widgets/ChartWidget.vue')));
  registerWidget('gauge', defineAsyncComponent(() => import('../components/widgets/GaugeWidget.vue')));
  registerWidget('switch', defineAsyncComponent(() => import('../components/widgets/SwitchWidget.vue')));
  registerWidget('slider', defineAsyncComponent(() => import('../components/widgets/SliderWidget.vue')));
  registerWidget('plotter', defineAsyncComponent(() => import('../components/widgets/PlotterWidget.vue')));
  registerWidget('scatter', defineAsyncComponent(() => import('../components/widgets/ScatterWidget.vue')));
  registerWidget('gantt', defineAsyncComponent(() => import('../components/widgets/GanttWidget.vue')));
  registerWidget('multi-slider', defineAsyncComponent(() => import('../components/widgets/MultiSliderWidget.vue')));
  registerWidget('multi-switch', defineAsyncComponent(() => import('../components/widgets/MultiSwitchWidget.vue')));
  registerWidget('3d-plotter', defineAsyncComponent(() => import('../components/widgets/3DPloterWidget.vue')));
  registerWidget('rtsp-camera', defineAsyncComponent(() => import('../components/widgets/RtspCameraWidget.vue')));
}
