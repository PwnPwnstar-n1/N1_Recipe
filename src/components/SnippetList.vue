<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { formatRelative } from "../format";
import { openEdit, select, store } from "../store";

const listEl = ref<HTMLElement | null>(null);

watch(
  () => store.selectedId,
  async (id) => {
    if (id === null) return;
    await nextTick();
    listEl.value
      ?.querySelector(`[data-id="${id}"]`)
      ?.scrollIntoView({ block: "nearest" });
  },
);
</script>

<template>
  <div ref="listEl" class="list">
    <div
      v-for="s in store.items"
      :key="s.id"
      class="item"
      :class="{ active: s.id === store.selectedId }"
      :data-id="s.id"
      @click="select(s.id)"
      @dblclick="openEdit(s.id)"
    >
      <div class="row1">
        <span class="badge" :class="`type-${s.type.toLowerCase()}`">{{ s.type }}</span>
        <span class="title">{{ s.title }}</span>
        <span v-if="s.favorite" class="star">★</span>
      </div>
      <div class="row2">
        <span v-if="s.category" class="meta category">{{ s.category }}</span>
        <span v-for="t in s.tags" :key="t" class="meta chip">{{ t }}</span>
        <span class="spacer"></span>
        <span v-if="s.usageCount > 0" class="meta usage" :title="`used ${s.usageCount} times`">
          ×{{ s.usageCount }}
        </span>
        <span v-if="s.lastUsedAt" class="meta time" :title="`last used ${new Date(s.lastUsedAt).toLocaleString()}`">
          {{ formatRelative(s.lastUsedAt) }}
        </span>
      </div>
    </div>

    <div v-if="!store.loading && store.items.length === 0" class="empty">
      <template v-if="store.query">No matches for “{{ store.query }}”</template>
      <template v-else>No snippets. Ctrl+N to create one.</template>
    </div>
  </div>
</template>

<style scoped>
.list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  min-height: 0;
}

.item {
  padding: 7px 8px;
  border-radius: 5px;
  border-left: 2px solid transparent;
  cursor: pointer;
}

.item:hover {
  background: var(--bg-hover);
}

.item.active {
  background: var(--bg-active);
  border-left-color: var(--accent);
}

.row1 {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
  font-size: 13px;
}

.item.active .title {
  color: var(--text-bright);
}

.star {
  color: var(--star);
  font-size: 12px;
}

.row2 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 3px;
  padding-left: 2px;
  min-width: 0;
  overflow: hidden;
}

.meta {
  font-size: 11px;
  color: var(--text-faint);
  white-space: nowrap;
}

.category {
  color: var(--text-dim);
}

.chip {
  padding: 1px 6px;
  border-radius: 8px;
  background: var(--bg-hover);
}

.spacer {
  flex: 1;
}

.usage {
  color: var(--accent);
}

.empty {
  padding: 40px 10px;
  text-align: center;
  color: var(--text-dim);
  font-size: 13px;
}
</style>
