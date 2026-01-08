<script setup lang="ts">
import { ref, watch } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import { useDashboardStore } from '../../stores/useDashboardStore';
import { invoke } from '@tauri-apps/api/core';
import { useMqttStore } from '../../stores/useMqttStore';
import { usePluginStore } from '../../stores/usePluginStore';
import { extractValue } from '../../utils/jsonExtractor';

const props = defineProps<{
  open: boolean;
  widgetId: string | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const dashboardStore = useDashboardStore();
const mqttStore = useMqttStore();
const pluginStore = usePluginStore();
const activeTab = ref('general');
const currentPlugin = ref<any>(null);

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

const availableTopics = ref<string[]>([]);
const previewValue = ref<any>(null);

const windowHours = ref(0);
const windowMinutes = ref(0);
const windowSeconds = ref(0);

const updateWindowSize = () => {
  if (!localConfig.value) return;
  const h = windowHours.value || 0;
  const m = windowMinutes.value || 0;
  const s = windowSeconds.value || 0;
  localConfig.value.settings.timeWindow = Math.max(0, h * 3600 + m * 60 + s);
};

const fetchTopics = async () => {
  try {
    const topics = await invoke<string[]>('get_distinct_topics');
    availableTopics.value = topics;
  } catch (error) {
    console.error('Failed to fetch topics:', error);
  }
};

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

      // Initialize Window Size breakdown
      const totalSeconds = localConfig.value.settings.timeWindow || 60;
      windowHours.value = Math.floor(totalSeconds / 3600);
      windowMinutes.value = Math.floor((totalSeconds % 3600) / 60);
      windowSeconds.value = totalSeconds % 60;

      // Check if it's a plugin
      const standardTypes = ['value-display', 'switch', 'chart', 'gauge', 'slider', 'gantt', 'scatter', 'plotter'];
      if (!standardTypes.includes(localConfig.value.type)) {
        currentPlugin.value = pluginStore.getPluginByTagName(localConfig.value.type);

        // Initialize default values for plugin settings if missing
        if (currentPlugin.value && currentPlugin.value.configSchema) {
          currentPlugin.value.configSchema.forEach((field: any) => {
            if (localConfig.value && localConfig.value.settings[field.name] === undefined && field.defaultValue !== undefined) {
              localConfig.value.settings[field.name] = field.defaultValue;
            }
          });
        }
      } else {
        currentPlugin.value = null;
      }

      localLayout.value = {
        x: item.x,
        y: item.y,
        w: item.w,
        h: item.h
      };

      // Fetch topics when modal opens
      fetchTopics();
      // Update preview initially
      updatePreview();
    }
  }
});

// Update preview when topic or key changes
watch(
  () => [localConfig.value?.topic, localConfig.value?.settings.valueKey],
  () => {
    updatePreview();
  },
  { deep: true }
);

const updatePreview = () => {
  if (!localConfig.value?.topic) {
    previewValue.value = null;
    return;
  }
  const lastMessage = mqttStore.lastMessages[localConfig.value.topic];
  if (lastMessage) {
    try {
      const payloadJson = JSON.parse(lastMessage.payload);
      const valueKey = localConfig.value.settings.valueKey || '';
      previewValue.value = {
        raw: payloadJson,
        extracted: extractValue(payloadJson, valueKey),
      };
    } catch (e) {
      // Not a JSON payload, show raw
      previewValue.value = {
        raw: lastMessage.payload,
        extracted: lastMessage.payload, // Treat raw value as extracted
      };
    }
  } else {
    previewValue.value = null;
  }
};


const handleSave = () => {
  if (localConfig.value && props.widgetId) {
    dashboardStore.updateWidget(props.widgetId, localConfig.value);
    if (localLayout.value) {
      dashboardStore.updateLayoutItem(props.widgetId, localLayout.value);
    }
    emit('close');
  }
};

const addSeries = () => {
  if (!localConfig.value) return;
  if (!localConfig.value.settings.series) {
    localConfig.value.settings.series = [];
  }
  const series = localConfig.value.settings.series as any[];
  const nextIndex = Number(series.length) + 1;
  series.push({
    topic: '',
    key: '',
    name: `Series ${nextIndex}`,
    color: '#3b82f6'
  });
};

const removeSeries = (index: number) => {
  if (!localConfig.value?.settings.series) return;
  localConfig.value.settings.series.splice(index, 1);
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
            <label class="label"><span class="label-text">Update Frequency (ms)</span></label>
            <input v-model.number="localConfig.updateInterval" type="number" class="input input-bordered"
              placeholder="0 (Realtime)" min="0" step="100" />
            <label class="label">
              <span class="label-text-alt">0 = Realtime. Higher values reduce rendering load.</span>
            </label>
          </div>

          <div class="form-control w-full" v-if="!currentPlugin || currentPlugin.capabilities?.requiresTopic">
            <label class="label"><span class="label-text">MQTT Topic</span></label>
            <input v-model="localConfig.topic" type="text" class="input input-bordered font-mono" list="topic-list" />
            <datalist id="topic-list">
              <option v-for="topic in availableTopics" :key="topic" :value="topic" />
            </datalist>
          </div>

          <div class="form-control w-full">
            <label class="label"><span class="label-text">Widget Type</span></label>
            <select v-model="localConfig.type" class="select select-bordered" disabled>
              <option value="value-display">Value Display</option>
              <option value="switch">Switch</option>
              <option value="chart">Chart</option>
              <option value="gauge">Gauge</option>
              <option value="slider">Slider</option>
            </select>
            <label class="label">
              <span class="label-text-alt">Type cannot be changed after creation</span>
            </label>
          </div>
        </div>

        <!-- Style Tab -->
        <div v-show="activeTab === 'style'" class="flex flex-col gap-4">

          <template v-if="currentPlugin">
            <template v-if="currentPlugin.configSchema">
              <div class="divider">{{ currentPlugin.name }} Settings</div>
              <div v-for="field in currentPlugin.configSchema" :key="field.name" class="form-control w-full">
                <label class="label">
                  <span class="label-text">{{ field.label }}</span>
                </label>

                <!-- Text Input -->
                <input v-if="field.type === 'text'" v-model="localConfig.settings[field.name]" type="text"
                  class="input input-bordered" />

                <!-- Number Input -->
                <input v-else-if="field.type === 'number'" v-model.number="localConfig.settings[field.name]"
                  type="number" class="input input-bordered" :min="field.min" :max="field.max" />

                <!-- Checkbox -->
                <label v-else-if="field.type === 'checkbox'" class="label cursor-pointer justify-start gap-4">
                  <input v-model="localConfig.settings[field.name]" type="checkbox" class="checkbox" />
                  <span class="label-text">{{ field.description || '' }}</span>
                </label>

                <!-- Color Input -->
                <div v-else-if="field.type === 'color'" class="flex items-center gap-4">
                  <input v-model="localConfig.settings[field.name]" type="color"
                    class="input input-bordered w-24 p-1 h-10" />
                  <span class="text-sm opacity-70 font-mono">{{ localConfig.settings[field.name] }}</span>
                </div>

                <!-- Select -->
                <select v-else-if="field.type === 'select'" v-model="localConfig.settings[field.name]"
                  class="select select-bordered">
                  <option v-for="opt in field.options" :key="opt.value" :value="opt.value">
                    {{ opt.label }}
                  </option>
                </select>
              </div>
            </template>
            <div v-else class="text-center opacity-60">No settings available for this plugin.</div>
          </template>

          <template v-else>
            <!-- Common Settings -->
            <div class="divider">Common</div>
            <div class="form-control w-full">
              <label class="label"><span class="label-text">Unit</span></label>
              <input v-model="localConfig.settings.unit" type="text" class="input input-bordered"
                placeholder="e.g. °C, %" />
            </div>

            <div class="form-control w-full" v-if="localConfig.type === 'switch' || localConfig.type === 'slider'">
              <div class="divider">MQTT Publish Settings</div>
              <label class="label"><span class="label-text">QoS Level</span></label>
              <select v-model.number="localConfig.settings.qos" class="select select-bordered">
                <option :value="0">0 (At Most Once)</option>
                <option :value="1">1 (At Least Once)</option>
                <option :value="2">2 (Exactly Once)</option>
              </select>
            </div>

            <div class="form-control w-full" v-if="localConfig.type === 'switch' || localConfig.type === 'slider'">
              <label class="label cursor-pointer justify-start gap-4">
                <input v-model="localConfig.settings.retain" type="checkbox" class="checkbox" />
                <span class="label-text">Retain Message</span>
              </label>
            </div>

            <template v-if="['chart', 'plotter', 'gantt'].includes(localConfig.type)">
              <div class="divider">Time Axis Settings</div>

              <div class="form-control w-full">
                <label class="label"><span class="label-text">Time Mode</span></label>
                <select v-model="localConfig.settings.timeMode" class="select select-bordered">
                  <option value="relative">Relative (Latest X seconds)</option>
                  <option value="absolute">Absolute Range</option>
                </select>
              </div>

              <div v-if="localConfig.settings.timeMode === 'absolute'" class="flex gap-4">
                <div class="form-control w-full">
                  <label class="label"><span class="label-text">Start Time</span></label>
                  <input v-model="localConfig.settings.startTime" type="time" step="1" class="input input-bordered" />
                </div>
                <div class="form-control w-full">
                  <label class="label"><span class="label-text">End Time</span></label>
                  <input v-model="localConfig.settings.endTime" type="time" step="1" class="input input-bordered" />
                </div>
              </div>

              <div v-else class="form-control w-full">
                <label class="label"><span class="label-text">Window Size</span></label>
                <div class="flex gap-2">
                  <div class="flex-1">
                    <input v-model.number="windowHours" @input="updateWindowSize" type="number"
                      class="input input-bordered w-full" min="0" placeholder="0" />
                    <label class="label"><span class="label-text-alt">Hours</span></label>
                  </div>
                  <div class="flex-1">
                    <input v-model.number="windowMinutes" @input="updateWindowSize" type="number"
                      class="input input-bordered w-full" min="0" max="59" placeholder="0" />
                    <label class="label"><span class="label-text-alt">Minutes</span></label>
                  </div>
                  <div class="flex-1">
                    <input v-model.number="windowSeconds" @input="updateWindowSize" type="number"
                      class="input input-bordered w-full" min="0" max="59" placeholder="0" />
                    <label class="label"><span class="label-text-alt">Seconds</span></label>
                  </div>
                </div>
              </div>

              <div class="form-control w-full">
                <label class="label"><span class="label-text">Tick Interval (Seconds)</span></label>
                <input v-model.number="localConfig.settings.tickInterval" type="number" class="input input-bordered"
                  placeholder="Auto" />
                <label class="label"><span class="label-text-alt">Leave empty for auto scaling</span></label>
              </div>
            </template>

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

            <template v-if="localConfig.type === 'slider'">
              <div class="divider">Slider Settings</div>
              <div class="grid grid-cols-3 gap-4">
                <div class="form-control w-full">
                  <label class="label"><span class="label-text">Min Value</span></label>
                  <input v-model.number="localConfig.settings.min" type="number" class="input input-bordered" />
                </div>
                <div class="form-control w-full">
                  <label class="label"><span class="label-text">Max Value</span></label>
                  <input v-model.number="localConfig.settings.max" type="number" class="input input-bordered" />
                </div>
                <div class="form-control w-full">
                  <label class="label"><span class="label-text">Step</span></label>
                  <input v-model.number="localConfig.settings.step" type="number" class="input input-bordered" />
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

            <template v-if="localConfig.type === 'chart'">
              <div class="divider">Chart Settings</div>
              <div class="form-control w-full">
                <label class="label"><span class="label-text">Chart Type</span></label>
                <select v-model="localConfig.settings.chartType" class="select select-bordered">
                  <option value="line">Line Chart</option>
                  <option value="bar">Bar Chart</option>
                </select>
              </div>

              <div class="divider">Chart Axes</div>
              <div class="form-control w-full">
                <label class="label"><span class="label-text">Y-Axis Key (Value)</span></label>
                <input v-model="localConfig.settings.yKey" type="text" class="input input-bordered font-mono"
                  placeholder="e.g. value" />
              </div>
            </template>
          </template>
        </div>

        <!-- Data Mapping Tab -->
        <div v-show="activeTab === 'data'" class="flex flex-col gap-4">
          <div class="alert alert-info text-sm">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"
              class="stroke-current shrink-0 w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <span>Extract values from JSON payload using object keys (e.g. "sensor.temp"). Leave empty for raw
              value.</span>
          </div>

          <template v-if="['chart', 'plotter', 'gantt'].includes(localConfig.type)">
            <div class="flex justify-between items-center">
              <span class="label-text font-bold">Series Configuration</span>
              <button class="btn btn-xs btn-primary" @click="addSeries">+ Add Series</button>
            </div>

            <div v-if="localConfig.settings.series && localConfig.settings.series.length > 0"
              class="flex flex-col gap-2">
              <div v-for="(series, idx) in localConfig.settings.series" :key="idx"
                class="collapse collapse-arrow bg-base-200 border border-base-300">
                <input type="checkbox" />
                <div class="collapse-title font-medium flex items-center gap-2 p-2 min-h-0">
                  <div class="w-3 h-3 rounded-full shrink-0" :style="{ background: series.color || '#3b82f6' }"></div>
                  <span class="text-sm truncate flex-1">{{ series.name || 'Series ' + (Number(idx) + 1) }}</span>
                </div>
                <div class="collapse-content">
                  <div class="form-control w-full">
                    <label class="label py-1"><span class="label-text-alt">Name</span></label>
                    <input v-model="series.name" type="text" class="input input-bordered input-sm" />
                  </div>
                  <div class="form-control w-full">
                    <label class="label py-1"><span class="label-text-alt">Topic (Optional)</span></label>
                    <input v-model="series.topic" type="text" class="input input-bordered input-sm font-mono"
                      list="topic-list" placeholder="Inherit Global Topic" />
                  </div>
                  <div class="form-control w-full">
                    <label class="label py-1"><span class="label-text-alt">Value Key</span></label>
                    <input v-model="series.key" type="text" class="input input-bordered input-sm font-mono" />
                  </div>
                  <div class="form-control w-full">
                    <label class="label py-1"><span class="label-text-alt">Color</span></label>
                    <div class="flex gap-2">
                      <input v-model="series.color" type="color" class="input input-bordered input-sm w-16 p-0" />
                      <input v-model="series.color" type="text"
                        class="input input-bordered input-sm flex-1 font-mono" />
                    </div>
                  </div>
                  <div class="mt-2 text-right">
                    <button class="btn btn-xs btn-error btn-outline"
                      @click="removeSeries(idx as number)">Remove</button>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="text-center text-sm opacity-60 p-4 border border-dashed rounded-md">
              No series defined. <br />Falls back to legacy "Value Key" below.
            </div>

            <div class="divider">Legacy / Single Value</div>
          </template>

          <div class="form-control w-full">
            <label class="label"><span class="label-text">Value Key</span></label>
            <input v-model="localConfig.settings.valueKey" type="text" class="input input-bordered font-mono"
              placeholder="e.g. temperature" />
            <label class="label" v-if="localConfig.type === 'chart'">
              <span class="label-text-alt text-warning">Used if no series are defined above.</span>
            </label>
          </div>

          <template v-if="localConfig.type === 'scatter'">
            <div class="divider">Scatter Configuration</div>
            <div class="grid grid-cols-2 gap-4">
              <div class="form-control w-full">
                <label class="label"><span class="label-text">X Key</span></label>
                <input v-model="localConfig.settings.xKey" type="text" class="input input-bordered font-mono"
                  placeholder="value.x" />
              </div>
              <div class="form-control w-full">
                <label class="label"><span class="label-text">Y Key</span></label>
                <input v-model="localConfig.settings.yKey" type="text" class="input input-bordered font-mono"
                  placeholder="value.y" />
              </div>
              <div class="form-control w-full">
                <label class="label"><span class="label-text">Label/Z Key</span></label>
                <input v-model="localConfig.settings.zKey" type="text" class="input input-bordered font-mono"
                  placeholder="value.z" />
              </div>
            </div>
          </template>

          <div class="divider">Preview</div>

          <div v-if="previewValue" class="space-y-2">
            <div>
              <label class="label-text font-bold">Last Raw Payload:</label>
              <pre class="bg-base-200 p-2 rounded-md text-xs whitespace-pre-wrap"><code>{{ JSON.stringify(previewValue.raw,
                null, 2) }}</code></pre>
            </div>
            <div>
              <label class="label-text font-bold">Extracted Value:</label>
              <pre class="bg-base-200 p-2 rounded-md text-xs"><code>{{ previewValue.extracted }}</code></pre>
            </div>
          </div>
          <div v-else class="text-center text-sm opacity-60">
            No message received yet on the selected topic.
          </div>

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
