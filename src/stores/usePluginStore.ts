import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  main: string;
  tagName: string;
  description?: string;
}

export const usePluginStore = defineStore('plugins', () => {
  const plugins = ref<PluginManifest[]>([]);
  const loadedPlugins = ref<Set<string>>(new Set());

  const fetchPlugins = async () => {
    try {
      plugins.value = await invoke<PluginManifest[]>('get_plugin_list');
    } catch (e) {
      console.error('Failed to fetch plugin list:', e);
    }
  };

  const loadPlugin = async (pluginId: string): Promise<boolean> => {
    if (loadedPlugins.value.has(pluginId)) {
      return true;
    }

    let plugin = plugins.value.find(p => p.id === pluginId);
    if (!plugin) {
      // Try fetching again, maybe initial fetch hasn't completed or new plugin added
      await fetchPlugins();
      plugin = plugins.value.find(p => p.id === pluginId);
    }

    if (!plugin) {
      console.error(`Plugin ${pluginId} not found in manifest list. Available: ${plugins.value.map(p => p.id).join(', ')}`);
      return false;
    }

    return new Promise((resolve) => {
      // Fetch script content via IPC to avoid CORS issues with custom protocol
      invoke<string>('load_plugin_file', { 
        pluginId: plugin.id, 
        fileName: plugin.main 
      }).then(base64Content => {
        // Decode Base64 to binary
        const binaryString = atob(base64Content);
        const len = binaryString.length;
        const bytes = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }

        const blob = new Blob([bytes], { type: 'application/javascript' });
        const url = URL.createObjectURL(blob);
        
        const script = document.createElement('script');
        script.src = url;
        script.type = 'module';
        
        script.onload = () => {
          console.log(`Plugin loaded: ${plugin.id}`);
          loadedPlugins.value.add(plugin.id);
          URL.revokeObjectURL(url); // Clean up
          
          if (plugin.tagName) {
            customElements.whenDefined(plugin.tagName).then(() => {
              resolve(true);
            });
          } else {
            resolve(true);
          }
        };

        script.onerror = (e) => {
          console.error(`Failed to load plugin script: ${plugin.id}`, e);
          URL.revokeObjectURL(url);
          resolve(false);
        };

        document.head.appendChild(script);
      }).catch(err => {
        console.error(`Failed to fetch plugin file for ${plugin.id}:`, err);
        resolve(false);
      });
    });
  };

  const getPluginByTagName = (tagName: string) => {
    return plugins.value.find(p => p.tagName === tagName);
  };

  return {
    plugins,
    loadedPlugins,
    fetchPlugins,
    loadPlugin,
    getPluginByTagName
  };
});
