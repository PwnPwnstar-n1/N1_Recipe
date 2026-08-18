<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { closeEditor, saveSnippet, store } from "../store";
import { SNIPPET_TYPES, type SnippetType } from "../types";

const form = reactive({
  title: "",
  content: "",
  type: "TEXT" as SnippetType,
  category: "",
  language: "",
  tags: "",
  aliases: "",
});

const error = ref("");

watch(
  () => store.editor,
  (editor) => {
    error.value = "";
    if (editor?.mode === "edit") {
      const src = store.items.find((i) => i.id === editor.id);
      if (src) {
        form.title = src.title;
        form.content = src.content;
        form.type = src.type;
        form.category = src.category;
        form.language = src.language;
        form.tags = src.tags.join(", ");
        form.aliases = src.aliases.join(", ");
        return;
      }
    }
    form.title = "";
    form.content = "";
    form.type = "TEXT";
    form.category = "";
    form.language = "";
    form.tags = "";
    form.aliases = "";
  },
  { immediate: true },
);

const isEdit = () => store.editor?.mode === "edit";

function splitCsv(value: string): string[] {
  return value
    .split(/[,，]/)
    .map((x) => x.trim())
    .filter(Boolean);
}

async function onSave(): Promise<void> {
  if (!form.title.trim()) {
    error.value = "Title is required";
    return;
  }
  await saveSnippet({
    title: form.title.trim(),
    content: form.content,
    type: form.type,
    category: form.category.trim(),
    language: form.language.trim(),
    tags: splitCsv(form.tags),
    aliases: splitCsv(form.aliases),
  });
}

function onKeydown(e: KeyboardEvent): void {
  if (e.ctrlKey && e.key.toLowerCase() === "s") {
    e.preventDefault();
    void onSave();
  }
}
</script>

<template>
  <div class="editor" @keydown="onKeydown">
    <h2 class="head">{{ isEdit() ? "Edit Snippet" : "New Snippet" }}</h2>

    <label class="field">
      <span class="label">Title *</span>
      <input v-model="form.title" type="text" placeholder="e.g. ret2libc EXP Template" />
    </label>

    <div class="row">
      <label class="field">
        <span class="label">Type</span>
        <select v-model="form.type">
          <option v-for="t in SNIPPET_TYPES" :key="t" :value="t">{{ t }}</option>
        </select>
      </label>
      <label class="field grow">
        <span class="label">Category</span>
        <input
          v-model="form.category"
          type="text"
          list="category-options"
          placeholder="e.g. Security/Pwn"
        />
        <datalist id="category-options">
          <option v-for="c in store.categories" :key="c.name" :value="c.name" />
        </datalist>
      </label>
    </div>

    <label class="field">
      <span class="label">Language</span>
      <input v-model="form.language" type="text" placeholder="python / bash / markdown …" />
    </label>

    <div class="row">
      <label class="field grow">
        <span class="label">Tags (comma separated)</span>
        <input v-model="form.tags" type="text" placeholder="pwn, pwntools" />
      </label>
      <label class="field grow">
        <span class="label">Aliases (comma separated)</span>
        <input v-model="form.aliases" type="text" placeholder="rtl, ret2libc" />
      </label>
    </div>

    <label class="field grow">
      <span class="label">Content</span>
      <textarea
        v-model="form.content"
        spellcheck="false"
        placeholder="Paste your snippet content here…"
      ></textarea>
    </label>

    <p v-if="error" class="error">{{ error }}</p>

    <div class="actions">
      <button class="btn primary" @click="onSave">Save (Ctrl+S)</button>
      <button class="btn" @click="closeEditor">Cancel (Esc)</button>
    </div>
  </div>
</template>

<style scoped>
.editor {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
}

.head {
  margin: 0;
  font-size: 16px;
  color: var(--text-bright);
}

.row {
  display: flex;
  gap: 10px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field.grow {
  flex: 1;
}

.label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.6px;
  text-transform: uppercase;
  color: var(--text-faint);
}

input,
select {
  padding: 6px 9px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg-panel);
  color: var(--text-bright);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  min-width: 0;
}

input:focus,
select:focus,
textarea:focus {
  border-color: var(--accent);
}

textarea {
  flex: 1;
  min-height: 240px;
  resize: vertical;
  padding: 9px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg-panel);
  color: var(--text);
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.55;
  outline: none;
}

.error {
  margin: 0;
  color: var(--danger);
  font-size: 12px;
}

.actions {
  display: flex;
  gap: 8px;
}

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
</style>
