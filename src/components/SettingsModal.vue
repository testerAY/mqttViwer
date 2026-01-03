<script setup lang="ts">
import { ref, watch } from 'vue';
import { useAppStore, type AppConfig } from '../stores/useAppStore';

const props = defineProps<{
  open: boolean
}>();

const emit = defineEmits<{
  (e: 'close'): void
}>();

const appStore = useAppStore();
const localConfig = ref<AppConfig | null>(null);

// Initialize form when modal opens
watch(() => props.open, (isOpen) => {
  if (isOpen) {
    // Clone settings to local state to avoid direct mutation
    localConfig.value = JSON.parse(JSON.stringify(appStore.settings));
  }
});

const handleSave = async () => {
  if (localConfig.value) {
    await appStore.saveSettings(localConfig.value);
    emit('close');
  }
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-2xl">
      <h3 class="font-bold text-lg">Application Settings</h3>

      <div v-if="localConfig" class="py-4 flex flex-col gap-4">

        <!-- Theme Settings -->
        <div class="form-control w-full">
          <label class="label"><span class="label-text">Theme</span></label>
          <select v-model="localConfig.theme" class="select select-bordered">
            <option value="light">Light</option>
            <option value="dark">Dark</option>
            <option value="system">System</option>
          </select>
        </div>

        <div class="divider">Broker Connection</div>

        <!-- Broker Mode -->
        <div class="form-control w-full">
          <label class="label"><span class="label-text">Connection Mode</span></label>
          <select v-model="localConfig.broker.mode" class="select select-bordered">
            <option value="internal">Internal (Built-in Broker)</option>
            <option value="external">External Broker</option>
          </select>
          <label class="label">
            <span class="label-text-alt text-warning">
              Restart required to apply changes to connection settings.
            </span>
          </label>
        </div>

        <div class="flex gap-4">
          <!-- Host -->
          <div class="form-control w-full">
            <label class="label"><span class="label-text">Host</span></label>
            <input v-model="localConfig.broker.host" type="text" class="input input-bordered"
              :disabled="localConfig.broker.mode === 'internal'" />
            <label class="label" v-if="localConfig.broker.mode === 'internal'">
              <span class="label-text-alt">Internal broker always listens on 127.0.0.1</span>
            </label>
          </div>

          <!-- Port -->
          <div class="form-control w-full">
            <label class="label"><span class="label-text">Port</span></label>
            <input v-model.number="localConfig.broker.port" type="number" class="input input-bordered" />
          </div>
        </div>

        <!-- Credentials -->
        <div class="flex gap-4" v-if="localConfig.broker.mode === 'external'">
          <!-- Username -->
          <div class="form-control w-full">
            <label class="label"><span class="label-text">Username</span></label>
            <input v-model="localConfig.broker.username" type="text" class="input input-bordered"
              placeholder="Optional" />
          </div>

          <!-- Password -->
          <div class="form-control w-full">
            <label class="label"><span class="label-text">Password</span></label>
            <input v-model="localConfig.broker.password" type="password" class="input input-bordered"
              placeholder="Optional" />
          </div>
        </div>

        <!-- Subscription Topic -->
        <div class="form-control w-full">
          <label class="label"><span class="label-text">Subscription Topic</span></label>
          <input v-model="localConfig.broker.subscription_topic" type="text" class="input input-bordered" />
          <label class="label">
            <span class="label-text-alt">Use MQTT topic syntax (e.g., `sensors/#`). Restart required.</span>
          </label>
        </div>

        <div class="divider">Data Retention</div>

        <!-- Retention Policy -->
        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-4">
            <span class="label-text">Enable Auto-deletion</span>
            <input type="checkbox" v-model="localConfig.retention.enabled" class="checkbox" />
          </label>
        </div>

        <div class="form-control w-full" v-if="localConfig.retention.enabled">
          <label class="label"><span class="label-text">Keep Data For (Days)</span></label>
          <input v-model.number="localConfig.retention.days" type="number" class="input input-bordered" min="1" />
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
