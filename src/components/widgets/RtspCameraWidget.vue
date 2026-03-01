<template>
  <div class="w-full h-full flex flex-col overflow-hidden">
    <!-- Video area -->
    <div class="flex-1 relative bg-black overflow-hidden flex items-center justify-center min-h-0">
      <img
        v-if="streamUrl && isStreaming"
        :src="streamUrl"
        class="max-w-full max-h-full object-contain"
        @error="handleStreamError"
      />
      <div v-else-if="isConnecting" class="flex flex-col items-center gap-2 text-base-content/60">
        <span class="loading loading-spinner loading-md"></span>
        <span class="text-sm">Connecting...</span>
      </div>
      <div v-else class="flex flex-col items-center gap-3">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 text-base-content/30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
        </svg>
        <button @click="startStream" class="btn btn-primary btn-sm" :disabled="!hasRtspUrl">
          Connect
        </button>
        <p v-if="!hasRtspUrl" class="text-warning text-xs">Set RTSP URL in widget settings</p>
        <p v-if="error" class="text-error text-xs max-w-48 text-center">{{ error }}</p>
      </div>

      <!-- Recording indicator -->
      <div v-if="isRecording" class="absolute top-2 right-2">
        <span class="badge badge-error badge-sm gap-1 animate-pulse">
          <svg class="w-2 h-2 fill-current" viewBox="0 0 8 8"><circle cx="4" cy="4" r="4"/></svg>
          REC
        </span>
      </div>
    </div>

    <!-- Controls bar -->
    <div v-if="showControls" class="flex items-center gap-1 px-2 py-1 bg-base-200 border-t border-base-300 shrink-0">
      <button
        @click="toggleStream"
        class="btn btn-xs"
        :class="isStreaming ? 'btn-error btn-outline' : 'btn-primary'"
        :disabled="isConnecting || !hasRtspUrl"
      >
        {{ isStreaming ? 'Stop' : 'Start' }}
      </button>
      <button
        @click="toggleRecord"
        class="btn btn-xs"
        :class="isRecording ? 'btn-error' : 'btn-outline'"
        :disabled="!isStreaming"
      >
        {{ isRecording ? 'Stop Rec' : 'Record' }}
      </button>
      <button
        @click="snapshot"
        class="btn btn-xs btn-outline"
        :disabled="!isStreaming"
      >
        Snapshot
      </button>
      <div class="flex-1"></div>
      <span v-if="isStreaming" class="badge badge-success badge-xs">LIVE</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import type { WidgetConfig, RtspWidgetSettings } from '../../types/dashboard';
import { useRtspStore } from '../../stores/useRtspStore';
import { useToastStore } from '../../stores/useToastStore';

const props = defineProps<{
  config: WidgetConfig;
  message?: any;
}>();

const rtspStore = useRtspStore();
const toastStore = useToastStore();

const reconnectTimer = ref<ReturnType<typeof setTimeout> | null>(null);
const reconnectAttempts = ref(0);
const maxReconnectAttempts = 10;

const settings = computed<RtspWidgetSettings>(() => ({
  rtspUrl: props.config.settings?.rtspUrl ?? '',
  mode: props.config.settings?.mode ?? 'passthrough',
  width: props.config.settings?.width ?? 640,
  height: props.config.settings?.height ?? 480,
  fps: props.config.settings?.fps ?? 15,
  bitrate: props.config.settings?.bitrate ?? '500k',
  quality: props.config.settings?.quality ?? 5,
  rtspTransport: props.config.settings?.rtspTransport ?? 'tcp',
  reconnectDelaySecs: props.config.settings?.reconnectDelaySecs ?? 3,
  showControls: props.config.settings?.showControls ?? true,
  autoStart: props.config.settings?.autoStart ?? false,
}));

const hasRtspUrl = computed(() => !!settings.value.rtspUrl);
const showControls = computed(() => settings.value.showControls);

const streamState = computed(() => rtspStore.getOrCreate(props.config.id));
const streamUrl = computed(() => streamState.value.url);
const isStreaming = computed(() => streamState.value.isStreaming);
const isConnecting = computed(() => streamState.value.isConnecting);
const isRecording = computed(() => streamState.value.isRecording);
const error = computed(() => streamState.value.error);

async function startStream() {
  try {
    reconnectAttempts.value = 0;
    await rtspStore.startStream(props.config.id, settings.value);
  } catch (e: any) {
    toastStore.addToast(`Failed to connect: ${e}`, 'error');
  }
}

async function stopStream() {
  clearReconnectTimer();
  await rtspStore.stopStream(props.config.id);
}

function toggleStream() {
  if (isStreaming.value || isConnecting.value) {
    stopStream();
  } else {
    startStream();
  }
}

async function toggleRecord() {
  if (isRecording.value) {
    await rtspStore.stopRecording(props.config.id);
    toastStore.addToast('Recording stopped', 'success');
  } else {
    try {
      const path = await rtspStore.startRecording(props.config.id);
      toastStore.addToast(`Recording to: ${path}`, 'info');
    } catch (e: any) {
      toastStore.addToast(`Recording failed: ${e}`, 'error');
    }
  }
}

async function snapshot() {
  try {
    const path = await rtspStore.takeSnapshot(
      props.config.id,
      settings.value.rtspUrl,
      settings.value.rtspTransport,
    );
    toastStore.addToast(`Snapshot saved: ${path}`, 'success');
  } catch (e: any) {
    toastStore.addToast(`Snapshot failed: ${e}`, 'error');
  }
}

function handleStreamError() {
  if (!isStreaming.value) return;

  if (reconnectAttempts.value < maxReconnectAttempts) {
    const delay = settings.value.reconnectDelaySecs * 1000 * Math.min(reconnectAttempts.value + 1, 5);
    reconnectAttempts.value++;
    reconnectTimer.value = setTimeout(async () => {
      if (isStreaming.value) {
        try {
          await rtspStore.startStream(props.config.id, settings.value);
          reconnectAttempts.value = 0;
        } catch {
          handleStreamError();
        }
      }
    }, delay);
  } else {
    rtspStore.stopStream(props.config.id);
    toastStore.addToast('Stream disconnected after max retries', 'error');
  }
}

function clearReconnectTimer() {
  if (reconnectTimer.value) {
    clearTimeout(reconnectTimer.value);
    reconnectTimer.value = null;
  }
}

onMounted(() => {
  if (settings.value.autoStart && hasRtspUrl.value) {
    startStream();
  }
});

onUnmounted(() => {
  clearReconnectTimer();
  rtspStore.stopStream(props.config.id);
  rtspStore.cleanup(props.config.id);
});
</script>
