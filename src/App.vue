<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  cancelVariableDialog,
  closeEditor,
  confirmVariableDialog,
  copySelectedOrFirst,
  focusSearch,
  initSettings,
  moveSelection,
  openCreate,
  openEdit,
  refresh,
  selected,
  store,
} from "./store";
import Sidebar from "./components/Sidebar.vue";
import SearchBar from "./components/SearchBar.vue";
import SnippetList from "./components/SnippetList.vue";
import SnippetPreview from "./components/SnippetPreview.vue";
import SnippetEditor from "./components/SnippetEditor.vue";
import TemplateVariableDialog from "./components/TemplateVariableDialog.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import LauncherView from "./components/LauncherView.vue";
import DockView from "./components/DockView.vue";

const { t } = useI18n();

// Multi-window foundation: each window renders this app bundle and picks its
// role from the window label. Only the main window boots the management UI.
type WindowRole = "main" | "launcher" | "dock";
const windowRole = ref<WindowRole>("main");

onMounted(async () => {
  windowRole.value = await detectWindowRole();
  await initSettings();
  if (windowRole.value === "main") {
    void refresh();
    focusSearch();
    window.addEventListener("keydown", onGlobalKeydown);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});

async function detectWindowRole(): Promise<WindowRole> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const label = getCurrentWindow().label;
    if (label === "launcher") return "launcher";
    if (label === "dock") return "dock";
    return "main";
  } catch {
    // plain browser (vite without Tauri) — assume main
    return "main";
  }
}

const showEditor = computed(() => store.editor !== null);
const showPreview = computed(
  () => store.editor === null && !store.settingsOpen && selected() !== null,
);

const TYPING_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT", "BUTTON"]);

function onGlobalKeydown(e: KeyboardEvent): void {
  // While the variable dialog is open it owns the keyboard: only Escape
  // (cancel) is handled here; the dialog itself handles Enter/Escape on its
  // fields. Swallowing the rest prevents background copy/edit shortcuts.
  if (store.variableDialog) {
    if (e.key === "Escape") {
      e.preventDefault();
      cancelVariableDialog();
    }
    return;
  }

  const target = e.target as HTMLElement | null;
  if (target && TYPING_TAGS.has(target.tagName)) return;

  if (e.ctrlKey && e.key.toLowerCase() === "n") {
    e.preventDefault();
    openCreate();
    return;
  }
  if (e.ctrlKey && e.key.toLowerCase() === "e") {
    e.preventDefault();
    const s = selected();
    if (s) openEdit(s.id);
    return;
  }
  if (e.ctrlKey && e.key.toLowerCase() === "f") {
    e.preventDefault();
    focusSearch();
    return;
  }

  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      moveSelection(1);
      break;
    case "ArrowUp":
      e.preventDefault();
      moveSelection(-1);
      break;
    case "Enter":
      e.preventDefault();
      void copySelectedOrFirst();
      break;
    case "Escape":
      if (store.editor) closeEditor();
      else if (store.settingsOpen) store.settingsOpen = false;
      else store.query = "";
      break;
  }
}
</script>

<template>
  <LauncherView v-if="windowRole === 'launcher'" />
  <DockView v-else-if="windowRole === 'dock'" />

  <div v-else class="app">
    <Sidebar />
    <section class="main-pane">
      <SearchBar />
      <SnippetList />
    </section>
    <aside class="detail-pane">
      <SnippetEditor v-if="showEditor" />
      <SettingsPanel v-else-if="store.settingsOpen" />
      <SnippetPreview v-else-if="showPreview" />
      <div v-else class="empty-detail">
        <p class="empty-title">{{ t("app.noSelection") }}</p>
        <p class="empty-hint">{{ t("app.shortcutsHint") }}</p>
      </div>
    </aside>
    <div v-if="store.toast" class="toast">{{ store.toast }}</div>
    <TemplateVariableDialog
      v-if="store.variableDialog"
      :title="store.variableDialog.title"
      :variables="store.variableDialog.variables"
      @confirm="confirmVariableDialog"
      @cancel="cancelVariableDialog"
    />
  </div>
</template>

<style scoped>
.app {
  display: grid;
  grid-template-columns: 220px minmax(340px, 1fr) minmax(360px, 460px);
  height: 100vh;
}

.main-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--bg);
}

.detail-pane {
  overflow-y: auto;
  background: var(--bg);
  border-left: 1px solid var(--border);
}

.empty-detail {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-dim);
}

.empty-title {
  font-size: 14px;
}

.empty-hint {
  font-size: 12px;
  color: var(--text-faint);
}

.toast {
  position: fixed;
  right: 16px;
  bottom: 16px;
  padding: 8px 14px;
  background: var(--bg-active);
  border: 1px solid var(--accent);
  border-radius: 6px;
  color: var(--text-bright);
  font-size: 12px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  z-index: 100;
}
</style>
