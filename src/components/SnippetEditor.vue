<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { closeEditor, saveSnippet, store } from "../store";
import { SNIPPET_TYPES, type SnippetType } from "../types";

const { t } = useI18n();

const form = reactive({
  title: "",
  content: "",
  type: "TEXT" as SnippetType,
  category: "",
  language: "",
  tags: "",
  aliases: "",
  kind: "TEXT" as "TEXT"|"COMMAND_GROUP",
  commands: [""] as string[],
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
        form.kind=src.kind; form.commands=src.commands.length?[...src.commands]:[""];
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
    form.kind="TEXT";form.commands=[""];
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
    error.value = t("errors.titleRequired");
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
    kind: form.kind,
    commands: form.kind==="COMMAND_GROUP"?form.commands.map(x=>x.trim()).filter(Boolean):[],
  });
}
function importLines(){const lines=form.content.split(/\r?\n/).map(x=>x.trim()).filter(Boolean);if(lines.length){form.commands=lines;form.content=""}}
function move(index:number,delta:number){const next=index+delta;if(next<0||next>=form.commands.length)return;[form.commands[index],form.commands[next]]=[form.commands[next],form.commands[index]]}

function onKeydown(e: KeyboardEvent): void {
  if (e.ctrlKey && e.key.toLowerCase() === "s") {
    e.preventDefault();
    void onSave();
  }
}
</script>

<template>
  <div class="editor" @keydown="onKeydown">
    <h2 class="head">{{ isEdit() ? t("editor.editTitle") : t("editor.newTitle") }}</h2>

    <label class="field">
      <span class="label">{{ t("editor.title") }}</span>
      <input v-model="form.title" type="text" :placeholder="t('editor.titlePlaceholder')" />
    </label>
    <label class="field"><span class="label">内容模式</span><select v-model="form.kind"><option value="TEXT">普通文本</option><option value="COMMAND_GROUP">命令组</option></select></label>

    <div class="row">
      <label class="field">
        <span class="label">{{ t("editor.type") }}</span>
        <select v-model="form.type">
          <option v-for="type in SNIPPET_TYPES" :key="type" :value="type">{{ type }}</option>
        </select>
      </label>
      <label class="field grow">
        <span class="label">{{ t("editor.category") }}</span>
        <input
          v-model="form.category"
          type="text"
          list="category-options"
          :placeholder="t('editor.categoryPlaceholder')"
        />
        <datalist id="category-options">
          <option v-for="c in store.categories" :key="c.name" :value="c.name" />
        </datalist>
      </label>
    </div>

    <label class="field">
      <span class="label">{{ t("editor.language") }}</span>
      <input v-model="form.language" type="text" :placeholder="t('editor.languagePlaceholder')" />
    </label>

    <div class="row">
      <label class="field grow">
        <span class="label">{{ t("editor.tags") }}</span>
        <input v-model="form.tags" type="text" :placeholder="t('editor.tagsPlaceholder')" />
      </label>
      <label class="field grow">
        <span class="label">{{ t("editor.aliases") }}</span>
        <input v-model="form.aliases" type="text" :placeholder="t('editor.aliasesPlaceholder')" />
      </label>
    </div>

    <label v-if="form.kind==='TEXT'" class="field grow">
      <span class="label">{{ t("editor.content") }}</span>
      <textarea
        v-model="form.content"
        spellcheck="false"
        :placeholder="t('editor.contentPlaceholder')"
      ></textarea>
    </label>
    <div v-else class="command-editor"><div class="command-tools"><span class="label">命令（按顺序）</span><button class="btn" @click="form.commands.push('')">＋ 新增</button></div><div v-for="(_,i) in form.commands" :key="i" class="command-row"><textarea v-model="form.commands[i]" spellcheck="false" placeholder="输入一条命令"></textarea><div><button class="mini" :disabled="i===0" @click="move(i,-1)">↑</button><button class="mini" :disabled="i===form.commands.length-1" @click="move(i,1)">↓</button><button class="mini danger" @click="form.commands.splice(i,1)">×</button></div></div><label class="field"><span class="label">多行快速导入</span><textarea v-model="form.content" class="import" placeholder="每个非空行导入为一条命令"></textarea></label><button class="btn" :disabled="!form.content.trim()" @click="importLines">导入非空行</button></div>

    <p v-if="error" class="error">{{ error }}</p>

    <div class="actions">
      <button class="btn primary" @click="onSave">{{ t("editor.save") }}</button>
      <button class="btn" @click="closeEditor">{{ t("editor.cancel") }}</button>
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
  overflow-y:auto;min-height:0;
}
.command-editor{display:flex;flex-direction:column;gap:8px}.command-tools,.command-row{display:flex;gap:8px;align-items:center}.command-tools .label{margin-right:auto}.command-row textarea{min-height:58px;resize:vertical}.command-row>div{display:flex;flex-direction:column}.mini{border:0;background:transparent;color:var(--text);cursor:pointer}.mini.danger{color:var(--danger)}textarea.import{min-height:80px}

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
