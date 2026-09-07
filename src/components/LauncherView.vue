<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  cancelVariableDialog,
  confirmVariableDialog,
  executeSelected,
  hideLauncher,
  launcher,
  moveSelection,
} from "../launcherStore";
import TemplateVariableDialog from "./TemplateVariableDialog.vue";

const { t } = useI18n();
const inputEl = ref<HTMLInputElement | null>(null);
const appWindow = getCurrentWindow();
let unlistenFocus: (() => void) | undefined;


function focusSearch(): void {
  inputEl.value?.focus();
  inputEl.value?.select();
}

function onKeydown(e: KeyboardEvent): void {
  if (launcher.variableDialog) return; // dialog owns the keyboard
  if (e.key === "ArrowDown") {
    e.preventDefault();
    moveSelection(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    moveSelection(-1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    void executeSelected();
  } else if (e.key === "Escape") {
    e.preventDefault();
    void hideLauncher();
  }
}

onMounted(async () => {
  // Every show focuses the search input; losing window focus hides the
  // launcher (palette behavior).
  unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }) => {
    if (focused) {
      focusSearch();
    } else {
      void hideLauncher();
    }
  });
  // Window-level listener: key events keep working no matter which element
  // has focus (e.g. after the variable dialog closes and focus falls back
  // to the body).
  window.addEventListener("keydown", onKeydown);
  focusSearch();
});

onUnmounted(() => {
  unlistenFocus?.();
  window.removeEventListener("keydown", onKeydown);
});

// Closing the variable dialog (cancel or confirm) returns focus to search.
watch(
  () => launcher.variableDialog,
  async (dialog) => {
    if (dialog === null) {
      await nextTick();
      focusSearch();
    }
  },
);
</script>

<template>
  <div class="launcher">
    <div class="search-row">
      <span class="search-icon">🔍</span>
      <input
        ref="inputEl"
        v-model="launcher.query"
        class="search-input"
        type="text"
        :placeholder="t('launcher.placeholder')"
        spellcheck="false"
      />
    </div>

    <div v-if="launcher.results.length" class="results">
      <div
        v-for="(s, i) in launcher.results"
        :key="s.id"
        class="row"
        :class="{ active: i === launcher.selectedIndex }"
        @mouseenter="launcher.selectedIndex = i"
        @click="launcher.selectedIndex = i"
        @dblclick="executeSelected"
      >
        <span class="badge" :class="`type-${s.type.toLowerCase()}`">{{ s.type }}</span>
        <span class="title">{{ s.title }}</span>
        <span v-if="s.tags.length" class="tags">
          <span v-for="tag in s.tags" :key="tag" class="chip">{{ tag }}</span>
        </span>
        <span class="spacer"></span>
        <span v-if="s.usageCount > 0" class="usage">×{{ s.usageCount }}</span>
      </div>
    </div>
    <div v-else-if="!launcher.loading" class="empty">
      {{ t("launcher.empty") }}
    </div>

    <div v-if="launcher.copied" class="status">{{ t("launcher.copied") }}</div>
    <div v-if="launcher.error" class="error">{{ launcher.error }}</div>

    <TemplateVariableDialog
      v-if="launcher.variableDialog"
      :title="launcher.variableDialog.title"
      :variables="launcher.variableDialog.variables"
      @confirm="confirmVariableDialog"
      @cancel="cancelVariableDialog"
    />
  </div>
</template>

<style scoped>
.launcher {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  user-select: none;
  outline: none;
}

.search-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
}

.search-icon {
  font-size: 13px;
  opacity: 0.7;
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-bright);
  font-size: 14px;
  font-family: inherit;
}

.search-input::placeholder {
  color: var(--text-faint);
}

.results {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  min-height: 0;
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  border-left: 2px solid transparent;
  cursor: pointer;
}

.row:hover {
  background: var(--bg-hover);
}

.row.active {
  background: var(--bg-active);
  border-left-color: var(--accent);
}

.title {
  flex-shrink: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
  font-size: 13px;
}

.row.active .title {
  color: var(--text-bright);
}

.tags {
  display: flex;
  gap: 4px;
  overflow: hidden;
}

.chip {
  padding: 1px 7px;
  border-radius: 8px;
  background: var(--bg-hover);
  color: var(--text-faint);
  font-size: 10.5px;
  white-space: nowrap;
}

.spacer {
  flex: 1;
}

.usage {
  color: var(--accent);
  font-size: 11px;
}

.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
  font-size: 13px;
}

.status,
.error {
  padding: 6px 14px;
  font-size: 12px;
  text-align: center;
}

.status {
  color: var(--accent);
}

.error {
  color: var(--danger);
  border-top: 1px solid var(--border);
}
</style>
