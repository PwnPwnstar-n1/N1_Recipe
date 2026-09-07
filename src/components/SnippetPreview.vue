<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { formatDateTime } from "../format";
import {
  copySnippet,
  openEdit,
  removeSnippet,
  selected,
  toggleFavorite,
} from "../store";
import * as api from "../api";
import TemplateVariableDialog from"./TemplateVariableDialog.vue";

const { t } = useI18n();
const s = computed(() => selected());
const confirmDelete = ref(false);
const copied = ref(false);
let copiedTimer: number | undefined;

watch(
  () => s.value?.id,
  () => {
    confirmDelete.value = false;
    window.clearTimeout(copiedTimer);
    copied.value = false;
  },
);

async function onCopy(): Promise<void> {
  if (!s.value) return;
  await copySnippet(s.value);
  copied.value = true;
  window.clearTimeout(copiedTimer);
  copiedTimer = window.setTimeout(() => (copied.value = false), 1200);
}

function onToggleFav(): void {
  if (s.value) void toggleFavorite(s.value);
}

function onDelete(): void {
  if (!s.value) return;
  if (!confirmDelete.value) {
    confirmDelete.value = true;
    window.setTimeout(() => (confirmDelete.value = false), 3000);
    return;
  }
  void removeSnippet(s.value.id);
}

function onEdit(): void {
  if (s.value) openEdit(s.value.id);
}
const commandDialog=ref<{text:string;variables:string[]}|null>(null);async function copyCommand(text:string){const variables=await api.listTextVariables(text);if(variables.length){commandDialog.value={text,variables};return}await api.copyTemplateText(text,{},s.value?.id);copied.value=true}async function confirmCommand(values:Record<string,string>){if(!commandDialog.value)return;await api.copyTemplateText(commandDialog.value.text,values,s.value?.id);commandDialog.value=null;copied.value=true}
async function copyAllCommands(){if(!s.value)return;await copyCommand(s.value.commands.join("\n"))}
async function togglePin(){if(s.value){await api.toggleDockPin(s.value.id);await import("../store").then(m=>m.refresh())}}
</script>

<template>
  <div v-if="s" class="preview">
    <div class="head">
      <span class="badge" :class="`type-${s.type.toLowerCase()}`">{{ s.type }}</span>
      <h2 class="title">{{ s.title }}</h2>
      <button
        class="icon-btn"
        :class="{ on: s.favorite }"
        :title="s.favorite ? t('preview.unfavorite') : t('preview.favorite')"
        @click="onToggleFav"
      >
        {{ s.favorite ? "★" : "☆" }}
      </button>
    </div>

    <div class="meta-grid">
      <div v-if="s.category" class="meta-row">
        <span class="k">{{ t("preview.category") }}</span><span class="v">{{ s.category }}</span>
      </div>
      <div v-if="s.language" class="meta-row">
        <span class="k">{{ t("preview.language") }}</span><span class="v">{{ s.language }}</span>
      </div>
      <div v-if="s.tags.length" class="meta-row">
        <span class="k">{{ t("preview.tags") }}</span>
        <span class="v">
          <span v-for="tag in s.tags" :key="tag" class="chip">{{ tag }}</span>
        </span>
      </div>
      <div v-if="s.aliases.length" class="meta-row">
        <span class="k">{{ t("preview.aliases") }}</span>
        <span class="v">
          <span v-for="a in s.aliases" :key="a" class="chip alias">{{ a }}</span>
        </span>
      </div>
      <div class="meta-row">
        <span class="k">{{ t("preview.used") }}</span>
        <span class="v">
          {{ t("preview.usedValue", { count: s.usageCount }) }}
          <span v-if="s.lastUsedAt" class="dim">
            · {{ t("preview.last", { time: formatDateTime(s.lastUsedAt) }) }}
          </span>
        </span>
      </div>
      <div class="meta-row">
        <span class="k">{{ t("preview.updated") }}</span>
        <span class="v">{{ formatDateTime(s.updatedAt) }}</span>
      </div>
    </div>

    <div v-if="s.kind==='COMMAND_GROUP'" class="commands"><div v-for="(command,i) in s.commands" :key="i" class="command"><span>{{i+1}}</span><code>{{command}}</code><button @click="copyCommand(command)">复制</button></div></div><pre v-else class="content">{{ s.content }}</pre>

    <div class="actions">
      <button class="btn" @click="togglePin">{{s.dockPinned?'取消固定':'固定到 Dock'}}</button>
      <button class="btn primary" @click="s.kind==='COMMAND_GROUP'?copyAllCommands():onCopy()">
        {{ copied ? t("preview.copied") : t("preview.copy") }}
      </button>
      <button class="btn" @click="onEdit">{{ t("preview.edit") }}</button>
      <button class="btn danger" @click="onDelete">
        {{ confirmDelete ? t("preview.confirmDelete") : t("preview.delete") }}
      </button>
    </div>
  </div>
  <TemplateVariableDialog v-if="commandDialog&&s" :title="s.title" :variables="commandDialog.variables" @confirm="confirmCommand" @cancel="commandDialog=null"/>
</template>

<style scoped>
.preview {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  height:100%;min-height:0;overflow:hidden;
}

.head {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.title {
  flex: 1;
  margin: 0;
  font-size: 17px;
  line-height: 1.35;
  color: var(--text-bright);
  overflow-wrap: anywhere;
}

.icon-btn {
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 17px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
}

.icon-btn.on {
  color: var(--star);
}

.meta-grid {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 10px 12px;
  background: var(--bg-panel);
  border-radius: 6px;
}

.meta-row {
  display: flex;
  gap: 10px;
  font-size: 12px;
  align-items: baseline;
}

.k {
  width: 72px;
  flex-shrink: 0;
  color: var(--text-faint);
  text-transform: uppercase;
  font-size: 10px;
  letter-spacing: 0.6px;
  padding-top: 2px;
}

.v {
  color: var(--text);
  overflow-wrap: anywhere;
}

.dim {
  color: var(--text-faint);
}

.content {
  flex: 1;
  margin: 0;
  padding: 12px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
  overflow:auto;min-height:0;
}

.actions {
  display: flex;
  gap: 8px;
}
.commands{flex:1;min-height:0;overflow:auto;display:flex;flex-direction:column;gap:8px}.command{display:flex;gap:8px;align-items:flex-start;padding:10px;background:var(--bg-panel);border:1px solid var(--border);border-radius:var(--radius)}.command>span{color:var(--text-faint)}.command code{flex:1;min-width:0;white-space:pre-wrap;overflow-wrap:anywhere;font-family:var(--mono)}.command button{border:1px solid var(--border);background:transparent;color:var(--accent);border-radius:4px;padding:4px 8px;cursor:pointer}

.btn {
  padding: 6px 14px;
  border-radius: 5px;
  border: 1px solid var(--border);
  background: var(--bg-panel);
  color: var(--text);
  font-size: 12.5px;
  cursor: pointer;
}

.btn:hover {
  background: var(--bg-hover);
}

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #0d1117;
  font-weight: 600;
}

.btn.primary:hover {
  filter: brightness(1.1);
}

.btn.danger {
  color: var(--danger);
}

.btn.danger:hover {
  background: rgba(241, 76, 76, 0.12);
  border-color: var(--danger);
}
</style>
