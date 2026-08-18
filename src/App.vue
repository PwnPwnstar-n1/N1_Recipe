<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import {
  cancelVariableDialog,
  closeEditor,
  copySelectedOrFirst,
  focusSearch,
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

onMounted(() => {
  void refresh();
  focusSearch();
  window.addEventListener("keydown", onGlobalKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});

const showEditor = computed(() => store.editor !== null);
const showPreview = computed(() => store.editor === null && selected() !== null);

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
      else store.query = "";
      break;
  }
}
</script>

<template>
  <div class="app">
    <Sidebar />
    <section class="main-pane">
      <SearchBar />
      <SnippetList />
    </section>
    <aside class="detail-pane">
      <SnippetEditor v-if="showEditor" />
      <SnippetPreview v-else-if="showPreview" />
      <div v-else class="empty-detail">
        <p class="empty-title">No snippet selected</p>
        <p class="empty-hint">
          ↑ ↓ navigate · Enter copy · Ctrl+E edit · Ctrl+N new · Ctrl+F search
        </p>
      </div>
    </aside>
    <div v-if="store.toast" class="toast">{{ store.toast }}</div>
    <TemplateVariableDialog />
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
