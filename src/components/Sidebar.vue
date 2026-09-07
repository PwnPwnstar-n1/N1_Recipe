<script setup lang="ts">
import { useI18n } from "vue-i18n";import{ref}from"vue";import InputDialog from"./InputDialog.vue";import ConfirmDialog from"./ConfirmDialog.vue";
import { createCategory, deleteCategory, openCreate, renameCategory, setView, store, toggleSettings } from "../store";
import type { ViewName } from "../types";

const { t } = useI18n();

const views: { key: ViewName; labelKey: string; icon: string }[] = [
  { key: "all", labelKey: "sidebar.all", icon: "▤" },
  { key: "favorites", labelKey: "sidebar.favorites", icon: "★" },
  { key: "recent", labelKey: "sidebar.recent", icon: "◷" },
  { key: "most_used", labelKey: "sidebar.mostUsed", icon: "▲" },
];
const input=ref<{mode:"add"|"edit";name:string}|null>(null),remove=ref<{name:string;count:number}|null>(null);function addCategory(){input.value={mode:"add",name:""}}function editCategory(name:string){input.value={mode:"edit",name}}function removeCategory(name:string,count:number){remove.value={name,count}}function saveCategory(name:string){const d=input.value;if(!d)return;void(d.mode==="add"?createCategory(name):renameCategory(d.name,name));input.value=null}
</script>

<template>
  <nav class="sidebar">
    <div class="brand"><span class="logo">N1</span><span>N1_Recipe</span></div>

    <div class="section">
      <div class="section-title">{{ t("sidebar.library") }}</div>
      <button
        v-for="v in views"
        :key="v.key"
        class="side-item"
        :class="{ active: store.view === v.key }"
        @click="setView(v.key)"
      >
        <span class="icon">{{ v.icon }}</span>
        <span class="label">{{ t(v.labelKey) }}</span>
      </button>
    </div>

    <div class="section grow">
      <div class="section-title category-head"><span>{{ t("sidebar.categories") }}</span><button title="新建分类" @click="addCategory">＋</button></div>
      <button class="side-item" :class="{active:store.view==='category'&&store.category===''}" @click="setView('category','')"><span class="icon">○</span><span class="label">未分类</span></button>
      <div
        v-for="c in store.categories"
        :key="c.name"
        class="side-item category-item"
        :class="{ active: store.view === 'category' && store.category === c.name }"
        @click="setView('category', c.name)"
      >
        <span class="icon">▸</span>
        <span class="label">{{ c.name }}</span>
        <span class="count">{{ c.count }}</span>
        <button class="mini" title="重命名" @click.stop="editCategory(c.name)">✎</button><button class="mini danger" title="删除" @click.stop="removeCategory(c.name,c.count)">×</button>
      </div>
      <div v-if="store.categories.length === 0" class="empty-cats">
        {{ t("sidebar.noCategories") }}
      </div>
    </div>

    <div class="footer">
      <button class="scratch-btn" :class="{active:store.view==='scratch'}" @click="setView('scratch')">临时区</button>
      <div class="footer-row">
        <button class="new-btn grow" @click="openCreate">
          {{ t("sidebar.newSnippet") }}
        </button>
        <button
          class="gear-btn"
          :class="{ active: store.settingsOpen }"
          :title="t('sidebar.settings')"
          @click="toggleSettings"
        >
          ⚙
        </button>
      </div>
    </div>
  </nav><InputDialog v-if="input" :title="input.mode==='add'?'新建分类':'重命名分类'" :initial="input.name" @confirm="saveCategory" @cancel="input=null"/><ConfirmDialog v-if="remove" title="删除分类" danger confirm-text="删除" :message="remove.count?`“${remove.name}”中有 ${remove.count} 条内容，删除后将移入“未分类”。`:`确认删除“${remove.name}”？`" @confirm="deleteCategory(remove!.name);remove=null" @cancel="remove=null"/>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
  overflow: hidden;
  user-select: none;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 14px 10px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-bright);
  letter-spacing: 0.2px;
}

.logo {
  color: var(--accent);
  font-size: 17px;
}

.section {
  padding: 6px 8px;
}

.section.grow {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.section-title {
  padding: 6px 8px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.8px;
  text-transform: uppercase;
  color: var(--text-faint);
}
.category-head{display:flex;align-items:center;justify-content:space-between}.category-head button{border:0;background:transparent;color:var(--accent);font-size:16px;cursor:pointer}
.category-item{cursor:pointer}.mini{border:0;background:transparent;color:var(--text-dim);cursor:pointer;padding:2px}.mini.danger:hover{color:var(--danger)}
.scratch-btn{width:100%;margin-bottom:8px;padding:7px;border:1px solid var(--border);border-radius:6px;background:transparent;color:var(--text);cursor:pointer}.scratch-btn:hover,.scratch-btn.active{background:var(--bg-hover);color:var(--accent)}

.side-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}

.side-item:hover {
  background: var(--bg-hover);
}

.side-item.active {
  background: var(--bg-active);
  color: var(--text-bright);
}

.icon {
  width: 16px;
  text-align: center;
  color: var(--text-dim);
  font-size: 12px;
}

.side-item.active .icon {
  color: var(--accent);
}

.label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.count {
  font-size: 11px;
  color: var(--text-faint);
}

.empty-cats {
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-faint);
}

.footer {
  padding: 10px;
  border-top: 1px solid var(--border);
}

.footer-row {
  display: flex;
  gap: 6px;
}

.new-btn {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--accent);
  border-radius: 5px;
  background: transparent;
  color: var(--accent);
  font-size: 13px;
  cursor: pointer;
}

.new-btn:hover {
  background: var(--bg-active);
}

.gear-btn {
  width: 34px;
  padding: 6px 0;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: transparent;
  color: var(--text-dim);
  font-size: 14px;
  cursor: pointer;
}

.gear-btn:hover,
.gear-btn.active {
  background: var(--bg-hover);
  color: var(--text-bright);
}
</style>
