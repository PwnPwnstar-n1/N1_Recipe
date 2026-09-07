<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { localizeError } from "../i18n";
import {
  changeLanguage,
  store,
  updateDockSettings,
  updateLauncherSettings,
  updateStartupSettings,
} from "../store";
import type { AppLocale } from "../i18n";
import RecipePackPanel from"./RecipePackPanel.vue";
import {applyAppearance,type FontSize,type Theme} from "../appearance";

const { t } = useI18n();
const theme=ref<Theme>((localStorage.getItem("snipdeck.theme") as Theme)||"system");
const fontSize=ref<FontSize>((localStorage.getItem("snipdeck.font") as FontSize)||"standard");
function changeAppearance(){applyAppearance(theme.value,fontSize.value)}

const options: { value: AppLocale; label: string }[] = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en-US", label: "English" },
];

function onChangeLanguage(e: Event): void {
  const target = e.target as HTMLSelectElement;
  void changeLanguage(target.value as AppLocale);
}

// Shortcut draft: local copy so typing is not clobbered by store updates;
// committed on Enter/blur. On failure the draft reverts to the stored value.
const shortcutDraft = ref("Ctrl+Alt+Space");
const shortcutFeedback = ref("");
const shortcutIsError = ref(false);

watch(
  () => store.settings?.launcher.shortcut,
  (value) => {
    if (value !== undefined) shortcutDraft.value = value;
  },
  { immediate: true },
);

async function commitShortcut(): Promise<void> {
  const current = store.settings?.launcher.shortcut ?? "Ctrl+Alt+Space";
  const enabled = store.settings?.launcher.enabled ?? true;
  if (shortcutDraft.value === current) {
    shortcutFeedback.value = "";
    shortcutIsError.value = false;
    return;
  }
  const error = await updateLauncherSettings({
    enabled,
    shortcut: shortcutDraft.value,
  });
  if (error !== null) {
    shortcutFeedback.value = error;
    shortcutIsError.value = true;
    shortcutDraft.value = current; // revert — the old shortcut is still active
  } else {
    shortcutFeedback.value = t("settings.shortcutSaved");
    shortcutIsError.value = false;
  }
}

async function onToggleEnabled(e: Event): Promise<void> {
  const target = e.target as HTMLInputElement;
  const enabled = target.checked;
  const shortcut = store.settings?.launcher.shortcut ?? "Ctrl+Alt+Space";
  const error = await updateLauncherSettings({ enabled, shortcut });
  if (error !== null) {
    shortcutFeedback.value = error;
    shortcutIsError.value = true;
    target.checked = !enabled; // revert UI — backend kept the old state
  } else {
    shortcutFeedback.value = "";
    shortcutIsError.value = false;
  }
}

async function openLauncher(): Promise<void> {
  try {
    await api.showLauncher();
  } catch (err) {
    shortcutFeedback.value = err instanceof Error ? err.message : String(err);
    shortcutIsError.value = true;
  }
}

// Desktop dock: each control commits the full dock settings (backend
// validates + applies geometry/visibility immediately).
const dockFeedback = ref("");
const dockIsError = ref(false);

function currentDock(): api.Settings["dock"] {
  return (
    store.settings?.dock ?? {
      enabled: false,
      position: "right",
      autoHide: true,
      alwaysOnTop: true,
      collapsedWidth: 8,
      expandedWidth: 360,
      hideDelay: 400,
    }
  );
}

async function commitDock(patch: Partial<api.Settings["dock"]>): Promise<void> {
  const next = { ...currentDock(), ...patch };
  const error = await updateDockSettings(next);
  if (error !== null) {
    dockFeedback.value = error;
    dockIsError.value = true;
  } else {
    dockFeedback.value = "";
    dockIsError.value = false;
  }
}

function onToggleDockEnabled(e: Event): void {
  const target = e.target as HTMLInputElement;
  void commitDock({ enabled: target.checked });
}

function onChangeDockPosition(e: Event): void {
  const target = e.target as HTMLSelectElement;
  void commitDock({ position: target.value as "left" | "right" });
}

function onToggleDockAlwaysOnTop(e: Event): void {
  const target = e.target as HTMLInputElement;
  void commitDock({ alwaysOnTop: target.checked });
}

function onToggleDockAutoHide(e: Event): void {
  const target = e.target as HTMLInputElement;
  void commitDock({ autoHide: target.checked });
}

// Startup behavior (autostart + start-hidden)
const startupFeedback = ref("");
const startupIsError = ref(false);

function currentStartup(): api.Settings["startup"] {
  return store.settings?.startup ?? { launchOnBoot: false, startHidden: true };
}

async function commitStartup(
  patch: Partial<api.Settings["startup"]>,
): Promise<void> {
  const next = { ...currentStartup(), ...patch };
  const error = await updateStartupSettings(next);
  if (error !== null) {
    startupFeedback.value = error;
    startupIsError.value = true;
  } else {
    startupFeedback.value = "";
    startupIsError.value = false;
  }
}

function onToggleLaunchOnBoot(e: Event): void {
  const target = e.target as HTMLInputElement;
  void commitStartup({ launchOnBoot: target.checked });
}

function onToggleStartHidden(e: Event): void {
  const target = e.target as HTMLInputElement;
  void commitStartup({ startHidden: target.checked });
}

// Backup & restore
const backupFeedback = ref("");
const backupIsError = ref(false);

async function onExport(): Promise<void> {
  const path = await save({
    filters: [{ name: "N1_Recipe Backup", extensions: ["json"] }],
    defaultPath: "n1-recipe-backup.json",
  });
  if (!path) return;
  try {
    const count = await api.exportSnippets(path);
    backupFeedback.value = t("toast.exportCompleted", { count });
    backupIsError.value = false;
  } catch (err) {
    backupFeedback.value = localizeError(err);
    backupIsError.value = true;
  }
}

async function onImport(): Promise<void> {
  const selected = await open({
    filters: [{ name: "N1_Recipe Backup", extensions: ["json"] }],
    multiple: false,
  });
  if (!selected) return;
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return;
  try {
    const summary = await api.importSnippets(path);
    backupFeedback.value = t("toast.importCompleted", {
      added: summary.added,
      skipped: summary.skippedDuplicates,
    });
    backupIsError.value = false;
  } catch (err) {
    backupFeedback.value = localizeError(err);
    backupIsError.value = true;
  }
}
</script>

<template>
  <div class="settings">
    <h2 class="title">{{ t("settings.title") }}</h2>
    <div class="group"><label class="label">外观</label><div class="appearance-row"><select v-model="theme" class="select" @change="changeAppearance"><option value="system">跟随系统</option><option value="light">亮色</option><option value="dark">暗色</option></select><select v-model="fontSize" class="select" @change="changeAppearance"><option value="small">小</option><option value="standard">标准</option><option value="large">大</option><option value="xlarge">特大</option></select></div><p class="hint">主题和字号会应用到主窗口、编辑器与快捷窗口。</p></div>

    <div class="group">
      <label class="label">{{ t("settings.language") }}</label>
      <select
        class="select"
        :value="store.settings?.language ?? 'en-US'"
        @change="onChangeLanguage"
      >
        <option v-for="opt in options" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
      <p class="hint">{{ t("settings.languageHint") }}</p>
    </div>

    <div class="group">
      <label class="label">N1 配方包</label><RecipePackPanel/>
    </div>
    <div class="group">
      <label class="label">{{ t("settings.quickLauncher") }}</label>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="store.settings?.launcher.enabled ?? true"
          @change="onToggleEnabled"
        />
        <span>{{ t("settings.enableLauncher") }}</span>
      </label>
      <div class="shortcut-row">
        <span class="shortcut-label">{{ t("settings.shortcut") }}</span>
        <input
          v-model="shortcutDraft"
          type="text"
          spellcheck="false"
          :placeholder="t('settings.shortcutPlaceholder')"
          @keydown.enter="commitShortcut"
          @blur="commitShortcut"
        />
      </div>
      <button class="btn" @click="openLauncher">
        {{ t("settings.openLauncher") }}
      </button>
      <p v-if="shortcutFeedback" class="feedback" :class="{ ok: !shortcutIsError }">
        {{ shortcutFeedback }}
      </p>
    </div>

    <div class="group">
      <label class="label">{{ t("settings.dock") }}</label>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="store.settings?.dock.enabled ?? false"
          @change="onToggleDockEnabled"
        />
        <span>{{ t("settings.enableDock") }}</span>
      </label>
      <div class="dock-row">
        <span class="dock-label">{{ t("settings.dockPosition") }}</span>
        <select
          class="select"
          :value="store.settings?.dock.position ?? 'right'"
          @change="onChangeDockPosition"
        >
          <option value="left">{{ t("settings.left") }}</option>
          <option value="right">{{ t("settings.right") }}</option>
        </select>
      </div>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="store.settings?.dock.alwaysOnTop ?? true"
          @change="onToggleDockAlwaysOnTop"
        />
        <span>{{ t("settings.alwaysOnTop") }}</span>
      </label>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="store.settings?.dock.autoHide ?? true"
          @change="onToggleDockAutoHide"
        />
        <span>{{ t("settings.autoHide") }}</span>
      </label>
      <p v-if="dockFeedback" class="feedback" :class="{ ok: !dockIsError }">
        {{ dockFeedback }}
      </p>
    </div>

    <div class="group">
      <label class="label">{{ t("settings.startup") }}</label>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="store.settings?.startup.launchOnBoot ?? false"
          @change="onToggleLaunchOnBoot"
        />
        <span>{{ t("settings.launchOnBoot") }}</span>
      </label>
      <label class="checkbox">
        <input
          type="checkbox"
          :checked="store.settings?.startup.startHidden ?? true"
          @change="onToggleStartHidden"
        />
        <span>{{ t("settings.startHidden") }}</span>
      </label>
      <p v-if="startupFeedback" class="feedback" :class="{ ok: !startupIsError }">
        {{ startupFeedback }}
      </p>
    </div>

    <div class="group">
      <label class="label">{{ t("settings.backup") }}</label>
      <div class="backup-row">
        <button class="btn" @click="onExport">
          {{ t("settings.exportData") }}
        </button>
        <button class="btn" @click="onImport">
          {{ t("settings.importData") }}
        </button>
      </div>
      <p v-if="backupFeedback" class="feedback" :class="{ ok: !backupIsError }">
        {{ backupFeedback }}
      </p>
    </div>
  </div>
</template>

<style scoped>
.settings {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 16px 16px 40px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.title {
  margin: 0;
  font-size: 16px;
  color: var(--text-bright);
}

.group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-panel);
  border-radius: 6px;
}

.label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--text-faint);
}

.select {
  padding: 6px 9px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--text-bright);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  align-self: flex-start;
  min-width: 180px;
}

.select:focus,
input:focus {
  border-color: var(--accent);
}

.checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
}

.shortcut-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.dock-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.backup-row {
  display: flex;
  gap: 8px;
}

.dock-label {
  width: 120px;
  color: var(--text-dim);
  font-size: 12px;
}

.shortcut-label {
  width: 120px;
  color: var(--text-dim);
  font-size: 12px;
}

input {
  flex: 1;
  padding: 6px 9px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--text-bright);
  font-size: 13px;
  font-family: var(--mono);
  outline: none;
  min-width: 0;
}

.btn {
  align-self: flex-start;
  padding: 6px 14px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  font-size: 12.5px;
  cursor: pointer;
}

.btn:hover {
  background: var(--bg-hover);
}

.feedback {
  margin: 0;
  font-size: 12px;
  color: var(--danger);
}

.feedback.ok {
  color: var(--accent);
}
</style>
