<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';

const props = defineProps<{
  tagName: string;
  config: WidgetConfig;
  message?: any;
}>();

const hostRef = ref<HTMLElement | null>(null);

function updateElementProperties() {
  if (hostRef.value) {
    // Pass properties to the Web Component
    (hostRef.value as any).config = props.config;
    (hostRef.value as any).message = props.message;
  }
}

onMounted(() => {
  updateElementProperties();
});

watch(() => props.config, () => {
  updateElementProperties();
}, { deep: true });

watch(() => props.message, () => {
  updateElementProperties();
}, { deep: true });
</script>

<template>
  <component 
    :is="tagName" 
    ref="hostRef"
    class="w-full h-full block"
  />
</template>
