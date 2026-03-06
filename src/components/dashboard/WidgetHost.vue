<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, shallowRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { usePluginStore } from '../../stores/usePluginStore';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { getWidgetComponent } from '../../registries/widgetRegistry';
import MultiSliderKeysModal from './MultiSliderKeysModal.vue';
import MultiSwitchKeysModal from './MultiSwitchKeysModal.vue';

const props = defineProps<{
  widget: WidgetConfig;
}>();

onMounted(() => {
  console.log(`Widget mounted: ${props.widget.id} (${props.widget.type})`);
});

const emit = defineEmits<{
  (e: 'edit', id: string): void;
  (e: 'remove', id: string): void;
}>();

const mqttStore = useMqttStore();
const dashboardStore = useDashboardStore();
const pluginTagName = ref<string | null>(null);
const loadError = ref<string | null>(null);
const isLoading = ref(false);

const MULTI_KEY_WIDGET_TYPES = ['multi-slider', 'multi-switch'];
const isMultiKeyWidget = computed(() => MULTI_KEY_WIDGET_TYPES.includes(props.widget.type));
const showKeysModal = ref(false);

const resolvedTopic = computed(() => {
  if (props.widget.mappingId) {
    const mapping = dashboardStore.getDataMappingById(props.widget.mappingId);
    if (mapping) return mapping.topic;
  }
  return props.widget.topic;
});

const resolvedMappingName = computed(() => {
  if (props.widget.mappingId) {
    const mapping = dashboardStore.getDataMappingById(props.widget.mappingId);
    if (mapping) return mapping.name;
  }
  return null;
});

const rawMessage = computed(() => {
  if (!resolvedTopic.value) return undefined;
  return mqttStore.dataMap.get(resolvedTopic.value);
});

const message = ref<any>(undefined);
const lastUpdate = ref(0);
let throttleTimer: ReturnType<typeof setTimeout> | null = null;

watch(rawMessage, (newMsg) => {
  const interval = props.widget.updateInterval || 0;

  if (interval <= 0) {
    message.value = newMsg;
    if (throttleTimer) {
      clearTimeout(throttleTimer);
      throttleTimer = null;
    }
    return;
  }

  const now = Date.now();
  const timeSinceLast = now - lastUpdate.value;

  if (timeSinceLast >= interval) {
    message.value = newMsg;
    lastUpdate.value = now;
    if (throttleTimer) {
      clearTimeout(throttleTimer);
      throttleTimer = null;
    }
  } else {
    // Schedule trailing update
    if (throttleTimer) clearTimeout(throttleTimer);
    throttleTimer = setTimeout(() => {
      message.value = newMsg;
      lastUpdate.value = Date.now();
      throttleTimer = null;
    }, interval - timeSinceLast);
  }
}, { immediate: true });

// B2: Data freshness indicator
const STALE_WARN_SEC = 5;
const STALE_DANGER_SEC = 15;
const lastDataTimestamp = ref(0);
const dataAgeSec = ref(-1); // -1 = no data yet
const dataFreshness = computed<'fresh' | 'warn' | 'danger' | 'none'>(() => {
  if (dataAgeSec.value < 0) return 'none';
  if (dataAgeSec.value >= STALE_DANGER_SEC) return 'danger';
  if (dataAgeSec.value >= STALE_WARN_SEC) return 'warn';
  return 'fresh';
});

watch(rawMessage, (msg) => {
  if (msg) lastDataTimestamp.value = Date.now();
}, { immediate: true });

let _freshnessTimer: ReturnType<typeof setInterval> | null = null;
onMounted(() => {
  _freshnessTimer = setInterval(() => {
    if (lastDataTimestamp.value > 0) {
      dataAgeSec.value = Math.floor((Date.now() - lastDataTimestamp.value) / 1000);
    }
  }, 1000);
});
onUnmounted(() => {
  if (_freshnessTimer) clearInterval(_freshnessTimer);
});

// B5: Fullscreen mode
const isFullscreen = ref(false);
const toggleFullscreen = () => { isFullscreen.value = !isFullscreen.value; };
const handleEsc = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isFullscreen.value) isFullscreen.value = false;
};
onMounted(() => document.addEventListener('keydown', handleEsc));
onUnmounted(() => document.removeEventListener('keydown', handleEsc));

const widgetComponent = shallowRef<any>(null);

const canExportCsv = computed(() => {
  const singleValueTypes = ['value-display', 'gauge', 'switch', 'slider'];
  return !singleValueTypes.includes(props.widget.type);
});

const CHART_WIDGET_TYPES = ['chart', 'plotter', 'gantt', 'scatter', '3d-plotter'];
const isChartWidget = computed(() => CHART_WIDGET_TYPES.includes(props.widget.type));
const clearToken = ref(0);
const clearPlot = () => { clearToken.value++; };

watch(() => props.widget.type, async (newType) => {

  const comp = getWidgetComponent(newType);

  if (comp) {
    widgetComponent.value = comp;
    return;
  }

  isLoading.value = true;
  loadError.value = null;
  widgetComponent.value = null;

  try {
    const success = await usePluginStore().loadPlugin(newType);
    if (!success) {
      loadError.value = `Plugin "${newType}" could not be loaded.`;
    }
    else {
      widgetComponent.value = getWidgetComponent(newType);
    }
  } catch (e) {
    loadError.value = `Error loading plugin: ${e}`;
  } finally {
    isLoading.value = false;
  }
}, { immediate: true });

const exportCsv = async () => {
  if (!resolvedTopic.value) {
    alert('No topic set for this widget.');
    return;
  }

  try {
    const csvData: string = await invoke('export_widget_data_as_csv', {
      topic: resolvedTopic.value,
    });

    if (!csvData) {
      alert('No data to export.');
      return;
    }

    const filePath = await save({
      defaultPath: `${props.widget.title.replace(/\s/g, '_')}.csv`,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    });

    if (filePath) {
      await writeTextFile(filePath, csvData);
      alert('Export successful!');
    }
  } catch (err) {
    console.error('Export failed:', err);
    alert(`Export failed: ${err}`);
  }
};
</script>

<template>
  <!-- B5: Fullscreen overlay -->
  <Teleport to="body">
    <div v-if="isFullscreen" class="fixed inset-0 z-[9999] bg-base-100 flex flex-col" @keydown.esc="isFullscreen = false">
      <div class="flex items-center justify-between p-2 bg-base-300">
        <h3 class="font-bold text-lg px-2">{{ widget.title }}</h3>
        <button class="btn btn-ghost btn-sm btn-circle" @click="isFullscreen = false" title="Exit fullscreen (Esc)">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <div class="flex-1 overflow-hidden">
        <component v-if="widgetComponent" :is="widgetComponent" :config="widget" :message="message"
          :tagName="pluginTagName" :clearToken="clearToken" />
      </div>
    </div>
  </Teleport>

  <div
    class="card bg-base-100 w-full h-full shadow-md overflow-hidden flex flex-col group transition-colors duration-300"
    :class="{
      'ring-2 ring-warning/50': dataFreshness === 'warn',
      'ring-2 ring-error/50': dataFreshness === 'danger'
    }"
  >
    <div class="card-body p-2 flex-grow-0 flex-row justify-between items-start min-h-[3rem]">
      <div class="overflow-hidden flex items-center gap-1">
        <h3 class="card-title text-sm truncate" :title="widget.title">{{ widget.title }}</h3>
        <!-- B2: Data freshness dot -->
        <span
          v-if="resolvedTopic && dataFreshness !== 'none'"
          class="inline-block w-2 h-2 rounded-full flex-shrink-0"
          :class="{
            'bg-success': dataFreshness === 'fresh',
            'bg-warning animate-pulse': dataFreshness === 'warn',
            'bg-error animate-pulse': dataFreshness === 'danger'
          }"
          :title="dataAgeSec >= 0 ? `Last update: ${dataAgeSec}s ago` : 'No data'"
        ></span>
        <template v-if="!isChartWidget">
          <div v-if="resolvedMappingName" class="badge badge-primary badge-xs truncate max-w-full"
            :title="resolvedTopic || ''">
            {{ resolvedMappingName }}
          </div>
          <div v-else-if="resolvedTopic" class="badge badge-ghost badge-xs truncate max-w-full" :title="resolvedTopic">
            {{ resolvedTopic }}
          </div>
        </template>
      </div>

      <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <!-- B5: Fullscreen button -->
        <button class="btn btn-ghost btn-xs btn-circle" @click="toggleFullscreen" title="Fullscreen">
          <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5v-4m0 4h-4m4 0l-5-5" />
          </svg>
        </button>

        <div class="dropdown dropdown-end">
          <div tabindex="0" role="button" class="btn btn-ghost btn-xs btn-circle">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"
              class="inline-block w-4 h-4 stroke-current">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z">
              </path>
            </svg>
          </div>
          <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
            <li><a @click="emit('edit', widget.id)">Settings</a></li>
            <li v-if="isMultiKeyWidget"><a @click="showKeysModal = true">Configure Keys</a></li>
            <li v-if="canExportCsv"><a @click="exportCsv">Export CSV</a></li>
            <li v-if="isChartWidget"><a @click="clearPlot">Clear Plot</a></li>
            <li><a @click="emit('remove', widget.id)" class="text-error">Remove</a></li>
          </ul>
        </div>
      </div>
    </div>

    <div class="flex-1 overflow-hidden relative">
      <!-- B2: Stale data overlay -->
      <div v-if="dataFreshness === 'danger'" class="absolute top-0 left-0 right-0 z-10 text-center">
        <span class="badge badge-error badge-xs">Stale Data ({{ dataAgeSec }}s)</span>
      </div>
      <component v-if="widgetComponent" :is="widgetComponent" :config="widget" :message="message"
        :tagName="pluginTagName" :clearToken="clearToken" />
      <div v-else class="flex items-center justify-center h-full opacity-50 flex-col p-4 text-center">
        <div v-if="isLoading">Loading plugin...</div>
        <div v-else-if="loadError" class="text-error text-xs">{{ loadError }}</div>
        <div v-else>Widget type "{{ widget.type }}" not implemented yet</div>
      </div>
    </div>
  </div>

  <MultiSliderKeysModal
    v-if="widget.type === 'multi-slider'"
    :open="showKeysModal"
    :widgetId="widget.id"
    @close="showKeysModal = false"
  />
  <MultiSwitchKeysModal
    v-if="widget.type === 'multi-switch'"
    :open="showKeysModal"
    :widgetId="widget.id"
    @close="showKeysModal = false"
  />
</template>

<style scoped>
.card {
  height: 100%;
}
</style>
