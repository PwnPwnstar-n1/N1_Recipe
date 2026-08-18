<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { changeLanguage, store } from "../store";
import type { AppLocale } from "../i18n";

const { t } = useI18n();

const options: { value: AppLocale; label: string }[] = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en-US", label: "English" },
];

function onChange(e: Event): void {
  const target = e.target as HTMLSelectElement;
  void changeLanguage(target.value as AppLocale);
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
        @change="onChange"
      >
        <option v-for="opt in options" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
      <p class="hint">{{ t("settings.languageHint") }}</p>
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
  gap: 6px;
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

.select:focus {
  border-color: var(--accent);
}

.hint {
  margin: 0;
  font-size: 11px;
  color: var(--text-faint);
}
</style>
