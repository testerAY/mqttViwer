<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useMqttStore } from './stores/useMqttStore';
import { storeToRefs } from 'pinia';

const mqttStore = useMqttStore();
const { dataMap, isConnected } = storeToRefs(mqttStore);

const publishTopic = ref('test/topic');
const publishPayload = ref('Hello Tauri!');

const handlePublish = async () => {
  if (!publishTopic.value || !publishPayload.value) return;
  await mqttStore.publishMessage(publishTopic.value, publishPayload.value);
};

onMounted(() => {
  mqttStore.setupListener();
});
</script>

<template>
  <div class="flex flex-col h-screen" data-theme="dark">
    <!-- Header -->
    <header class="navbar bg-base-300 text-neutral-content">
      <div class="flex-1">
        <a class="btn btn-ghost text-xl">MQTT Web Viewer</a>
      </div>
      <div class="flex-none">
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

      <div v-if="dataMap.size === 0" class="text-center">
        <p>Waiting for data...</p>
      </div>
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <!-- Value Display Widget -->
        <div v-for="[topic, message] in dataMap.entries()" :key="topic" class="card bg-base-200 shadow-xl">
          <div class="card-body">
            <h2 class="card-title truncate">{{ topic }}</h2>
            <p class="text-4xl font-bold">{{ message.payload }}</p>
            <div class="text-xs text-gray-500">
              {{ new Date(message.timestamp * 1000).toLocaleString() }}
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
/* Scoped styles if needed */
</style>
