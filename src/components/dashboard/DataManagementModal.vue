<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { useToastStore } from '../../stores/useToastStore';

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: 'close'): void }>();

const dashboardStore = useDashboardStore();
const toastStore = useToastStore();

const messageCounts = ref<Record<string, number>>({});
const selectedTopics = ref<Set<string>>(new Set());
const isLoading = ref(false);
const isDeleting = ref(false);
const confirmStep = ref(false);

// Unique topics from DataMappings with associated mapping names
const topicEntries = computed(() => {
  const map = new Map<string, string[]>();
  for (const m of dashboardStore.dataMappings) {
    const existing = map.get(m.topic) ?? [];
    existing.push(m.name);
    map.set(m.topic, existing);
  }
  return [...map.entries()].map(([topic, names]) => ({ topic, names }));
});

const allSelected = computed(() =>
  topicEntries.value.length > 0 &&
  topicEntries.value.every(e => selectedTopics.value.has(e.topic))
);

const toggleAll = () => {
  if (allSelected.value) {
    selectedTopics.value = new Set();
  } else {
    selectedTopics.value = new Set(topicEntries.value.map(e => e.topic));
  }
};

const toggleTopic = (topic: string) => {
  const next = new Set(selectedTopics.value);
  if (next.has(topic)) next.delete(topic);
  else next.add(topic);
  selectedTopics.value = next;
};

const totalSelectedRecords = computed(() => {
  let total = 0;
  for (const topic of selectedTopics.value) {
    total += messageCounts.value[topic] ?? 0;
  }
  return total;
});

const totalAllRecords = computed(() =>
  Object.values(messageCounts.value).reduce((a, b) => a + b, 0)
);

const loadCounts = async () => {
  isLoading.value = true;
  try {
    messageCounts.value = await invoke<Record<string, number>>('get_message_counts');
  } catch (e) {
    console.error('Failed to get message counts:', e);
  } finally {
    isLoading.value = false;
  }
};

watch(() => props.open, (isOpen) => {
  if (isOpen) {
    selectedTopics.value = new Set();
    confirmStep.value = false;
    loadCounts();
  }
});

const handleDelete = async () => {
  if (selectedTopics.value.size === 0) return;

  if (!confirmStep.value) {
    confirmStep.value = true;
    return;
  }

  isDeleting.value = true;
  try {
    const topics = [...selectedTopics.value];
    const deleted = await invoke<number>('delete_messages', { topics });
    toastStore.addToast(`${deleted} records deleted.`, 'success');
    await loadCounts();
    selectedTopics.value = new Set();
    confirmStep.value = false;
  } catch (e) {
    toastStore.addToast(`Delete failed: ${e}`, 'error');
  } finally {
    isDeleting.value = false;
  }
};

const cancelConfirm = () => {
  confirmStep.value = false;
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-2xl">
      <h3 class="font-bold text-lg mb-1">Data Management</h3>
      <p class="text-sm opacity-60 mb-4">Select topics to delete their stored records from the database.</p>

      <!-- Summary bar -->
      <div class="stats stats-horizontal bg-base-200 rounded-box w-full mb-4 text-sm">
        <div class="stat py-2 px-4">
          <div class="stat-title text-xs">Total Records</div>
          <div class="stat-value text-base">{{ isLoading ? '…' : totalAllRecords.toLocaleString() }}</div>
        </div>
        <div class="stat py-2 px-4">
          <div class="stat-title text-xs">Selected Records</div>
          <div class="stat-value text-base text-error">{{ totalSelectedRecords.toLocaleString() }}</div>
        </div>
        <div class="stat py-2 px-4">
          <div class="stat-title text-xs">Topics</div>
          <div class="stat-value text-base">{{ selectedTopics.size }} / {{ topicEntries.length }}</div>
        </div>
      </div>

      <!-- Topic list -->
      <div class="border border-base-300 rounded-box overflow-hidden mb-4">
        <!-- Header row -->
        <div class="flex items-center gap-3 px-4 py-2 bg-base-200 border-b border-base-300">
          <input type="checkbox" class="checkbox checkbox-sm" :checked="allSelected"
            :indeterminate="selectedTopics.size > 0 && !allSelected" @change="toggleAll" />
          <span class="text-xs font-bold flex-1">Topic</span>
          <span class="text-xs font-bold w-20 text-right">Records</span>
        </div>

        <!-- Loading -->
        <div v-if="isLoading" class="flex justify-center items-center py-8">
          <span class="loading loading-spinner loading-md"></span>
        </div>

        <!-- Empty -->
        <div v-else-if="topicEntries.length === 0" class="text-center text-sm opacity-60 py-8">
          No Data Mappings defined.
        </div>

        <!-- Topic rows -->
        <div v-else class="max-h-64 overflow-y-auto divide-y divide-base-300">
          <label v-for="entry in topicEntries" :key="entry.topic"
            class="flex items-center gap-3 px-4 py-3 hover:bg-base-200 cursor-pointer transition-colors">
            <input type="checkbox" class="checkbox checkbox-sm"
              :checked="selectedTopics.has(entry.topic)" @change="toggleTopic(entry.topic)" />
            <div class="flex-1 min-w-0">
              <div class="font-mono text-sm truncate">{{ entry.topic }}</div>
              <div class="text-xs opacity-60 truncate">{{ entry.names.join(', ') }}</div>
            </div>
            <span class="text-sm w-20 text-right tabular-nums shrink-0">
              {{ (messageCounts[entry.topic] ?? 0).toLocaleString() }}
            </span>
          </label>
        </div>
      </div>

      <!-- Confirm warning -->
      <div v-if="confirmStep" class="alert alert-error mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none"
          viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <div>
          <p class="font-bold">Confirm deletion</p>
          <p class="text-sm">
            This will permanently delete
            <strong>{{ totalSelectedRecords.toLocaleString() }} records</strong>
            from {{ selectedTopics.size }} topic(s). This action cannot be undone.
          </p>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn" @click="confirmStep ? cancelConfirm() : $emit('close')" :disabled="isDeleting">
          {{ confirmStep ? 'Back' : 'Close' }}
        </button>
        <button class="btn btn-error" :disabled="selectedTopics.size === 0 || isDeleting" @click="handleDelete">
          <span v-if="isDeleting" class="loading loading-spinner loading-xs"></span>
          <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24"
            stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          {{ confirmStep ? 'Delete Now' : 'Delete Selected' }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="$emit('close')">close</button>
    </form>
  </dialog>
</template>
