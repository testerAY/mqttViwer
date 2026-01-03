<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useMqttStore } from '../../stores/useMqttStore';
import { usePluginStore } from '../../stores/usePluginStore';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { getWidgetComponent } from '../../registries/widgetRegistry'; // レジストリ

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
const pluginTagName = ref<string | null>(null);
const loadError = ref<string | null>(null);
const isLoading = ref(false);

const message = computed(() => {
  if (!props.widget.topic) return undefined;
  return mqttStore.dataMap.get(props.widget.topic);
});

// コンポーネント解決ロジック
const widgetComponent = computed(() => {
  // レジストリから取得
  const comp = getWidgetComponent(props.widget.type);
  if (comp) return comp;

  // まだロードされていない、または存在しない場合
  return null;
});

watch(() => props.widget.type, async (newType) => {
  // すでにレジストリにある場合は何もしない
  if (getWidgetComponent(newType)) return;

  // レジストリになければ、PluginStoreを使ってロードを試みる
  isLoading.value = true;
  loadError.value = null;

  try {
    const success = await usePluginStore().loadPlugin(newType);
    if (!success) {
      loadError.value = `Plugin "${newType}" could not be loaded.`;
    }
  } catch (e) {
    loadError.value = `Error loading plugin: ${e}`;
  } finally {
    isLoading.value = false;
  }
}, { immediate: true });

const exportCsv = async () => {
  if (!props.widget.topic) {
    alert('No topic set for this widget.');
    return;
  }

  try {
    const csvData: string = await invoke('export_widget_data_as_csv', {
      topic: props.widget.topic,
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
        <div v-if="widget.topic" class="badge badge-ghost badge-xs truncate max-w-full" :title="widget.topic">{{
          widget.topic }}</div>
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
          <li><a @click="exportCsv">Export CSV</a></li>
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
