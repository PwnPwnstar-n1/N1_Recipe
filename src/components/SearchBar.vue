<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { copySelectedOrFirst, moveSelection, store } from "../store";

const { t } = useI18n();
const inputEl = ref<HTMLInputElement | null>(null);

onMounted(() => inputEl.value?.focus());

watch(
  () => store.searchFocusTick,
  () => {
    inputEl.value?.focus();
    inputEl.value?.select();
  },
);

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    moveSelection(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    moveSelection(-1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    void copySelectedOrFirst();
  } else if (e.key === "Escape") {
    store.query = "";
  }
}
</script>

<template>
  <div class="searchbar">
    <span class="search-icon">🔍</span>
    <input
      ref="inputEl"
      v-model="store.query"
      class="search-input"
      type="text"
      :placeholder="t('search.placeholder')"
      spellcheck="false"
      @keydown="onKeydown"
    />
    <span class="hint">{{ t("search.enterToCopy") }}</span>
  </div>
</template>

<style scoped>
.searchbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg);
}

.search-icon {
  font-size: 13px;
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

.hint {
  font-size: 11px;
  color: var(--text-faint);
  white-space: nowrap;
}
</style>
