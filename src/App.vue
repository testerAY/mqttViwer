<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useMqttStore } from './stores/useMqttStore';
import { useDashboardStore } from './stores/useDashboardStore';
import { useAppStore } from './stores/useAppStore';
import { storeToRefs } from 'pinia';
import DashboardGrid from './components/dashboard/DashboardGrid.vue';
import SettingsModal from './components/SettingsModal.vue';

const mqttStore = useMqttStore();
const { isConnected } = storeToRefs(mqttStore);

const dashboardStore = useDashboardStore();
const { isEditing } = storeToRefs(dashboardStore);
const { toggleEditMode } = dashboardStore;

const appStore = useAppStore();
const showSettings = ref(false);

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

const widgetTypes = [
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
];

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

onMounted(async () => {
  await appStore.loadSettings();
  mqttStore.setupListener();
});
</script>

<template>
  <div class="flex flex-col h-screen" :data-theme="appStore.settings.theme">
    <!-- Header -->
    <header class="navbar bg-base-300 text-neutral-content z-50">
      <div class="flex-1">
        <a class="btn btn-ghost text-xl">MQTT Web Viewer</a>
      </div>
      <div class="flex-none flex items-center gap-4">
        <button class="btn btn-ghost btn-circle" @click="showSettings = true" title="Settings">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-circle" @click="dashboardStore.openLayout" title="Open Layout">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-circle" @click="dashboardStore.saveLayoutAs" title="Save Layout">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
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
      <aside v-if="isEditing" class="w-64 bg-base-200 border-r border-base-300 flex flex-col z-40 transition-all duration-300">
        <div class="p-4 border-b border-base-300">
          <h2 class="font-bold text-lg">Widgets</h2>
          <p class="text-xs text-base-content/70">Drag to dashboard</p>
        </div>
        <div class="p-4 space-y-3 overflow-y-auto flex-1">
          <div 
            v-for="type in widgetTypes" 
            :key="type.id"
            class="card bg-base-100 shadow-sm cursor-grab hover:shadow-md transition-shadow border border-base-300 active:cursor-grabbing"
            draggable="true"
            @dragstart="handleDragStart($event, type.id)"
            @dragend="handleDragEnd"
          >
            <div class="card-body p-4 flex flex-row items-center gap-3">
              <div class="p-2 bg-primary/10 rounded-lg text-primary">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
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
        <!-- Publish Test Section -->
        <div class="card bg-base-200 shadow-xl mb-6">
          <div class="card-body">
            <h2 class="card-title">Publish Test</h2>
            <div class="flex gap-4 items-end flex-wrap">
              <div class="form-control w-full max-w-xs">
                <label class="label">
                  <span class="label-text">Topic</span>
                </label>
                <input v-model="publishTopic" type="text" placeholder="Topic" class="input input-bordered w-full max-w-xs" />
              </div>
              <div class="form-control w-full max-w-xs">
                <label class="label">
                  <span class="label-text">Payload</span>
                </label>
                <input v-model="publishPayload" type="text" placeholder="Payload" class="input input-bordered w-full max-w-xs" />
              </div>
              <button class="btn btn-primary" @click="handlePublish">Send</button>
            </div>
          </div>
        </div>

        <!-- Dashboard Grid -->
        <DashboardGrid />
      </div>
    </main>
    <SettingsModal :open="showSettings" @close="showSettings = false" />
  </div>
</template>

<style scoped>
/* Scoped styles if needed */
</style>
