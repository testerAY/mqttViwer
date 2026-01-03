import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./style.css";
import { initPluginSdk } from "./utils/pluginSdk";
import { usePluginStore } from "./stores/usePluginStore";
import { registerCoreWidgets } from './registries/widgetRegistry';

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);

// Initialize Plugin SDK
initPluginSdk();
// default widgets registration
registerCoreWidgets();

// Fetch plugins on startup
const pluginStore = usePluginStore();
pluginStore.fetchPlugins();

app.mount("#app");
