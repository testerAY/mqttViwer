<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { useMqttStore } from '../../stores/useMqttStore';
import type { DataMapping } from '../../types/dashboard';
import { invoke } from '@tauri-apps/api/core';
import { extractValue } from '../../utils/jsonExtractor';

const props = defineProps<{
    open: boolean;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();

const dashboardStore = useDashboardStore();
const mqttStore = useMqttStore();

const selectedMappingId = ref<string | null>(null);
const isEditing = ref(false);
const localMapping = ref<DataMapping | null>(null);
const availableTopics = ref<string[]>([]);
const previewValue = ref<any>(null);

const mappings = computed(() => dashboardStore.dataMappings);

const selectedMapping = computed(() =>
    mappings.value.find(m => m.id === selectedMappingId.value)
);

const modalTitle = computed(() => {
    if (isEditing.value) {
        return selectedMappingId.value ? 'Edit Mapping' : 'New Mapping';
    }
    return localMapping.value?.name || '';
});

watch(() => props.open, (isOpen) => {
    if (isOpen) {
        fetchTopics();
        if (mappings.value.length > 0 && !selectedMappingId.value) {
            selectedMappingId.value = mappings.value[0].id;
        }
    }
});

watch(selectedMappingId, (newId) => {
    if (newId) {
        const m = mappings.value.find(m => m.id === newId);
        if (m) {
            localMapping.value = JSON.parse(JSON.stringify(m));
            isEditing.value = false;
        }
    } else {
        localMapping.value = null;
        isEditing.value = false;
    }
});

const fetchTopics = async () => {
    try {
        const topics = await invoke<string[]>('get_distinct_topics');
        availableTopics.value = topics;
    } catch (error) {
        console.error('Failed to fetch topics:', error);
    }
};

const createNew = () => {
    const newMapping: DataMapping = {
        id: crypto.randomUUID(),
        name: 'New Mapping',
        type: 'sub',
        topic: '',
        valueKey: '',
        description: ''
    };
    localMapping.value = newMapping;
    selectedMappingId.value = null;
    isEditing.value = true;
};

const editSelected = () => {
    if (selectedMapping.value) {
        localMapping.value = JSON.parse(JSON.stringify(selectedMapping.value));
        isEditing.value = true;
    }
};

const save = () => {
    if (!localMapping.value) return;

    if (selectedMappingId.value) {
        dashboardStore.updateDataMapping(localMapping.value.id, localMapping.value);
    } else {
        const { id, ...rest } = localMapping.value;
        dashboardStore.addDataMapping(rest);
        // Select the last created one
        setTimeout(() => {
            const last = mappings.value[mappings.value.length - 1];
            if (last) selectedMappingId.value = last.id;
        }, 100);
    }
    isEditing.value = false;
    dashboardStore.saveLayout();
};

const cancel = () => {
    if (selectedMappingId.value) {
        const m = mappings.value.find(m => m.id === selectedMappingId.value);
        if (m) localMapping.value = JSON.parse(JSON.stringify(m));
        isEditing.value = false;
    } else {
        if (mappings.value.length > 0) {
            selectedMappingId.value = mappings.value[0].id;
        } else {
            localMapping.value = null;
        }
        isEditing.value = false;
    }
};

const remove = () => {
    if (selectedMappingId.value && confirm('Are you sure you want to delete this mapping?')) {
        dashboardStore.removeDataMapping(selectedMappingId.value);
        dashboardStore.saveLayout();
        selectedMappingId.value = mappings.value.length > 0 ? mappings.value[0].id : null;
    }
};

// Preview Logic
watch(
    () => [localMapping.value?.topic, localMapping.value?.valueKey],
    () => {
        updatePreview();
    },
    { deep: true }
);

const updatePreview = () => {
    if (!localMapping.value?.topic) {
        previewValue.value = null;
        return;
    }
    const lastMessage = mqttStore.lastMessages[localMapping.value.topic];
    if (lastMessage) {
        try {
            const payloadJson = JSON.parse(lastMessage.payload);
            const valueKey = localMapping.value.valueKey || '';
            previewValue.value = {
                raw: payloadJson,
                extracted: extractValue(payloadJson, valueKey),
            };
        } catch (e) {
            previewValue.value = {
                raw: lastMessage.payload,
                extracted: lastMessage.payload,
            };
        }
    } else {
        previewValue.value = null;
    }
};
</script>

<template>
    <dialog class="modal" :class="{ 'modal-open': open }">
        <div class="modal-box w-11/12 max-w-5xl h-[80vh] flex flex-col p-0 overflow-hidden">
            <!-- Header -->
            <div class="p-4 bg-base-200 flex justify-between items-center shrink-0">
                <h3 class="font-bold text-lg">Data Mappings</h3>
                <button class="btn btn-sm btn-ghost" @click="$emit('close')">✕</button>
            </div>

            <!-- Content -->
            <div class="flex-1 flex overflow-hidden">
                <!-- Sidebar List -->
                <div class="w-1/3 border-r border-base-300 flex flex-col bg-base-100">
                    <div class="p-2 border-b border-base-300">
                        <button class="btn btn-primary btn-sm w-full" @click="createNew"
                            :disabled="isEditing && !selectedMappingId">+ New Mapping</button>
                    </div>
                    <div class="flex-1 overflow-y-auto p-2 space-y-1">
                        <div v-for="m in mappings" :key="m.id"
                            class="p-2 rounded cursor-pointer hover:bg-base-200 transition-colors"
                            :class="{ 'bg-primary text-primary-content hover:bg-primary hover:text-primary-content': selectedMappingId === m.id }"
                            @click="!isEditing && (selectedMappingId = m.id)">
                            <div class="font-bold truncate">{{ m.name }}</div>
                            <div class="text-xs opacity-70 truncate">{{ m.topic }}</div>
                        </div>
                        <div v-if="mappings.length === 0" class="text-center p-4 opacity-50">
                            No mappings defined.
                        </div>
                    </div>
                </div>

                <!-- Detail View -->
                <div class="flex-1 flex flex-col overflow-y-auto bg-base-100 p-6">
                    <template v-if="localMapping">
                        <div class="flex justify-between items-start mb-6">
                            <h2 class="text-2xl font-bold">{{ modalTitle }}</h2>
                            <div class="flex gap-2">
                                <template v-if="!isEditing">
                                    <button class="btn btn-sm btn-outline btn-error" @click="remove">Delete</button>
                                    <button class="btn btn-sm btn-outline" @click="editSelected">Edit</button>
                                </template>
                                <template v-else>
                                    <button class="btn btn-sm" @click="cancel">Cancel</button>
                                    <button class="btn btn-sm btn-primary" @click="save">Save</button>
                                </template>
                            </div>
                        </div>

                        <div class="space-y-4" :class="{ 'opacity-80 pointer-events-none': !isEditing }">
                            <div class="form-control w-full">
                                <label class="label"><span class="label-text">Name</span></label>
                                <input v-model="localMapping.name" type="text" class="input input-bordered"
                                    placeholder="e.g. Living Room Temp" />
                            </div>

                            <div class="flex gap-4">
                                <div class="form-control w-1/3">
                                    <label class="label"><span class="label-text">Type</span></label>
                                    <select v-model="localMapping.type" class="select select-bordered">
                                        <option value="sub">Subscribe (Read)</option>
                                        <option value="pub">Publish (Write)</option>
                                    </select>
                                </div>
                                <div class="form-control flex-1">
                                    <label class="label"><span class="label-text">MQTT Topic</span></label>
                                    <input v-model="localMapping.topic" type="text"
                                        class="input input-bordered font-mono" list="mapping-topic-list" />
                                    <datalist id="mapping-topic-list">
                                        <option v-for="t in availableTopics" :key="t" :value="t" />
                                    </datalist>
                                </div>
                            </div>

                            <div class="form-control w-full">
                                <label class="label"><span class="label-text">Value Key (JSON Path)</span></label>
                                <input v-model="localMapping.valueKey" type="text"
                                    class="input input-bordered font-mono" placeholder="e.g. sensor.temperature" />
                                <label class="label"><span class="label-text-alt">Leave empty for raw
                                        value</span></label>
                            </div>

                            <div class="form-control w-full">
                                <label class="label"><span class="label-text">Description</span></label>
                                <textarea v-model="localMapping.description" class="textarea textarea-bordered h-24"
                                    placeholder="Optional notes..."></textarea>
                            </div>

                            <!-- Preview -->
                            <div class="divider">Preview</div>
                            <div v-if="previewValue" class="space-y-2 p-4 bg-base-200 rounded-lg">
                                <div class="flex justify-between">
                                    <span class="font-bold">Extracted Value:</span>
                                    <span class="font-mono bg-base-300 px-2 rounded">{{ previewValue.extracted }}</span>
                                </div>
                                <div class="collapse collapse-arrow border border-base-300 bg-base-100 rounded-box">
                                    <input type="checkbox" />
                                    <div class="collapse-title text-sm font-medium">Raw Payload</div>
                                    <div class="collapse-content">
                                        <pre
                                            class="text-xs overflow-x-auto">{{ JSON.stringify(previewValue.raw, null, 2) }}</pre>
                                    </div>
                                </div>
                            </div>
                            <div v-else class="text-center p-4 border border-dashed rounded opacity-50">
                                No data received on this topic yet.
                            </div>
                        </div>
                    </template>
                    <div v-else class="flex-1 flex items-center justify-center text-opacity-50">
                        Select a mapping to view details or create a new one.
                    </div>
                </div>
            </div>
        </div>
        <form method="dialog" class="modal-backdrop">
            <button @click="$emit('close')">close</button>
        </form>
    </dialog>
</template>
