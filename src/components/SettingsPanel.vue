<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import * as api from "../api";
import { changeLanguage, store, updateLauncherSettings } from "../store";
import type { AppLocale } from "../i18n";

const { t } = useI18n();

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
</script>

<template>
  <div class="settings">
    <h2 class="title">{{ t("settings.title") }}</h2>

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
  </div>
</template>

<style scoped>
.settings {
  padding: 16px;
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
