<script setup lang="ts">
import { useToastStore } from '../stores/useToastStore';

const toastStore = useToastStore();

const toastClass = (type: string) => {
  switch (type) {
    case 'success':
      return 'alert-success';
    case 'error':
      return 'alert-error';
    case 'info':
    default:
      return 'alert-info';
  }
};
</script>

<template>
  <div class="toast toast-end toast-bottom z-50">
    <div
      v-for="toast in toastStore.toasts"
      :key="toast.id"
      class="alert"
      :class="toastClass(toast.type)"
    >
      <span>{{ toast.message }}</span>
    </div>
  </div>
</template>
