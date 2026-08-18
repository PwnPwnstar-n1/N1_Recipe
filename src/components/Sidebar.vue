<script setup lang="ts">
import { openCreate, setView, store } from "../store";
import type { ViewName } from "../types";

const views: { key: ViewName; label: string; icon: string }[] = [
  { key: "all", label: "All Snippets", icon: "▤" },
  { key: "favorites", label: "Favorites", icon: "★" },
  { key: "recent", label: "Recent", icon: "◷" },
  { key: "most_used", label: "Most Used", icon: "▲" },
];
</script>

<template>
  <nav class="sidebar">
    <div class="brand"><span class="logo">⌘</span><span>SnipDeck</span></div>

    <div class="section">
      <div class="section-title">Library</div>
      <button
        v-for="v in views"
        :key="v.key"
        class="side-item"
        :class="{ active: store.view === v.key }"
        @click="setView(v.key)"
      >
        <span class="icon">{{ v.icon }}</span>
        <span class="label">{{ v.label }}</span>
      </button>
    </div>

    <div class="section grow">
      <div class="section-title">Categories</div>
      <button
        v-for="c in store.categories"
        :key="c.name"
        class="side-item"
        :class="{ active: store.view === 'category' && store.category === c.name }"
        @click="setView('category', c.name)"
      >
        <span class="icon">▸</span>
        <span class="label">{{ c.name }}</span>
        <span class="count">{{ c.count }}</span>
      </button>
      <div v-if="store.categories.length === 0" class="empty-cats">
        No categories yet
      </div>
    </div>

    <div class="footer">
      <button class="new-btn" @click="openCreate">＋ New Snippet</button>
    </div>
  </nav>
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

.new-btn {
  width: 100%;
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
</style>
