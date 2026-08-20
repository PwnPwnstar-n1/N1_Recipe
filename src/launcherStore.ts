import { reactive, watch } from "vue";
import * as api from "./api";
import { localizeError } from "./i18n";
import type { Snippet } from "./types";

export interface LauncherVariableDialog {
  id: number;
  title: string;
  variables: string[];
}

/// Launcher-window state. Deliberately separate from the main window store:
/// the launcher only owns search results, selection, the variable dialog and
/// copy feedback, while reusing api.ts / i18n / TemplateVariableDialog and
/// the shared backend services.
export const launcher = reactive({
  query: "",
  results: [] as Snippet[],
  selectedIndex: 0,
  loading: false,
  variableDialog: null as LauncherVariableDialog | null,
  copied: false,
  error: "",
});

export function selectedResult(): Snippet | null {
  return launcher.results[launcher.selectedIndex] ?? null;
}

function clampIndex(): void {
  if (launcher.results.length === 0) {
    launcher.selectedIndex = 0;
    return;
  }
  launcher.selectedIndex = Math.min(
    Math.max(launcher.selectedIndex, 0),
    launcher.results.length - 1,
  );
}

export function moveSelection(delta: number): void {
  if (launcher.results.length === 0) return;
  launcher.selectedIndex = Math.min(
    Math.max(launcher.selectedIndex + delta, 0),
    launcher.results.length - 1,
  );
}

async function runSearch(): Promise<void> {
  launcher.loading = true;
  try {
    launcher.results = await api.searchSnippets(launcher.query);
    // results changed: reset to the top and clamp (never out of range)
    launcher.selectedIndex = 0;
    clampIndex();
    launcher.error = "";
  } catch (err) {
    launcher.results = [];
    launcher.selectedIndex = 0;
    launcher.error = localizeError(err);
  } finally {
    launcher.loading = false;
  }
}

// Debounced live search
let searchTimer: number | undefined;
watch(
  () => launcher.query,
  () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void runSearch();
    }, 120);
  },
);

export function resetLauncher(): void {
  launcher.query = "";
  launcher.results = [];
  launcher.selectedIndex = 0;
  launcher.variableDialog = null;
  launcher.copied = false;
  launcher.error = "";
}

export async function hideLauncher(): Promise<void> {
  try {
    await api.hideLauncher();
  } catch {
    // window already gone / command unavailable — nothing to recover
  }
}

let copiedTimer: number | undefined;
function finishCopy(): void {
  launcher.copied = true;
  launcher.error = "";
  clearTimeout(copiedTimer);
  copiedTimer = setTimeout(() => {
    launcher.copied = false;
    void hideLauncher();
  }, 350);
}

async function performCopy(id: number, values: Record<string, string>): Promise<string> {
  const res = await api.copySnippet(id, values);
  return res.text;
}

/// Enter on a result: template snippets open the variable dialog; plain and
/// system-variable snippets copy immediately (backend renders system vars at
/// copy time). usage_count only increments on a successful clipboard write.
export async function executeSelected(): Promise<void> {
  const snippet = selectedResult();
  if (!snippet) return; // Enter with no results does nothing
  try {
    const variables = await api.listVariables(snippet.id);
    if (variables.length > 0) {
      launcher.variableDialog = { id: snippet.id, title: snippet.title, variables };
      launcher.error = "";
      return;
    }
    await performCopy(snippet.id, {});
    finishCopy();
  } catch (err) {
    launcher.error = localizeError(err);
  }
}

export function cancelVariableDialog(): void {
  launcher.variableDialog = null;
}

export async function confirmVariableDialog(
  values: Record<string, string>,
): Promise<void> {
  const dialog = launcher.variableDialog;
  if (!dialog) return;
  // close immediately so a double Enter cannot copy twice; restore on error
  launcher.variableDialog = null;
  try {
    await performCopy(dialog.id, values);
    finishCopy();
  } catch (err) {
    // clipboard/render failure: stay visible, show the error, never count
    launcher.error = localizeError(err);
    launcher.variableDialog = dialog;
  }
}

/// Full search entry point for tests and the "focus search" path.
export async function searchSnippets(query: string): Promise<void> {
  launcher.query = query;
  await runSearch();
}
