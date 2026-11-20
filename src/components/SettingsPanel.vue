<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
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
const secondaryDraft = ref("Ctrl+Alt+F12");
const shortcutFeedback = ref("");
const shortcutIsError = ref(false);
const shortcutDiagnostics = ref<api.ShortcutDiagnostics | null>(null);

function currentLauncher(): api.Settings["launcher"] {
  return store.settings?.launcher ?? {
    enabled: true, shortcut: "Ctrl+Alt+Space", secondaryEnabled: false,
    secondaryShortcut: "Ctrl+Alt+F12",
    mouse: { enabled: false, button: "XButton1", modifiers: "", consume: false },
  };
}

watch(
  () => store.settings?.launcher.shortcut,
  (value) => {
    if (value !== undefined) shortcutDraft.value = value;
  },
  { immediate: true },
);
watch(() => store.settings?.launcher.secondaryShortcut, value => {
  if (value !== undefined) secondaryDraft.value = value;
}, { immediate: true });

async function refreshDiagnostics(): Promise<void> {
  try { shortcutDiagnostics.value = await api.getShortcutDiagnostics(); } catch { /* app may be closing */ }
}
let diagnosticTimer: number | undefined;
onMounted(() => { void refreshDiagnostics(); diagnosticTimer = window.setInterval(refreshDiagnostics, 2000); });
onUnmounted(() => { if (diagnosticTimer !== undefined) window.clearInterval(diagnosticTimer); });

async function commitLauncher(patch: Partial<api.Settings["launcher"]>): Promise<boolean> {
  const error = await updateLauncherSettings({ ...currentLauncher(), ...patch });
  if (error !== null) { shortcutFeedback.value = error; shortcutIsError.value = true; return false; }
  shortcutFeedback.value = t("settings.shortcutSaved"); shortcutIsError.value = false;
  await refreshDiagnostics(); return true;
}

async function commitShortcut(): Promise<void> {
  const current = store.settings?.launcher.shortcut ?? "Ctrl+Alt+Space";
  if (shortcutDraft.value === current) {
    shortcutFeedback.value = "";
    shortcutIsError.value = false;
    return;
  }
  if (!(await commitLauncher({ shortcut: shortcutDraft.value }))) {
    shortcutDraft.value = current; // revert — the old shortcut is still active
  }
}

async function commitSecondary(): Promise<void> {
  const current = currentLauncher().secondaryShortcut;
  if (secondaryDraft.value === current) return;
  if (!(await commitLauncher({ secondaryShortcut: secondaryDraft.value }))) secondaryDraft.value = current;
}

async function onToggleEnabled(e: Event): Promise<void> {
  const target = e.target as HTMLInputElement;
  const enabled = target.checked;
  if (!(await commitLauncher({ enabled }))) {
    target.checked = !enabled; // revert UI — backend kept the old state
  }
}


function recordedKey(event: KeyboardEvent): string | null {
  if (["Control", "Alt", "Shift", "Meta"].includes(event.key)) return null;
  const names: Record<string,string> = { " ": "Space", Esc: "Escape", Up: "ArrowUp", Down: "ArrowDown", Left: "ArrowLeft", Right: "ArrowRight" };
  let key = names[event.key] ?? event.key;
  if (/^[a-z]$/i.test(key)) key = key.toUpperCase();
  if (!(/^[A-Z0-9]$/.test(key) || /^F(?:[1-9]|1[0-9]|2[0-4])$/.test(key) || ["Space","Tab","Enter","Escape","Backspace","Delete","Insert","Home","End","PageUp","PageDown","ArrowUp","ArrowDown","ArrowLeft","ArrowRight","Pause","PrintScreen"].includes(key))) return null;
  const modifiers = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Win"].filter(Boolean);
  return modifiers.length ? [...modifiers, key].join("+") : null;
}

function recordShortcut(event: KeyboardEvent, secondary = false): void {
  event.preventDefault(); const value = recordedKey(event); if (!value) return;
  if (secondary) { secondaryDraft.value = value; void commitSecondary(); }
  else { shortcutDraft.value = value; void commitShortcut(); }
}

async function onReregister(): Promise<void> {
  try { shortcutDiagnostics.value = await api.reregisterShortcuts(); shortcutFeedback.value = "重新注册成功"; shortcutIsError.value = false; }
  catch (err) { shortcutFeedback.value = localizeError(err); shortcutIsError.value = true; await refreshDiagnostics(); }
}
async function onTestLauncher(): Promise<void> {
  try { shortcutDiagnostics.value = await api.testLauncher(); } catch (err) { shortcutFeedback.value = localizeError(err); shortcutIsError.value = true; }
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
        <span class="shortcut-label">主快捷键</span>
        <input
          v-model="shortcutDraft"
          type="text"
          readonly
          spellcheck="false"
          :placeholder="t('settings.shortcutPlaceholder')"
          @keydown="recordShortcut($event)"
        />
      </div>
      <label class="checkbox"><input type="checkbox" :checked="currentLauncher().secondaryEnabled" @change="commitLauncher({ secondaryEnabled: ($event.target as HTMLInputElement).checked })"/><span>启用备用快捷键</span></label>
      <div class="shortcut-row"><span class="shortcut-label">备用快捷键</span><input v-model="secondaryDraft" type="text" readonly spellcheck="false" @keydown="recordShortcut($event, true)"/></div>
      <label class="checkbox"><input type="checkbox" :checked="currentLauncher().mouse.enabled" @change="commitLauncher({ mouse: { ...currentLauncher().mouse, enabled: ($event.target as HTMLInputElement).checked } })"/><span>启用鼠标侧键</span></label>
      <div class="appearance-row">
        <select class="select" :value="currentLauncher().mouse.button" @change="commitLauncher({ mouse: { ...currentLauncher().mouse, button: ($event.target as HTMLSelectElement).value as 'XButton1'|'XButton2' } })"><option value="XButton1">Mouse XButton1</option><option value="XButton2">Mouse XButton2</option></select>
        <select class="select" :value="currentLauncher().mouse.modifiers" @change="commitLauncher({ mouse: { ...currentLauncher().mouse, modifiers: ($event.target as HTMLSelectElement).value } })"><option value="">无修饰键</option><option value="Ctrl">Ctrl</option><option value="Alt">Alt</option><option value="Shift">Shift</option><option value="Win">Win</option><option value="Ctrl+Shift">Ctrl + Shift</option><option value="Ctrl+Alt">Ctrl + Alt</option></select>
      </div>
      <label class="checkbox"><input type="checkbox" :checked="currentLauncher().mouse.consume" @change="commitLauncher({ mouse: { ...currentLauncher().mouse, consume: ($event.target as HTMLInputElement).checked } })"/><span>消费匹配的侧键事件（会阻止浏览器前进/后退）</span></label>
      <div class="backup-row"><button class="btn" @click="openLauncher">{{ t("settings.openLauncher") }}</button><button class="btn" @click="onTestLauncher">测试 Launcher</button><button class="btn" @click="onReregister">重新注册</button></div>
      <p v-if="shortcutFeedback" class="feedback" :class="{ ok: !shortcutIsError }">
        {{ shortcutFeedback }}
      </p>
      <div v-if="shortcutDiagnostics" class="diagnostics">
        <div>主快捷键：{{ shortcutDiagnostics.primary.binding ?? "关闭" }} · {{ shortcutDiagnostics.primary.state }}</div>
        <div>备用快捷键：{{ shortcutDiagnostics.secondary.binding ?? "关闭" }} · {{ shortcutDiagnostics.secondary.state }}</div>
        <div>鼠标：{{ shortcutDiagnostics.mouse.binding ?? "关闭" }} · {{ shortcutDiagnostics.mouse.state }}</div>
        <div>最近输入：{{ shortcutDiagnostics.lastInputAt ?? "尚未收到" }} <span v-if="shortcutDiagnostics.lastInputSource">({{ shortcutDiagnostics.lastInputSource }})</span></div>
        <div>最近显示：{{ shortcutDiagnostics.lastLauncherResult ?? "尚未测试" }}<span v-if="shortcutDiagnostics.lastLauncherDurationMs !== null"> · {{ shortcutDiagnostics.lastLauncherDurationMs }} ms</span></div>
        <div v-if="shortcutDiagnostics.primary.lastError || shortcutDiagnostics.secondary.lastError || shortcutDiagnostics.mouse.lastError" class="feedback">最近错误：{{ shortcutDiagnostics.primary.lastError || shortcutDiagnostics.secondary.lastError || shortcutDiagnostics.mouse.lastError }}</div>
        <p class="hint">{{ shortcutDiagnostics.registrationEvidenceNote }}</p>
      </div>
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

.diagnostics { padding: 9px; border: 1px solid var(--border); border-radius: 5px; color: var(--text-dim); font-size: 11px; line-height: 1.65; }
.hint { margin: 0; color: var(--text-faint); font-size: 11px; }
</style>
