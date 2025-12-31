<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useMqttStore } from './stores/useMqttStore';
import { useDashboardStore } from './stores/useDashboardStore';
import { storeToRefs } from 'pinia';
import DashboardGrid from './components/dashboard/DashboardGrid.vue';

const mqttStore = useMqttStore();
const { isConnected } = storeToRefs(mqttStore);

const dashboardStore = useDashboardStore();
const { isEditing } = storeToRefs(dashboardStore);
const { toggleEditMode } = dashboardStore;

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

onMounted(() => {
  mqttStore.setupListener();
});
</script>

<template>
  <div class="flex flex-col h-screen" data-theme="dark">
    <!-- Header -->
    <header class="navbar bg-base-300 text-neutral-content z-50">
      <div class="flex-1">
        <a class="btn btn-ghost text-xl">MQTT Web Viewer</a>
      </div>
      <div class="flex-none flex items-center gap-4">
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
    <main class="flex-1 p-4 overflow-y-auto bg-base-100">
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
    </main>
  </div>
</template>

<style scoped>
/* Scoped styles if needed */
</style>
