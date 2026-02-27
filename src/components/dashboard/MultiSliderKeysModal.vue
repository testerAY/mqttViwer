<script setup lang="ts">
import { ref, watch } from 'vue';
import type { MultiSliderItem } from '../../types/dashboard';
import { useDashboardStore } from '../../stores/useDashboardStore';

const props = defineProps<{
  open: boolean;
  widgetId: string | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const dashboardStore = useDashboardStore();
const localSliders = ref<MultiSliderItem[]>([]);
const validationError = ref('');

watch(() => props.open, (isOpen) => {
  if (isOpen && props.widgetId) {
    const item = dashboardStore.layout.find(i => i.widget.id === props.widgetId);
    if (item) {
      localSliders.value = JSON.parse(JSON.stringify(item.widget.settings?.sliders ?? []));
    } else {
      localSliders.value = [];
    }
    validationError.value = '';
  }
});

const addSlider = () => {
  localSliders.value.push({
    key: '',
    label: '',
    min: 0,
    max: 100,
    step: 1,
    defaultValue: 0,
  });
};

const removeSlider = (index: number) => {
  localSliders.value.splice(index, 1);
};

const validate = (): boolean => {
  for (const s of localSliders.value) {
    if (!s.key.trim()) {
      validationError.value = 'キーが空のスライダーがあります。';
      return false;
    }
  }
  const keys = localSliders.value.map(s => s.key.trim());
  const unique = new Set(keys);
  if (unique.size !== keys.length) {
    validationError.value = 'キーが重複しています。';
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
  updatedConfig.settings.sliders = localSliders.value.map(s => ({
    ...s,
    key: s.key.trim(),
    label: s.label.trim(),
  }));

  dashboardStore.updateWidget(props.widgetId, updatedConfig);
  emit('close');
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-4xl">
      <h3 class="font-bold text-lg mb-4">Slider Key Configuration</h3>

      <div class="mb-4">
        <button class="btn btn-sm btn-primary" @click="addSlider">+ Add Slider</button>
      </div>

      <div v-if="localSliders.length > 0" class="overflow-x-auto">
        <table class="table table-sm">
          <thead>
            <tr>
              <th>Key</th>
              <th>Label</th>
              <th>Min</th>
              <th>Max</th>
              <th>Step</th>
              <th>Default</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(slider, idx) in localSliders" :key="idx">
              <td>
                <input
                  v-model="slider.key"
                  type="text"
                  class="input input-bordered input-sm w-24 font-mono"
                  placeholder="x"
                />
              </td>
              <td>
                <input
                  v-model="slider.label"
                  type="text"
                  class="input input-bordered input-sm w-32"
                  placeholder="X軸"
                />
              </td>
              <td>
                <input
                  v-model.number="slider.min"
                  type="number"
                  class="input input-bordered input-sm w-20"
                />
              </td>
              <td>
                <input
                  v-model.number="slider.max"
                  type="number"
                  class="input input-bordered input-sm w-20"
                />
              </td>
              <td>
                <input
                  v-model.number="slider.step"
                  type="number"
                  class="input input-bordered input-sm w-20"
                  min="0"
                />
              </td>
              <td>
                <input
                  v-model.number="slider.defaultValue"
                  type="number"
                  class="input input-bordered input-sm w-20"
                />
              </td>
              <td>
                <button class="btn btn-xs btn-error btn-outline" @click="removeSlider(idx)">
                  Delete
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else class="text-center text-sm opacity-60 p-8 border border-dashed rounded-md">
        スライダーが未定義です。「+ Add Slider」で追加してください。
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
