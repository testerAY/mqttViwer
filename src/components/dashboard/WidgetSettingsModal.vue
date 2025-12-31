<script setup lang="ts">
import { ref, watch } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useDashboardStore } from '../../stores/useDashboardStore';

const props = defineProps<{
  open: boolean;
  widgetId: string | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const dashboardStore = useDashboardStore();
const activeTab = ref('general');

// Local state for editing
interface EditableWidgetConfig extends WidgetConfig {
  settings: Record<string, any>;
}
const localConfig = ref<EditableWidgetConfig | null>(null);

interface EditableLayout {
  x: number;
  y: number;
  w: number;
  h: number;
}
const localLayout = ref<EditableLayout | null>(null);

watch(() => props.open, (isOpen) => {
  if (isOpen && props.widgetId) {
    const item = dashboardStore.layout.find(i => i.widget.id === props.widgetId);
    if (item) {
      // Deep clone to avoid mutation
      const widgetClone = JSON.parse(JSON.stringify(item.widget));
      // Ensure settings object exists
      if (!widgetClone.settings) {
        widgetClone.settings = {};
      }
      localConfig.value = widgetClone as EditableWidgetConfig;

      localLayout.value = {
        x: item.x,
        y: item.y,
        w: item.w,
        h: item.h
      };
    }
  }
});

const handleSave = () => {
  if (localConfig.value && props.widgetId) {
    dashboardStore.updateWidget(props.widgetId, localConfig.value);
    if (localLayout.value) {
      dashboardStore.updateLayoutItem(props.widgetId, localLayout.value);
    }
    emit('close');
  }
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-3xl">
      <h3 class="font-bold text-lg mb-4">Widget Settings</h3>

      <div class="tabs tabs-bordered mb-4">
        <a class="tab" :class="{ 'tab-active': activeTab === 'general' }" @click="activeTab = 'general'">General</a>
        <a class="tab" :class="{ 'tab-active': activeTab === 'style' }" @click="activeTab = 'style'">Style</a>
        <a class="tab" :class="{ 'tab-active': activeTab === 'data' }" @click="activeTab = 'data'">Data Mapping</a>
        <a class="tab" :class="{ 'tab-active': activeTab === 'layout' }" @click="activeTab = 'layout'">Layout</a>
      </div>
      
      <div v-if="localConfig" class="py-4 h-96 overflow-y-auto">
        
        <!-- General Tab -->
        <div v-show="activeTab === 'general'" class="flex flex-col gap-4">
          <div class="form-control w-full">
            <label class="label"><span class="label-text">Title</span></label>
            <input v-model="localConfig.title" type="text" class="input input-bordered" />
          </div>
          
          <div class="form-control w-full">
            <label class="label"><span class="label-text">MQTT Topic</span></label>
            <input v-model="localConfig.topic" type="text" class="input input-bordered font-mono" />
          </div>

          <div class="form-control w-full">
            <label class="label"><span class="label-text">Widget Type</span></label>
            <select v-model="localConfig.type" class="select select-bordered" disabled>
              <option value="value-display">Value Display</option>
              <option value="switch">Switch</option>
              <option value="chart">Chart</option>
              <option value="gauge">Gauge</option>
            </select>
            <label class="label">
              <span class="label-text-alt">Type cannot be changed after creation</span>
            </label>
          </div>
        </div>

        <!-- Style Tab -->
        <div v-show="activeTab === 'style'" class="flex flex-col gap-4">
           <!-- Common Settings -->
           <div class="divider">Common</div>
           <div class="form-control w-full">
             <label class="label"><span class="label-text">Unit</span></label>
             <input v-model="localConfig.settings.unit" type="text" class="input input-bordered" placeholder="e.g. °C, %" />
           </div>

           <!-- Type Specific -->
           <template v-if="localConfig.type === 'gauge'">
             <div class="divider">Gauge Settings</div>
             <div class="flex gap-4">
               <div class="form-control w-full">
                 <label class="label"><span class="label-text">Min Value</span></label>
                 <input v-model.number="localConfig.settings.min" type="number" class="input input-bordered" />
               </div>
               <div class="form-control w-full">
                 <label class="label"><span class="label-text">Max Value</span></label>
                 <input v-model.number="localConfig.settings.max" type="number" class="input input-bordered" />
               </div>
             </div>
           </template>

           <template v-if="localConfig.type === 'switch'">
             <div class="divider">Switch Settings</div>
             <div class="flex gap-4">
               <div class="form-control w-full">
                 <label class="label"><span class="label-text">ON Payload</span></label>
                 <input v-model="localConfig.settings.onPayload" type="text" class="input input-bordered" />
               </div>
               <div class="form-control w-full">
                 <label class="label"><span class="label-text">OFF Payload</span></label>
                 <input v-model="localConfig.settings.offPayload" type="text" class="input input-bordered" />
               </div>
             </div>
           </template>
        </div>

        <!-- Data Mapping Tab -->
        <div v-show="activeTab === 'data'" class="flex flex-col gap-4">
          <div class="alert alert-info text-sm">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
            <span>Extract values from JSON payload using object keys (e.g. "sensor.temp"). Leave empty for raw value.</span>
          </div>
          
          <div class="form-control w-full">
             <label class="label"><span class="label-text">Value Key</span></label>
             <input v-model="localConfig.settings.valueKey" type="text" class="input input-bordered font-mono" placeholder="e.g. temperature" />
          </div>

          <template v-if="localConfig.type === 'chart'">
             <div class="divider">Chart Axes</div>
             <div class="form-control w-full">
               <label class="label"><span class="label-text">X-Axis Key (Timestamp)</span></label>
               <input v-model="localConfig.settings.xKey" type="text" class="input input-bordered font-mono" placeholder="e.g. timestamp" />
             </div>
             <div class="form-control w-full">
               <label class="label"><span class="label-text">Y-Axis Key (Value)</span></label>
               <input v-model="localConfig.settings.yKey" type="text" class="input input-bordered font-mono" placeholder="e.g. value" />
             </div>
          </template>
        </div>
        
        <!-- Layout Tab -->
        <div v-if="localLayout && activeTab === 'layout'" class="flex flex-col gap-4">
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control w-full">
               <label class="label"><span class="label-text">X Position</span></label>
               <input v-model.number="localLayout.x" type="number" class="input input-bordered" min="0" />
            </div>
            <div class="form-control w-full">
               <label class="label"><span class="label-text">Y Position</span></label>
               <input v-model.number="localLayout.y" type="number" class="input input-bordered" min="0" />
            </div>
            <div class="form-control w-full">
               <label class="label"><span class="label-text">Width</span></label>
               <input v-model.number="localLayout.w" type="number" class="input input-bordered" min="1" max="12" />
            </div>
            <div class="form-control w-full">
               <label class="label"><span class="label-text">Height</span></label>
               <input v-model.number="localLayout.h" type="number" class="input input-bordered" min="1" />
            </div>
          </div>
        </div>

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
