import { reactive, watch } from "vue";
import * as api from "./api";
import type { Snippet, SnippetInput, ViewName } from "./types";

export type EditorState = { mode: "create" } | { mode: "edit"; id: number };

export interface VariableDialogState {
  id: number;
  title: string;
  variables: string[];
}

export const store = reactive({
  view: "all" as ViewName,
  category: "",
  query: "",
  items: [] as Snippet[],
  categories: [] as { name: string; count: number }[],
  selectedId: null as number | null,
  editor: null as EditorState | null,
  variableDialog: null as VariableDialogState | null,
  loading: false,
  toast: "",
  toastTimer: undefined as number | undefined,
  searchFocusTick: 0,
});

export function selected(): Snippet | null {
  return store.items.find((i) => i.id === store.selectedId) ?? null;
}

export function showToast(msg: string): void {
  store.toast = msg;
  window.clearTimeout(store.toastTimer);
  store.toastTimer = window.setTimeout(() => {
    store.toast = "";
  }, 1800);
}

export function focusSearch(): void {
  store.searchFocusTick += 1;
}

async function loadList(): Promise<void> {
  store.loading = true;
  try {
    const query = store.query.trim();
    if (query) {
      store.items = await api.searchSnippets(query);
    } else if (store.view === "category") {
      store.items = await api.listSnippets(store.view, store.category);
    } else {
      store.items = await api.listSnippets(store.view);
    }

    const exists = store.items.some((i) => i.id === store.selectedId);
    if (!exists) {
      store.selectedId = store.items[0]?.id ?? null;
    }
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
  } finally {
    store.loading = false;
  }
}

async function loadCategories(): Promise<void> {
  try {
    store.categories = await api.listCategories();
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
  }
}

export async function refresh(): Promise<void> {
  await Promise.all([loadList(), loadCategories()]);
}

export async function setView(view: ViewName, category = ""): Promise<void> {
  store.view = view;
  store.category = category;
  store.query = "";
  store.editor = null;
  await Promise.all([loadList(), loadCategories()]);
}

export function select(id: number): void {
  store.selectedId = id;
}

export function moveSelection(delta: number): void {
  if (store.items.length === 0) return;
  const idx = store.items.findIndex((i) => i.id === store.selectedId);
  const base = idx === -1 ? 0 : idx;
  const next = Math.min(Math.max(base + delta, 0), store.items.length - 1);
  store.selectedId = store.items[next].id;
}

export function openCreate(): void {
  store.editor = { mode: "create" };
}

export function openEdit(id: number): void {
  store.editor = { mode: "edit", id };
}

export function closeEditor(): void {
  store.editor = null;
}

export async function toggleFavorite(s: Snippet): Promise<void> {
  try {
    await api.toggleFavorite(s.id);
    await loadList();
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
  }
}

export async function removeSnippet(id: number): Promise<void> {
  try {
    await api.deleteSnippet(id);
    if (store.editor?.mode === "edit" && store.editor.id === id) {
      store.editor = null;
    }
    await Promise.all([loadList(), loadCategories()]);
    showToast("Deleted");
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
  }
}

export async function saveSnippet(input: SnippetInput): Promise<void> {
  try {
    if (store.editor?.mode === "edit") {
      const updated = await api.updateSnippet(store.editor.id, input);
      store.editor = null;
      await loadList();
      store.selectedId = updated.id;
      showToast("Saved");
    } else {
      const created = await api.createSnippet(input);
      store.editor = null;
      await Promise.all([loadList(), loadCategories()]);
      store.selectedId = created.id;
      showToast("Created");
    }
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
  }
}

/// Copy flow entry point for a snippet. Snippets with ordinary template
/// variables open the variable dialog instead of copying immediately; the
/// actual copy (and usage_count bump) happens on dialog confirm via
/// `confirmVariableDialog`. System variables resolve at copy time in the
/// backend.
export async function copySnippet(s: Snippet): Promise<string> {
  try {
    const variables = await api.listVariables(s.id);
    if (variables.length > 0) {
      store.variableDialog = { id: s.id, title: s.title, variables };
      return "";
    }
    return await performCopy(s.id, {});
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
    return "";
  }
}

async function performCopy(id: number, values: Record<string, string>): Promise<string> {
  const res = await api.copySnippet(id, values);
  replaceItem(res.snippet);
  // copied items move to the top of these views — reload to keep order true
  if (store.view === "recent" || store.view === "most_used") {
    void loadList();
  }
  showToast("Copied to clipboard");
  return res.text;
}

export function cancelVariableDialog(): void {
  store.variableDialog = null;
}

/// Confirm the variable dialog: copy with the filled-in values. On failure
/// the dialog stays open so the user can retry; usage_count is untouched
/// because the backend only counts after a successful clipboard write.
export async function confirmVariableDialog(
  values: Record<string, string>,
): Promise<void> {
  const dialog = store.variableDialog;
  if (!dialog) return;
  try {
    await performCopy(dialog.id, values);
    store.variableDialog = null;
  } catch (err) {
    showToast(err instanceof Error ? err.message : String(err));
  }
}

export async function copySelectedOrFirst(): Promise<string | null> {
  const s = selected() ?? store.items[0] ?? null;
  if (!s) return null;
  await copySnippet(s);
  return s.content;
}

function replaceItem(updated: Snippet): void {
  const idx = store.items.findIndex((i) => i.id === updated.id);
  if (idx >= 0) store.items[idx] = updated;
}

// Debounced live search
let searchTimer: number | undefined;
watch(
  () => store.query,
  () => {
    window.clearTimeout(searchTimer);
    searchTimer = window.setTimeout(() => {
      void loadList();
    }, 120);
  },
);
