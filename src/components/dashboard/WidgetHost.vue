<script setup lang="ts">
import { computed, onMounted, ref, watch, shallowRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { usePluginStore } from '../../stores/usePluginStore';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { getWidgetComponent } from '../../registries/widgetRegistry';

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

const widgetComponent = shallowRef<any>(null);

const canExportCsv = computed(() => {
  const singleValueTypes = ['value-display', 'gauge', 'switch', 'slider'];
  return !singleValueTypes.includes(props.widget.type);
});

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
  <div class="card bg-base-100 w-full h-full shadow-md overflow-hidden flex flex-col group">
    <div class="card-body p-2 flex-grow-0 flex-row justify-between items-start min-h-[3rem]">
      <div class="overflow-hidden">
        <h3 class="card-title text-sm truncate" :title="widget.title">{{ widget.title }}</h3>
        <div v-if="resolvedMappingName" class="badge badge-primary badge-xs truncate max-w-full"
          :title="resolvedTopic || ''">
          {{ resolvedMappingName }}
        </div>
        <div v-else-if="resolvedTopic" class="badge badge-ghost badge-xs truncate max-w-full" :title="resolvedTopic">
          {{ resolvedTopic }}
        </div>
      </div>

      <div class="dropdown dropdown-end opacity-0 group-hover:opacity-100 transition-opacity">
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
          <li v-if="canExportCsv"><a @click="exportCsv">Export CSV</a></li>
          <li><a @click="emit('remove', widget.id)" class="text-error">Remove</a></li>
        </ul>
      </div>
    </div>

    <div class="flex-1 overflow-hidden relative">
      <component v-if="widgetComponent" :is="widgetComponent" :config="widget" :message="message"
        :tagName="pluginTagName" />
      <div v-else class="flex items-center justify-center h-full opacity-50 flex-col p-4 text-center">
        <div v-if="isLoading">Loading plugin...</div>
        <div v-else-if="loadError" class="text-error text-xs">{{ loadError }}</div>
        <div v-else>Widget type "{{ widget.type }}" not implemented yet</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  height: 100%;
}
</style>
