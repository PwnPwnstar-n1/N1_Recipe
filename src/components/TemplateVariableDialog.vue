<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";

// Reusable template-variable dialog. Driven purely by props/emits so the
// main window, the future launcher and the edge dock can all use it without
// sharing main-window store state.
const props = defineProps<{
  title: string;
  variables: string[];
}>();

const emit = defineEmits<{
  (e: "confirm", values: Record<string, string>): void;
  (e: "cancel"): void;
}>();

const { t } = useI18n();
const container = ref<HTMLElement | null>(null);
const values = reactive<Record<string, string>>({});

for (const name of props.variables) values[name] = "";

onMounted(async () => {
  await nextTick();
  container.value?.querySelector("input")?.focus();
});

function onConfirm(): void {
  emit("confirm", { ...values });
}

function onCancel(): void {
  emit("cancel");
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
  <div class="overlay" @mousedown.self="onCancel" @keydown="onKeydown">
    <div ref="container" class="dialog">
      <h2 class="title">{{ props.title }}</h2>
      <p class="subtitle">{{ t("dialog.fillVariables") }}</p>

      <label v-for="name in props.variables" :key="name" class="field">
        <span class="name">{{ name }}</span>
        <input
          v-model="values[name]"
          type="text"
          spellcheck="false"
          :placeholder="name"
        />
      </label>

      <div class="actions">
        <button class="btn primary" @click="onConfirm">{{ t("dialog.copy") }}</button>
        <button class="btn" @click="onCancel">{{ t("dialog.cancel") }}</button>
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
