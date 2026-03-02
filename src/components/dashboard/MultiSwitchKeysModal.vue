<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import type { MultiSwitchItem, ProtoField } from '../../types/dashboard';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { getRootMessageFields } from '../../utils/protoUtils';

const props = defineProps<{
  open: boolean;
  widgetId: string | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const dashboardStore = useDashboardStore();
const localSwitches = ref<MultiSwitchItem[]>([]);
const validationError = ref('');

const widgetMapping = computed(() => {
  const item = dashboardStore.layout.find(i => i.widget.id === props.widgetId);
  if (!item?.widget.mappingId) return null;
  return dashboardStore.getDataMappingById(item.widget.mappingId);
});
const isBinaryMode = computed(() => widgetMapping.value?.valueType === 'binary');
const protoFields = computed<ProtoField[]>(() => {
  if (!isBinaryMode.value || !widgetMapping.value?.protoSchema) return [];
  return getRootMessageFields(widgetMapping.value.protoSchema);
});

watch(() => props.open, (isOpen) => {
  if (isOpen && props.widgetId) {
    const item = dashboardStore.layout.find(i => i.widget.id === props.widgetId);
    if (item) {
      localSwitches.value = JSON.parse(JSON.stringify(item.widget.settings?.switches ?? []));
    } else {
      localSwitches.value = [];
    }
    validationError.value = '';
  }
});

const addSwitch = () => {
  localSwitches.value.push({ key: '', label: '' });
};

const removeSwitch = (index: number) => {
  localSwitches.value.splice(index, 1);
};

const validate = (): boolean => {
  for (const s of localSwitches.value) {
    if (!s.key.trim()) {
      validationError.value = 'Some switches have an empty key.';
      return false;
    }
  }
  const keys = localSwitches.value.map(s => s.key.trim());
  const unique = new Set(keys);
  if (unique.size !== keys.length) {
    validationError.value = 'Duplicate keys are not allowed.';
    return false;
  }
  validationError.value = '';
  return true;
};

const handleSave = () => {
  if (!validate() || !props.widgetId) return;

  const item = dashboardStore.layout.find(i => i.widget.id === props.widgetId);
  if (!item) return;

  const updatedConfig = JSON.parse(JSON.stringify(item.widget));
  if (!updatedConfig.settings) updatedConfig.settings = {};
  updatedConfig.settings.switches = localSwitches.value.map(s => ({
    key: s.key.trim(),
    label: s.label.trim(),
  }));

  dashboardStore.updateWidget(props.widgetId, updatedConfig);
  emit('close');
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-2xl">
      <h3 class="font-bold text-lg mb-4">Switch Key Configuration</h3>

      <div class="mb-4">
        <button class="btn btn-sm btn-primary" @click="addSwitch">+ Add Switch</button>
      </div>

      <div v-if="localSwitches.length > 0" class="overflow-x-auto">
        <table class="table table-sm">
          <thead>
            <tr>
              <th>Key / Field</th>
              <th>Label</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(sw, idx) in localSwitches" :key="idx">
              <td>
                <select v-if="isBinaryMode" v-model="sw.key"
                  class="select select-bordered select-sm w-36 font-mono">
                  <option value="">-- Field --</option>
                  <option v-for="f in protoFields" :key="f.name" :value="f.name">
                    {{ f.name }} ({{ f.type }})
                  </option>
                </select>
                <input v-else
                  v-model="sw.key"
                  type="text"
                  class="input input-bordered input-sm w-32 font-mono"
                  placeholder="x"
                />
              </td>
              <td>
                <input
                  v-model="sw.label"
                  type="text"
                  class="input input-bordered input-sm w-48"
                  placeholder="Motor A"
                />
              </td>
              <td>
                <button class="btn btn-xs btn-error btn-outline" @click="removeSwitch(idx)">
                  Delete
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="text-center text-sm opacity-60 p-8 border border-dashed rounded-md">
        No switches defined. Click "+ Add Switch" to add one.
      </div>

      <div v-if="validationError" class="alert alert-error mt-4 text-sm py-2">
        {{ validationError }}
      </div>

      <div class="modal-action">
        <button class="btn" @click="$emit('close')">Cancel</button>
        <button class="btn btn-primary" @click="handleSave">Save</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="$emit('close')">close</button>
    </form>
  </dialog>
</template>
