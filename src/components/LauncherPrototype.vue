<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

async function hide(): Promise<void> {
  try {
    await invoke("hide_launcher");
  } catch {
    // prototype: nothing to recover from
  }
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.preventDefault();
    void hide();
  }
}
</script>

<template>
  <div class="launcher" tabindex="0" @keydown="onKeydown">
    <div class="logo">⌘ SnipDeck</div>
    <p class="hint">{{ t("launcher.prototype") }}</p>
    <button class="btn" @click="hide">{{ t("launcher.hide") }}</button>
  </div>
</template>

<style scoped>
.launcher {
  height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  background: var(--bg-panel);
  outline: none;
}

.logo {
  font-size: 18px;
  font-weight: 600;
  color: var(--accent);
}

.hint {
  margin: 0;
  color: var(--text-dim);
  font-size: 13px;
}

.btn {
  padding: 6px 16px;
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
</style>
