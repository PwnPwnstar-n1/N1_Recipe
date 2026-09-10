<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { Snippet } from "../types";
import {
  categorySelectionState,
  groupRecipes,
  selectedRecipeIds,
  setCategorySelection,
} from "../recipeExportSelection";
import AppDialog from "./AppDialog.vue";
import { refresh, showToast, store } from "../store";

const preview = ref<api.PackPreview | null>(null);
const path = ref("");
const strategy = ref("skip");
const busy = ref(false);
const showExport = ref(false);
const recipes = ref<Snippet[]>([]);
const selected = ref(new Set<number>());
const expanded = ref(new Set<string>());
const groups = computed(() => groupRecipes(recipes.value));
const selectedCount = computed(() => selected.value.size);
const meta = reactive({
  id: "local.my-recipes",
  name: "我的配方包",
  version: "1.0.0",
  description: "",
  author: "",
  homepage: "",
  license: "",
  tags: "",
  minimumAppVersion: "0.2.0",
});

async function openExport(): Promise<void> {
  busy.value = true;
  try {
    recipes.value = await api.listSnippets("all");
    const defaults = store.view === "category"
      ? recipes.value.filter((recipe) => recipe.category === store.category)
      : recipes.value;
    selected.value = new Set(defaults.map((recipe) => recipe.id));
    expanded.value = new Set(store.view === "category" ? [store.category] : []);
    showExport.value = true;
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error));
  } finally {
    busy.value = false;
  }
}

function toggleExpanded(category: string): void {
  const next = new Set(expanded.value);
  if (next.has(category)) next.delete(category);
  else next.add(category);
  expanded.value = next;
}

function toggleCategory(group: (typeof groups.value)[number], checked: boolean): void {
  const next = new Set(selected.value);
  setCategorySelection(next, group.recipes, checked);
  selected.value = next;
}

function toggleRecipe(id: number, checked: boolean): void {
  const next = new Set(selected.value);
  if (checked) next.add(id);
  else next.delete(id);
  selected.value = next;
}

function selectAll(): void {
  selected.value = new Set(recipes.value.map((recipe) => recipe.id));
}

function clearAll(): void {
  selected.value = new Set();
}

async function chooseImport(): Promise<void> {
  const picked = await open({
    multiple: false,
    filters: [{ name: "N1 Recipe Pack", extensions: ["n1recipe.json", "json"] }],
  });
  if (typeof picked !== "string") return;
  try {
    path.value = picked;
    preview.value = await api.previewRecipePack(picked);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error));
  }
}

async function doImport(): Promise<void> {
  busy.value = true;
  try {
    const result = await api.importRecipePack(path.value, strategy.value);
    preview.value = null;
    await refresh();
    showToast(`导入完成：新增 ${result.added}，更新 ${result.updated}，跳过 ${result.skipped}`);
  } finally {
    busy.value = false;
  }
}

async function doExport(): Promise<void> {
  const ids = selectedRecipeIds(selected.value);
  if (ids.length === 0) return;
  const picked = await save({
    defaultPath: "n1-recipes.n1recipe.json",
    filters: [{ name: "N1 Recipe Pack", extensions: ["n1recipe.json"] }],
  });
  if (!picked) return;
  busy.value = true;
  try {
    const count = await api.exportRecipePack(
      picked,
      {
        ...meta,
        tags: meta.tags.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean),
        exportedAt: new Date().toISOString(),
      },
      null,
      ids,
    );
    showExport.value = false;
    showToast(`已导出 ${count} 条配方：${picked}`);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="pack-actions">
    <button :disabled="busy" @click="openExport">导出配方</button>
    <button @click="chooseImport">导入配方包</button>
  </div>
  <p class="hint">分享前请检查 Token、密码、Cookie、私钥和个人路径；应用不会修改或执行配方内容。</p>

  <AppDialog
    v-if="showExport"
    title="导出配方"
    :confirm-text="`导出 ${selectedCount} 个配方`"
    :confirm-disabled="selectedCount === 0"
    :busy="busy"
    @cancel="showExport = false"
    @confirm="doExport"
  >
    <div class="selection-toolbar">
      <strong>已选择 {{ selectedCount }} 个配方</strong>
      <span>
        <button type="button" @click="selectAll">全选</button>
        <button type="button" @click="clearAll">取消全选</button>
      </span>
    </div>
    <div v-if="groups.length" class="category-list">
      <section v-for="group in groups" :key="group.category" class="category-group">
        <div class="category-row">
          <button class="expand" type="button" @click="toggleExpanded(group.category)">
            {{ expanded.has(group.category) ? "▾" : "▸" }}
          </button>
          <input
            type="checkbox"
            :checked="categorySelectionState(group.recipes, selected) === 'all'"
            :indeterminate="categorySelectionState(group.recipes, selected) === 'partial'"
            @change="toggleCategory(group, ($event.target as HTMLInputElement).checked)"
          />
          <button class="category-name" type="button" @click="toggleExpanded(group.category)">
            {{ group.label }} <span>({{ group.recipes.length }})</span>
          </button>
        </div>
        <div v-if="expanded.has(group.category)" class="recipe-list">
          <label v-for="recipe in group.recipes" :key="recipe.id">
            <input
              type="checkbox"
              :checked="selected.has(recipe.id)"
              @change="toggleRecipe(recipe.id, ($event.target as HTMLInputElement).checked)"
            />
            <span>{{ recipe.title }}</span>
            <small v-if="recipe.kind === 'COMMAND_GROUP'">Command Group</small>
          </label>
        </div>
      </section>
    </div>
    <p v-else class="empty">没有可导出的配方。</p>

    <details class="package-details">
      <summary>配方包信息</summary>
      <div class="fields">
        <label>Package ID<input v-model="meta.id" /></label>
        <label>名称<input v-model="meta.name" /></label>
        <label>版本<input v-model="meta.version" /></label>
        <label>作者<input v-model="meta.author" /></label>
        <label>说明<textarea v-model="meta.description" /></label>
        <label>主页<input v-model="meta.homepage" /></label>
        <label>许可证<input v-model="meta.license" /></label>
        <label>标签<input v-model="meta.tags" placeholder="逗号分隔" /></label>
      </div>
    </details>
  </AppDialog>

  <AppDialog v-if="preview" title="导入配方包预览" confirm-text="确认导入" :busy="busy" @cancel="preview = null" @confirm="doImport">
    <div class="summary">
      <b>{{ preview.package.name }}</b>
      <p>{{ preview.package.author }} · {{ preview.package.version }}</p>
      <p>{{ preview.package.description }}</p>
      <p>分类 {{ preview.categoryCount }} · 配方 {{ preview.recipeCount }} · 普通文本 {{ preview.textCount }} · 命令组 {{ preview.commandGroupCount }}</p>
      <p>类型：{{ preview.types.join("、") || "无" }}</p>
      <p>标签：{{ preview.tags.join("、") || "无" }}</p>
      <p>检测到同来源冲突：{{ preview.conflicts }}</p>
      <p>主页：{{ preview.package.homepage || "无" }}</p>
      <label>冲突策略<select v-model="strategy"><option value="skip">跳过已有（默认）</option><option value="duplicate">保留两份</option><option value="overwrite">覆盖同来源配方</option></select></label>
    </div>
  </AppDialog>
</template>

<style scoped>
.pack-actions { display: flex; gap: 8px; }
.pack-actions button, .selection-toolbar button { padding: 7px 12px; border: 1px solid var(--border); border-radius: 7px; background: var(--surface); color: var(--text); cursor: pointer; }
.hint, .empty { color: var(--text-dim); font-size: var(--font-size-small); }
.selection-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
.selection-toolbar span { display: flex; gap: 6px; }
.category-list { max-height: 360px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; }
.category-group + .category-group { border-top: 1px solid var(--border); }
.category-row { display: flex; align-items: center; gap: 7px; padding: 8px 10px; background: var(--surface); }
.expand, .category-name { border: 0; background: transparent; color: var(--text); cursor: pointer; text-align: left; }
.expand { width: 20px; padding: 0; }
.category-name { flex: 1; font-weight: 600; }
.category-name span { color: var(--text-dim); font-weight: 400; }
.recipe-list { display: grid; gap: 2px; padding: 5px 10px 8px 38px; }
.recipe-list label { display: flex; align-items: center; gap: 8px; padding: 5px 0; min-width: 0; }
.recipe-list label span { flex: 1; overflow-wrap: anywhere; }
.recipe-list small { color: var(--text-dim); }
.package-details { margin-top: 12px; color: var(--text-dim); }
.package-details summary { cursor: pointer; }
.fields { display: grid; gap: 10px; margin-top: 10px; }
.fields label, .summary label { display: grid; gap: 4px; color: var(--text-dim); }
input, textarea, select { width: 100%; padding: 8px; border: 1px solid var(--border); border-radius: 7px; background: var(--surface); color: var(--text); }
input[type="checkbox"] { width: auto; padding: 0; accent-color: var(--accent); }
textarea { min-height: 80px; }
.summary p { overflow-wrap: anywhere; }
</style>
