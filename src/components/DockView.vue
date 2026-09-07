<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import {
  cancelVariableDialog,
  collapseNow,
  confirmVariableDialog,
  dock,
  executeSelected,
  loadContent,
  mouseEnterDock,
  mouseLeaveDock,
  moveSelection,
  openTextCopy,
  setDockSettings,
  setDockState,
} from "../dockStore";
import type { DockVisualState } from "../dockStore";
import TemplateVariableDialog from "./TemplateVariableDialog.vue";

const { t } = useI18n();
const searchEl = ref<HTMLInputElement | null>(null);
const unlisteners: (() => void)[] = [];
const expandedId=ref<number|null>(null);
async function copyStep(text:string){const s=dock.results.find(x=>x.id===expandedId.value);if(s)await openTextCopy(text,s.title,s.id)}

function onKeydown(e: KeyboardEvent): void {
  if (dock.state !== "expanded") return;
  if (dock.variableDialog) return; // dialog owns the keyboard
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
    collapseNow();
  }
}

onMounted(async () => {
  // The dock owns its local expand/collapse transitions (mouse/keyboard/
  // copy). Backend state events are only adopted for EXTERNALLY initiated
  // transitions — enable/disable from the main window — so a late event
  // from an already-finished command can never overwrite local state.
  unlisteners.push(
    await listen<string>("dock:state", (event) => {
      const payload = event.payload as DockVisualState;
      if (payload === "hidden") {
        setDockState("hidden");
      } else if (payload === "collapsed" && dock.state === "hidden") {
        setDockState("collapsed");
      }
      if (payload === "expanded") {
        void loadContent();
        searchEl.value?.focus();
      }
    }),
    await listen("dock:settings", (event) => {
      setDockSettings(event.payload as api.Settings["dock"]);
    }),
  );
  try {
    const settings = await api.getSettings();
    setDockSettings(settings.dock);
  } catch {
    // keep defaults (autoHide true, hideDelay 400)
  }
  void loadContent();
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  for (const unlisten of unlisteners) unlisten();
  window.removeEventListener("keydown", onKeydown);
});

function onRowClick(index: number): void {
  dock.selectedIndex = index;
}

function onRowDoubleClick(): void {
  void executeSelected();
}
</script>

<template>
  <div
    class="dock-root"
    :class="[dock.state, { left: dock.settings?.position === 'left' }]"
    @mouseenter="mouseEnterDock"
    @mouseleave="mouseLeaveDock"
  >
    <div v-if="dock.state !== 'expanded'" class="handle" :title="t('settings.dock')">
      <span class="dots">⋯</span>
    </div>

    <div v-else class="content">
      <div class="search-row">
        <span class="search-icon">🔍</span>
        <input
          ref="searchEl"
          v-model="dock.query"
          class="search-input"
          type="text"
          :placeholder="t('dock.placeholder')"
          spellcheck="false"
        />
      </div>

      <div v-if="!dock.query.trim()" class="sections">
        <div class="section-header">
          ⭐ {{ t("dock.favorites") }}
          <span class="count">{{ dock.favorites.length }}</span>
        </div>
        <div class="section-header">
          ◷ {{ t("dock.recent") }}
          <span class="count">{{ dock.recent.length }}</span>
        </div>
      </div>

      <div v-if="dock.results.length" class="results">
        <div
          v-for="(s, i) in dock.results"
          :key="s.id"
          class="row"
          :class="{ active: i === dock.selectedIndex }"
          @mouseenter="dock.selectedIndex = i"
          @click="onRowClick(i)"
          @dblclick="onRowDoubleClick"
        >
          <span class="badge" :class="`type-${s.type.toLowerCase()}`">{{ s.type }}</span>
          <span class="title">{{ s.title }}</span>
          <span v-if="s.favorite" class="star">★</span>
          <button v-if="s.kind==='COMMAND_GROUP'" class="expand" @click.stop="expandedId=expandedId===s.id?null:s.id">{{expandedId===s.id?'收起':'展开'}}</button>
          <div v-if="expandedId===s.id" class="steps"><div v-for="(cmd,n) in s.commands" :key="n"><code>{{cmd}}</code><button @click.stop="copyStep(cmd)">复制</button></div><button @click.stop="copyStep(s.commands.join('\n'))">复制全部</button></div>
        </div>
      </div>
      <div v-else-if="!dock.loading" class="empty">
        {{ t("dock.empty") }}
      </div>

      <div v-if="dock.copied" class="status">{{ t("dock.copied") }}</div>
      <div v-if="dock.error" class="error">{{ dock.error }}</div>
    </div>

    <TemplateVariableDialog
      v-if="dock.variableDialog"
      :title="dock.variableDialog.title"
      :variables="dock.variableDialog.variables"
      @confirm="confirmVariableDialog"
      @cancel="cancelVariableDialog"
    />
  </div>
</template>

<style scoped>
.dock-root {
  height: 100vh;
  width: 100vw;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  overflow: hidden;
  user-select: none;
  outline: none;
}

.dock-root.left {
  border-width: 1px 0 1px 1px;
}

/* collapsed: slim edge handle */
.handle {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  background: var(--bg-panel);
}

.dots {
  writing-mode: vertical-rl;
  color: var(--text-faint);
  font-size: 11px;
  letter-spacing: 3px;
}

.handle:hover {
  background: var(--bg-hover);
}

.handle:hover .dots {
  color: var(--accent);
}

/* expanded content */
.content {
  height: 100vh;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.search-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}

.search-icon {
  font-size: 12px;
  opacity: 0.7;
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-bright);
  font-size: 13px;
  font-family: inherit;
}

.search-input::placeholder {
  color: var(--text-faint);
}

.sections {
  padding: 8px 12px 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.section-header {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: var(--text-dim);
}

.count {
  color: var(--text-faint);
  font-weight: 400;
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
  padding: 7px 9px;
  border-radius: 5px;
  border-left: 2px solid transparent;
  cursor: pointer;
}
.row{flex-wrap:wrap}.expand{border:0;background:transparent;color:var(--accent);cursor:pointer}.steps{width:100%;min-width:0;display:flex;flex-direction:column;gap:5px;padding:6px}.steps>div{display:flex;gap:6px}.steps code{flex:1;min-width:0;white-space:pre-wrap;overflow-wrap:anywhere;font-family:var(--mono)}.steps button{border:1px solid var(--border);background:var(--surface);color:var(--accent);border-radius:5px}

.row:hover {
  background: var(--bg-hover);
}

.row.active {
  background: var(--bg-active);
  border-left-color: var(--accent);
}

.title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
  font-size: 12.5px;
}

.row.active .title {
  color: var(--text-bright);
}

.star {
  color: var(--star);
  font-size: 11px;
}

.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
  font-size: 12px;
}

.status,
.error {
  padding: 6px 12px;
  font-size: 12px;
}

.status {
  color: var(--accent);
}

.error {
  color: var(--danger);
  border-top: 1px solid var(--border);
}
</style>
