<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { useMqttStore } from '../../stores/useMqttStore';
import type { DataMapping, ProtoSchema, ProtoMessage, ProtoField, ProtoFieldType, ProtoFieldRule } from '../../types/dashboard';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { extractValue } from '../../utils/jsonExtractor';
import { generateProtoText, parseProtoText, decodeProtoPayload, validateProtoSchema } from '../../utils/protoUtils';

const props = defineProps<{
    open: boolean;
}>();

defineEmits<{
    (e: 'close'): void;
}>();

const dashboardStore = useDashboardStore();
const mqttStore = useMqttStore();

const selectedMappingId = ref<string | null>(null);
const isEditing = ref(false);
const localMapping = ref<DataMapping | null>(null);
const availableTopics = ref<string[]>([]);
const previewValue = ref<any>(null);

// Proto editor state
const protoValidationErrors = ref<string[]>([]);
const protoSaveStatus = ref<'idle' | 'saving' | 'saved' | 'error'>('idle');
const protoSaveError = ref('');
const selectedMessageIndex = ref(0);
const editingFieldIndex = ref<number | null>(null);
const editingField = ref<ProtoField | null>(null);
const protoDecodePreview = ref<Record<string, any> | null>(null);
const protoDecodeError = ref('');

const PROTO_FIELD_TYPES: ProtoFieldType[] = [
    'double', 'float',
    'int32', 'int64', 'uint32', 'uint64',
    'sint32', 'sint64',
    'fixed32', 'fixed64', 'sfixed32', 'sfixed64',
    'bool', 'string', 'bytes',
];

const PROTO_FIELD_RULES: ProtoFieldRule[] = ['optional', 'required', 'repeated'];

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

const isBinaryMode = computed(() => localMapping.value?.valueType === 'binary');

const currentProtoSchema = computed<ProtoSchema>(() => {
    if (!localMapping.value?.protoSchema) {
        return { messages: [], rootMessage: '' };
    }
    return localMapping.value.protoSchema;
});

const currentMessage = computed<ProtoMessage | null>(() => {
    const msgs = currentProtoSchema.value.messages;
    if (msgs.length === 0 || selectedMessageIndex.value >= msgs.length) return null;
    return msgs[selectedMessageIndex.value];
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
    resetProtoEditorState();
});

// Initialize protoSchema when switching to binary mode
watch(() => localMapping.value?.valueType, (vt) => {
    if (vt === 'binary' && localMapping.value && !localMapping.value.protoSchema) {
        localMapping.value.protoSchema = {
            packageName: '',
            rootMessage: 'MyMessage',
            messages: [{ name: 'MyMessage', fields: [] }],
        };
        selectedMessageIndex.value = 0;
    }
    if (vt === 'binary') {
        updateProtoPreview();
    } else {
        updatePreview();
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

const resetProtoEditorState = () => {
    selectedMessageIndex.value = 0;
    editingFieldIndex.value = null;
    editingField.value = null;
    protoValidationErrors.value = [];
    protoDecodePreview.value = null;
    protoDecodeError.value = '';
    protoSaveStatus.value = 'idle';
};

const createNew = () => {
    const newMapping: DataMapping = {
        id: crypto.randomUUID(),
        name: 'New Mapping',
        type: 'sub',
        topic: '',
        valueType: 'json',
        valueKey: '',
        description: ''
    };
    localMapping.value = newMapping;
    selectedMappingId.value = null;
    isEditing.value = true;
    resetProtoEditorState();
};

const editSelected = () => {
    if (selectedMapping.value) {
        localMapping.value = JSON.parse(JSON.stringify(selectedMapping.value));
        isEditing.value = true;
    }
};

const save = () => {
    if (!localMapping.value) return;

    if (localMapping.value.valueType === 'binary') {
        const errors = validateProtoSchema(localMapping.value.protoSchema ?? { messages: [], rootMessage: '' });
        protoValidationErrors.value = errors;
        if (errors.length > 0) return;
    }

    if (selectedMappingId.value) {
        dashboardStore.updateDataMapping(localMapping.value.id, localMapping.value);
    } else {
        const { id, ...rest } = localMapping.value;
        dashboardStore.addDataMapping(rest);
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
    resetProtoEditorState();
};

const remove = () => {
    if (selectedMappingId.value && confirm('Are you sure you want to delete this mapping?')) {
        dashboardStore.removeDataMapping(selectedMappingId.value);
        dashboardStore.saveLayout();
        selectedMappingId.value = mappings.value.length > 0 ? mappings.value[0].id : null;
    }
};

// --- Proto schema editor functions ---

const addMessage = () => {
    if (!localMapping.value) return;
    if (!localMapping.value.protoSchema) {
        localMapping.value.protoSchema = { messages: [], rootMessage: '' };
    }
    const newMsg: ProtoMessage = { name: 'NewMessage', fields: [] };
    localMapping.value.protoSchema.messages.push(newMsg);
    selectedMessageIndex.value = localMapping.value.protoSchema.messages.length - 1;
};

const removeMessage = (idx: number) => {
    if (!localMapping.value?.protoSchema) return;
    const msgs = localMapping.value.protoSchema.messages;
    const removedName = msgs[idx].name;
    msgs.splice(idx, 1);
    if (localMapping.value.protoSchema.rootMessage === removedName && msgs.length > 0) {
        localMapping.value.protoSchema.rootMessage = msgs[0].name;
    }
    selectedMessageIndex.value = Math.max(0, idx - 1);
    editingFieldIndex.value = null;
    editingField.value = null;
};

const addField = () => {
    if (!currentMessage.value) return;
    const usedIds = new Set(currentMessage.value.fields.map(f => f.id));
    let nextId = 1;
    while (usedIds.has(nextId)) nextId++;
    const newField: ProtoField = { id: nextId, name: 'newField', type: 'string', rule: 'optional' };
    currentMessage.value.fields.push(newField);
    editingFieldIndex.value = currentMessage.value.fields.length - 1;
    editingField.value = { ...newField };
};

const startEditField = (idx: number) => {
    if (!currentMessage.value) return;
    editingFieldIndex.value = idx;
    editingField.value = { ...currentMessage.value.fields[idx] };
};

const saveEditField = () => {
    if (editingFieldIndex.value === null || !editingField.value || !currentMessage.value) return;
    currentMessage.value.fields[editingFieldIndex.value] = { ...editingField.value };
    editingFieldIndex.value = null;
    editingField.value = null;
};

const cancelEditField = () => {
    // If this was a newly added field that was never saved, remove it
    if (editingFieldIndex.value !== null && currentMessage.value) {
        const field = currentMessage.value.fields[editingFieldIndex.value];
        if (field.name === 'newField') {
            currentMessage.value.fields.splice(editingFieldIndex.value, 1);
        }
    }
    editingFieldIndex.value = null;
    editingField.value = null;
};

const removeField = (idx: number) => {
    if (!currentMessage.value) return;
    if (editingFieldIndex.value === idx) {
        editingFieldIndex.value = null;
        editingField.value = null;
    }
    currentMessage.value.fields.splice(idx, 1);
};

const exportProtoFile = async () => {
    if (!localMapping.value?.protoSchema) return;
    const errors = validateProtoSchema(localMapping.value.protoSchema);
    protoValidationErrors.value = errors;
    if (errors.length > 0) return;

    const protoText = generateProtoText(localMapping.value.protoSchema);
    const suggestedName = localMapping.value.name.replace(/\s+/g, '_').toLowerCase();

    protoSaveStatus.value = 'saving';
    protoSaveError.value = '';
    try {
        await invoke('save_proto_file', { filename: suggestedName, content: protoText });
        protoSaveStatus.value = 'saved';
        setTimeout(() => { protoSaveStatus.value = 'idle'; }, 2000);
    } catch (e) {
        protoSaveStatus.value = 'error';
        protoSaveError.value = String(e);
    }
};

const importProtoFile = async () => {
    const path = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: 'Proto Files', extensions: ['proto'] }],
    });

    if (!path || !localMapping.value) return;

    try {
        const content = await readTextFile(path as string);
        const parsed = parseProtoText(content);

        if (parsed.messages.length === 0) {
            protoValidationErrors.value = ['No messages found in the selected .proto file.'];
            return;
        }

        localMapping.value.protoSchema = parsed;
        selectedMessageIndex.value = 0;
        protoValidationErrors.value = [];
        editingFieldIndex.value = null;
        editingField.value = null;
    } catch (e) {
        protoValidationErrors.value = [`Failed to import file: ${e}`];
    }
};

const updateProtoPreview = () => {
    protoDecodePreview.value = null;
    protoDecodeError.value = '';

    if (!localMapping.value?.topic || !localMapping.value.protoSchema) return;

    const lastMsg = mqttStore.lastMessages[localMapping.value.topic];
    if (!lastMsg) return;

    if (lastMsg.payload_encoding !== 'base64') {
        protoDecodeError.value = 'Last received message is not binary (payload_encoding is not base64).';
        return;
    }

    try {
        protoDecodePreview.value = decodeProtoPayload(lastMsg.payload, localMapping.value.protoSchema);
    } catch (e) {
        protoDecodeError.value = String(e);
    }
};

// --- Preview logic ---

watch(
    () => [localMapping.value?.topic, localMapping.value?.valueKey, localMapping.value?.valueType, localMapping.value?.protoSchema],
    () => {
        if (isBinaryMode.value) {
            updateProtoPreview();
        } else {
            updatePreview();
        }
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
        if (localMapping.value.valueType === 'value') {
            previewValue.value = {
                raw: lastMessage.payload,
                extracted: lastMessage.payload,
            };
        } else {
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
                                        <option value="both">Both (Read & Write)</option>
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

                            <!-- Value Type -->
                            <div class="form-control w-full">
                                <label class="label"><span class="label-text">Value Type</span></label>
                                <div class="flex gap-4 flex-wrap">
                                    <label class="label cursor-pointer gap-2">
                                        <input type="radio" v-model="localMapping.valueType" value="json"
                                            class="radio radio-sm" />
                                        <span class="label-text">JSON</span>
                                    </label>
                                    <label class="label cursor-pointer gap-2">
                                        <input type="radio" v-model="localMapping.valueType" value="value"
                                            class="radio radio-sm" />
                                        <span class="label-text">Value</span>
                                    </label>
                                    <label class="label cursor-pointer gap-2">
                                        <input type="radio" v-model="localMapping.valueType" value="binary"
                                            class="radio radio-sm" />
                                        <span class="label-text">Binary (Protobuf)</span>
                                    </label>
                                </div>
                            </div>

                            <!-- JSON value key (only for json mode) -->
                            <div class="form-control w-full" v-if="localMapping.valueType === 'json'">
                                <label class="label"><span class="label-text">Value Key (JSON Path)</span></label>
                                <input v-model="localMapping.valueKey" type="text"
                                    class="input input-bordered font-mono" placeholder="e.g. sensor.temperature" />
                                <label class="label"><span class="label-text-alt">Leave empty for root
                                        value</span></label>
                            </div>

                            <!-- Binary / Protobuf Schema Editor -->
                            <div v-if="isBinaryMode" class="border border-base-300 rounded-box p-4 space-y-4">
                                <div class="flex items-center justify-between">
                                    <h3 class="font-bold text-base">Protobuf Schema</h3>
                                    <div class="flex gap-2">
                                        <button class="btn btn-xs btn-outline" @click="importProtoFile">
                                            Import .proto
                                        </button>
                                        <button class="btn btn-xs btn-outline btn-success" @click="exportProtoFile">
                                            <span v-if="protoSaveStatus === 'saving'">Saving...</span>
                                            <span v-else-if="protoSaveStatus === 'saved'">✓ Saved</span>
                                            <span v-else>Export .proto</span>
                                        </button>
                                    </div>
                                </div>

                                <div v-if="protoSaveStatus === 'error'" class="alert alert-error p-2 text-xs">
                                    {{ protoSaveError }}
                                </div>

                                <!-- Package name -->
                                <div class="form-control">
                                    <label class="label"><span class="label-text text-xs">Package Name
                                            (optional)</span></label>
                                    <input v-model="localMapping.protoSchema!.packageName" type="text"
                                        class="input input-bordered input-sm font-mono" placeholder="e.g. sensors" />
                                </div>

                                <!-- Message tabs -->
                                <div>
                                    <label class="label"><span class="label-text text-xs">Messages</span></label>
                                    <div class="flex gap-1 flex-wrap items-center">
                                        <button v-for="(msg, idx) in currentProtoSchema.messages" :key="idx"
                                            class="btn btn-xs"
                                            :class="selectedMessageIndex === idx ? 'btn-primary' : 'btn-outline'"
                                            @click="selectedMessageIndex = idx; editingFieldIndex = null; editingField = null">
                                            {{ msg.name }}
                                            <span v-if="localMapping.protoSchema?.rootMessage === msg.name"
                                                class="badge badge-xs badge-accent ml-1">root</span>
                                        </button>
                                        <button class="btn btn-xs btn-ghost" @click="addMessage">
                                            + Add
                                        </button>
                                    </div>
                                </div>

                                <!-- Selected message editor -->
                                <template v-if="currentMessage">
                                    <div class="flex gap-2 items-end">
                                        <div class="form-control flex-1">
                                            <label class="label"><span class="label-text text-xs">Message
                                                    Name</span></label>
                                            <input v-model="currentMessage.name" type="text"
                                                class="input input-bordered input-sm font-mono" />
                                        </div>
                                        <label class="label cursor-pointer gap-2 pb-2">
                                            <span class="label-text text-xs">Root</span>
                                            <input type="checkbox" class="checkbox checkbox-sm"
                                                :checked="localMapping.protoSchema?.rootMessage === currentMessage.name"
                                                @change="localMapping.protoSchema!.rootMessage = currentMessage!.name" />
                                        </label>
                                        <button class="btn btn-xs btn-error btn-outline mb-1"
                                            @click="removeMessage(selectedMessageIndex)"
                                            :disabled="currentProtoSchema.messages.length <= 1">
                                            Remove
                                        </button>
                                    </div>

                                    <!-- Fields table -->
                                    <div class="overflow-x-auto border border-base-300 rounded">
                                        <table class="table table-xs w-full">
                                            <thead class="bg-base-200">
                                                <tr>
                                                    <th class="w-16">Field #</th>
                                                    <th>Name</th>
                                                    <th>Type</th>
                                                    <th>Rule</th>
                                                    <th class="w-24"></th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                <tr v-for="(field, fIdx) in currentMessage.fields" :key="fIdx">
                                                    <template v-if="editingFieldIndex === fIdx && editingField">
                                                        <td>
                                                            <input v-model.number="editingField.id" type="number"
                                                                min="1"
                                                                class="input input-xs input-bordered w-16 font-mono" />
                                                        </td>
                                                        <td>
                                                            <input v-model="editingField.name" type="text"
                                                                class="input input-xs input-bordered font-mono w-full" />
                                                        </td>
                                                        <td>
                                                            <select v-model="editingField.type"
                                                                class="select select-xs select-bordered font-mono">
                                                                <option v-for="t in PROTO_FIELD_TYPES" :key="t"
                                                                    :value="t">{{ t }}</option>
                                                            </select>
                                                        </td>
                                                        <td>
                                                            <select v-model="editingField.rule"
                                                                class="select select-xs select-bordered">
                                                                <option v-for="r in PROTO_FIELD_RULES" :key="r"
                                                                    :value="r">{{ r }}</option>
                                                            </select>
                                                        </td>
                                                        <td>
                                                            <div class="flex gap-1">
                                                                <button class="btn btn-xs btn-primary"
                                                                    @click="saveEditField">OK</button>
                                                                <button class="btn btn-xs"
                                                                    @click="cancelEditField">✕</button>
                                                            </div>
                                                        </td>
                                                    </template>
                                                    <template v-else>
                                                        <td class="font-mono">{{ field.id }}</td>
                                                        <td class="font-mono">{{ field.name }}</td>
                                                        <td class="font-mono text-info">{{ field.type }}
                                                        </td>
                                                        <td class="opacity-70 text-xs">{{ field.rule }}</td>
                                                        <td>
                                                            <div class="flex gap-1">
                                                                <button class="btn btn-xs btn-ghost"
                                                                    @click="startEditField(fIdx)"
                                                                    :disabled="editingFieldIndex !== null">Edit</button>
                                                                <button class="btn btn-xs btn-ghost text-error"
                                                                    @click="removeField(fIdx)"
                                                                    :disabled="editingFieldIndex !== null">Del</button>
                                                            </div>
                                                        </td>
                                                    </template>
                                                </tr>
                                                <tr v-if="currentMessage.fields.length === 0">
                                                    <td colspan="5" class="text-center opacity-50 py-3 text-sm">
                                                        No fields defined. Click "+ Add Field" to add one.
                                                    </td>
                                                </tr>
                                            </tbody>
                                        </table>
                                    </div>

                                    <div class="flex gap-2 items-center">
                                        <button class="btn btn-sm btn-outline" @click="addField"
                                            :disabled="editingFieldIndex !== null">
                                            + Add Field
                                        </button>
                                    </div>
                                </template>

                                <!-- Value Key for binary (extract from decoded object) -->
                                <div class="form-control w-full">
                                    <label class="label">
                                        <span class="label-text text-xs">Value Key (field path after
                                            decode)</span>
                                    </label>
                                    <input v-model="localMapping.valueKey" type="text"
                                        class="input input-bordered input-sm font-mono"
                                        placeholder="e.g. temperature" />
                                    <label class="label"><span class="label-text-alt">Dot-notation path
                                            into the decoded object. Leave empty for full
                                            object.</span></label>
                                </div>

                                <!-- Validation errors -->
                                <div v-if="protoValidationErrors.length > 0" class="alert alert-error p-2">
                                    <ul class="list-disc list-inside text-xs space-y-1">
                                        <li v-for="err in protoValidationErrors" :key="err">{{ err }}
                                        </li>
                                    </ul>
                                </div>

                                <!-- Decode Preview -->
                                <div class="divider text-xs my-1">Decode Preview</div>
                                <div v-if="protoDecodePreview" class="p-3 bg-base-200 rounded-box space-y-2">
                                    <div v-if="localMapping.valueKey" class="flex justify-between items-center">
                                        <span class="text-sm font-bold">Extracted Value:</span>
                                        <span class="font-mono bg-base-300 px-2 py-1 rounded text-sm">{{
                                            extractValue(protoDecodePreview,
                                                localMapping.valueKey) }}</span>
                                    </div>
                                    <div class="collapse collapse-arrow border border-base-300 bg-base-100 rounded-box">
                                        <input type="checkbox" checked />
                                        <div class="collapse-title text-sm font-medium">Decoded Object
                                        </div>
                                        <div class="collapse-content">
                                            <pre
                                                class="text-xs overflow-x-auto">{{ JSON.stringify(protoDecodePreview, null, 2) }}</pre>
                                        </div>
                                    </div>
                                </div>
                                <div v-else-if="protoDecodeError" class="alert alert-warning p-2 text-xs">
                                    {{ protoDecodeError }}
                                </div>
                                <div v-else class="text-center p-3 border border-dashed rounded opacity-50 text-sm">
                                    No binary data received on this topic yet.
                                </div>
                            </div>

                            <div class="form-control w-full">
                                <label class="label"><span class="label-text">Description</span></label>
                                <textarea v-model="localMapping.description" class="textarea textarea-bordered h-24"
                                    placeholder="Optional notes..."></textarea>
                            </div>

                            <!-- Preview (JSON / Value mode only) -->
                            <template v-if="!isBinaryMode">
                                <div class="divider">Preview</div>
                                <div v-if="previewValue" class="space-y-2 p-4 bg-base-200 rounded-lg">
                                    <div class="flex justify-between">
                                        <span class="font-bold">Extracted Value:</span>
                                        <span class="font-mono bg-base-300 px-2 rounded">{{ previewValue.extracted
                                            }}</span>
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
                            </template>
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
