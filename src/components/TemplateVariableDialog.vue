<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import {
  cancelVariableDialog,
  confirmVariableDialog,
  store,
} from "../store";

const dialog = computed(() => store.variableDialog);
const container = ref<HTMLElement | null>(null);
const values = reactive<Record<string, string>>({});

// Reset values and focus the first input each time a dialog opens.
watch(
  () => dialog.value?.id,
  async (id) => {
    for (const key of Object.keys(values)) delete values[key];
    if (id !== undefined) {
      for (const name of dialog.value?.variables ?? []) values[name] = "";
      await nextTick();
      container.value?.querySelector("input")?.focus();
    }
  },
);

function onConfirm(): void {
  void confirmVariableDialog({ ...values });
}

function onCancel(): void {
  cancelVariableDialog();
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    onCancel();
  } else if (e.key === "Enter") {
    e.preventDefault();
    onConfirm();
  }
}
</script>

<template>
  <div
    v-if="dialog"
    class="overlay"
    @mousedown.self="onCancel"
    @keydown="onKeydown"
  >
    <div ref="container" class="dialog">
      <h2 class="title">{{ dialog.title }}</h2>
      <p class="subtitle">Fill template variables</p>

      <label v-for="name in dialog.variables" :key="name" class="field">
        <span class="name">{{ name }}</span>
        <input
          v-model="values[name]"
          type="text"
          spellcheck="false"
          :placeholder="name"
        />
      </label>

      <div class="actions">
        <button class="btn primary" @click="onConfirm">Copy</button>
        <button class="btn" @click="onCancel">Cancel</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 90;
}

.dialog {
  width: 420px;
  max-width: calc(100vw - 48px);
  max-height: calc(100vh - 48px);
  overflow-y: auto;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 18px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.title {
  margin: 0;
  font-size: 15px;
  color: var(--text-bright);
  overflow-wrap: anywhere;
}

.subtitle {
  margin: 0;
  font-size: 11px;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: 0.6px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.name {
  font-family: var(--mono);
  font-size: 12px;
  color: var(--accent);
}

input {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--text-bright);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}

input:focus {
  border-color: var(--accent);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
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

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #0d1117;
  font-weight: 600;
}
</style>
