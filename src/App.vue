<script setup lang="ts">
import { onMounted, ref, computed } from 'vue';
import { useMqttStore } from './stores/useMqttStore';
import { useDashboardStore } from './stores/useDashboardStore';
import { useAppStore } from './stores/useAppStore';
import { storeToRefs } from 'pinia';
import DashboardGrid from './components/dashboard/DashboardGrid.vue';
import SettingsModal from './components/SettingsModal.vue';
import DataMappingModal from './components/dashboard/DataMappingModal.vue';
import DataManagementModal from './components/dashboard/DataManagementModal.vue';
import ToastContainer from './components/ToastContainer.vue';
import { usePluginStore } from './stores/usePluginStore';

const mqttStore = useMqttStore();
const { isConnected } = storeToRefs(mqttStore);

const dashboardStore = useDashboardStore();
const { isEditing } = storeToRefs(dashboardStore);
const { toggleEditMode } = dashboardStore;

const appStore = useAppStore();
const showSettings = ref(false);
const showDataMappings = ref(false);
const showDataManagement = ref(false);

const publishTopic = ref('test/topic');
const publishPayload = ref('Hello Tauri!');

const handlePublish = async () => {
  if (!publishTopic.value || !publishPayload.value) return;
  await mqttStore.publishMessage(publishTopic.value, publishPayload.value);
};

const testHistory = async () => {
  console.log('Fetching history...');
  const history = await mqttStore.getHistory();
  console.log('History:', history);
};

const pluginStore = usePluginStore();
const { plugins } = storeToRefs(pluginStore);

const builtInWidgets = [
  {
    id: 'value-display',
    name: 'Value Display',
    icon: 'M7 20l4-16m2 16l4-16M6 9h14M4 15h14'
  },
  {
    id: 'chart',
    name: 'Chart',
    icon: 'M7 12l3-2 3 2 4-4M8 21l4-4 4 4M3 4v16a1 1 0 001 1h16'
  },
  {
    id: 'gauge',
    name: 'Gauge',
    icon: 'M12 20a8 8 0 100-16 8 8 0 000 16z M12 14v-4 M12 14l2 2'
  },
  {
    id: 'switch',
    name: 'Switch',
    icon: 'M5 12h14M12 5l7 7-7 7'
  },
  {
    id: 'slider',
    name: 'Slider',
    icon: 'M4 6h16M4 12h16M4 18h16'
  },
  {
    id: 'plotter',
    name: 'Plotter',
    icon: 'M3 12h18M3 6h18M3 18h18M5 6v12M19 6v12'
  },
  {
    id: 'gantt',
    name: 'Gantt',
    icon: 'M3 4h18v16H3z M5 8h4v2H5z M11 12h6v2H11z M6 16h5v2H6z'
  },
  {
    id: 'scatter',
    name: 'Scatter',
    icon: 'M4 4h16v16H4z M8 14a1 1 0 100-2 1 1 0 000 2z M12 10a1 1 0 100-2 1 1 0 000 2z M16 8a1 1 0 100-2 1 1 0 000 2z'
  },
  {
    id: 'multi-slider',
    name: 'Multi Slider',
    icon: 'M4 6h16M4 10h16M4 14h8M16 14h4M4 18h8M16 18h4'
  },
  {
    id: 'multi-switch',
    name: 'Multi Switch',
    icon: 'M5 8h2a2 2 0 012 2v0a2 2 0 01-2 2H5a2 2 0 01-2-2v0a2 2 0 012-2z M17 8h2a2 2 0 012 2v0a2 2 0 01-2 2h-2a2 2 0 01-2-2v0a2 2 0 012-2z M5 14h2a2 2 0 012 2v0a2 2 0 01-2 2H5a2 2 0 01-2-2v0a2 2 0 012-2z'
  },
  {
    id: '3d-plotter',
    name: '3D Plotter',
    icon: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5'
  },
];

// 標準ウィジェットとプラグインを結合したリストを生成
const availableWidgets = computed(() => {
  // プラグインをウィジェットパレット用の形式に変換
  const pluginWidgets = plugins.value.map(p => ({
    id: p.id,
    name: p.name,
    // プラグイン用の汎用アイコン（必要に応じてmanifestから取得するように拡張可能）
    icon: 'M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z'
  }));

  return [...builtInWidgets, ...pluginWidgets];
});

const handleDragStart = (event: DragEvent, type: string) => {
  dashboardStore.setDraggingNewWidget(true);
  if (event.dataTransfer) {
    event.dataTransfer.setData('text/plain', type);
    event.dataTransfer.setData('widget-type', type);
    event.dataTransfer.effectAllowed = 'copy';
  }
};

const handleDragEnd = () => {
  dashboardStore.setDraggingNewWidget(false);
};

// アプリ全体でドラッグイベントを許可する（WebView対策）
const handleGlobalDragOver = (event: DragEvent) => {
  event.preventDefault();
};

onMounted(async () => {
  await appStore.loadSettings();
  await pluginStore.fetchPlugins();
  mqttStore.setupListener();
});
</script>

<template>
  <div class="flex flex-col h-screen" :data-theme="appStore.settings.theme" @dragover="handleGlobalDragOver">
    <!-- Header -->
    <header class="navbar bg-base-300 text-neutral-content z-50">
      <div class="flex-1">
        <a class="btn btn-ghost text-xl">MQTT Web Viewer</a>
      </div>
      <div class="flex-none flex items-center gap-4">
        <button class="btn btn-ghost btn-circle" @click="showDataManagement = true" title="Data Management">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-circle" @click="showDataMappings = true" title="Data Mappings">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-circle" @click="showSettings = true" title="Settings">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-circle" @click="dashboardStore.openLayout" title="Open Layout">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-circle" @click="dashboardStore.saveLayoutAs" title="Save Layout">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
          </svg>
        </button>
        <div class="divider divider-horizontal m-0"></div>
        <button class="btn btn-xs btn-outline btn-warning" @click="testHistory">Test History</button>
        <button class="btn btn-xs btn-outline btn-info" @click="mqttStore.startSimulation()">Simulate Data</button>
        <div class="form-control">
          <label class="label cursor-pointer gap-2">
            <span class="label-text text-neutral-content">Edit Mode</span>
            <input type="checkbox" class="toggle toggle-primary" :checked="isEditing" @change="toggleEditMode" />
          </label>
        </div>
        <div class="badge" :class="isConnected ? 'badge-success' : 'badge-error'">
          {{ isConnected ? 'Connected' : 'Disconnected' }}
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="flex-1 overflow-hidden bg-base-100 flex">
      <!-- Widget Palette -->
      <aside v-if="isEditing"
        class="w-64 bg-base-200 border-r border-base-300 flex flex-col z-40 transition-all duration-300">
        <div class="p-4 border-b border-base-300">
          <h2 class="font-bold text-lg">Widgets</h2>
          <p class="text-xs text-base-content/70">Drag to dashboard</p>
        </div>
        <div class="p-4 space-y-3 overflow-y-auto flex-1">
          <div v-for="type in availableWidgets" :key="type.id"
            class="card bg-base-100 shadow-sm cursor-grab hover:shadow-md transition-shadow border border-base-300 active:cursor-grabbing"
            draggable="true" @dragstart="handleDragStart($event, type.id)" @dragend="handleDragEnd">
            <div class="card-body p-4 flex flex-row items-center gap-3">
              <div class="p-2 bg-primary/10 rounded-lg text-primary">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24"
                  stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" :d="type.icon" />
                </svg>
              </div>
              <span class="font-medium">{{ type.name }}</span>
            </div>
          </div>
        </div>
      </aside>

      <!-- Dashboard Area -->
      <div class="flex-1 p-4 overflow-y-auto relative">
        <!-- Dashboard Grid -->
        <DashboardGrid />
      </div>
    </main>
    <SettingsModal :open="showSettings" @close="showSettings = false" />
    <DataMappingModal :open="showDataMappings" @close="showDataMappings = false" />
    <DataManagementModal :open="showDataManagement" @close="showDataManagement = false" />
    <ToastContainer />
  </div>
</template>

<style scoped>
/* Scoped styles if needed */
</style>
