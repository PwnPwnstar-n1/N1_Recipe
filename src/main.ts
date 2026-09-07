import { createApp } from "vue";
import App from "./App.vue";
import { i18n } from "./i18n";
import "./style.css";
import {applyAppearance} from "./appearance";
applyAppearance();

createApp(App).use(i18n).mount("#app");
