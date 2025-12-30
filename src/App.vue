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
