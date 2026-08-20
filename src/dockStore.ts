import { reactive, watch } from "vue";
import * as api from "./api";
import { localizeError } from "./i18n";
import type { Snippet } from "./types";

export type DockVisualState = "hidden" | "collapsed" | "expanded";

export interface DockVariableDialog {
  id: number;
  title: string;
  variables: string[];
}

/// Dock-window state. Deliberately separate from the main store and the
/// launcher store: the dock owns its content lists (favorites/recent/search
/// results), selection, hover state machine and copy feedback, while
/// reusing api.ts / i18n / TemplateVariableDialog and the shared backend
/// services (search, template engine, clipboard, usage count).
export const dock = reactive({
  query: "",
  favorites: [] as Snippet[],
  recent: [] as Snippet[],
  results: [] as Snippet[],
  selectedIndex: 0,
  state: "hidden" as DockVisualState,
  settings: null as api.Settings["dock"] | null,
  variableDialog: null as DockVariableDialog | null,
  copied: false,
  error: "",
  loading: false,
});

let collapseTimer: number | undefined;
let copiedTimer: number | undefined;
let searchTimer: number | undefined;
/// Generation counter: every transition (enter/leave/collapse) bumps it so
/// a stale collapse timer can never fold a dock that was re-expanded in the
/// meantime.
let generation = 0;

export function setDockState(state: DockVisualState): void {
  dock.state = state;
}

export function setDockSettings(settings: api.Settings["dock"]): void {
  dock.settings = settings;
}

function clampIndex(): void {
  if (dock.results.length === 0) {
    dock.selectedIndex = 0;
    return;
  }
  dock.selectedIndex = Math.min(
    Math.max(dock.selectedIndex, 0),
    dock.results.length - 1,
  );
}

function mergeById(a: Snippet[], b: Snippet[]): Snippet[] {
  const seen = new Set<number>();
  const out: Snippet[] = [];
  for (const snippet of [...a, ...b]) {
    if (!seen.has(snippet.id)) {
      seen.add(snippet.id);
      out.push(snippet);
    }
  }
  return out;
}

/// Load the idle content: favorites + recent (merged, deduplicated).
export async function loadContent(): Promise<void> {
  dock.loading = true;
  try {
    const [favorites, recent] = await Promise.all([
      api.listSnippets("favorites"),
      api.listSnippets("recent"),
    ]);
    dock.favorites = favorites;
    dock.recent = recent;
    dock.results = mergeById(favorites, recent);
    dock.selectedIndex = 0;
    clampIndex();
    dock.error = "";
  } catch (err) {
    dock.results = [];
    dock.selectedIndex = 0;
    dock.error = localizeError(err);
  } finally {
    dock.loading = false;
  }
}

async function runSearch(): Promise<void> {
  dock.loading = true;
  try {
    dock.results = await api.searchSnippets(dock.query);
    dock.selectedIndex = 0;
    clampIndex();
    dock.error = "";
  } catch (err) {
    dock.results = [];
    dock.selectedIndex = 0;
    dock.error = localizeError(err);
  } finally {
    dock.loading = false;
  }
}

// Debounced live search; empty query falls back to favorites+recent.
watch(
  () => dock.query,
  () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      if (dock.query.trim()) {
        void runSearch();
      } else {
        void loadContent();
      }
    }, 120);
  },
);

/// mouseenter: cancel any pending collapse and expand (generation guard).
/// Skips the backend command when the dock is already expanded so rapid
/// enter/leave bursts cannot queue redundant geometry commands.
export function mouseEnterDock(): void {
  generation += 1;
  clearTimeout(collapseTimer);
  const wasExpanded = dock.state === "expanded";
  dock.state = "expanded";
  if (wasExpanded) return;
  void api.expandDock().catch((err) => {
    dock.error = localizeError(err);
  });
}

/// mouseleave: schedule collapse after `hideDelay`; re-entering (or any
/// newer transition) invalidates this timer. An open variable dialog pins
/// the dock open so the user can finish filling it.
export function mouseLeaveDock(): void {
  const gen = ++generation;
  clearTimeout(collapseTimer);
  if (dock.variableDialog) return;
  if (!dock.settings?.autoHide) return;
  collapseTimer = setTimeout(() => {
    if (gen !== generation) return; // stale — a newer transition took over
    dock.state = "collapsed";
    void api.collapseDock().catch((err) => {
      dock.error = localizeError(err);
    });
  }, dock.settings.hideDelay ?? 400);
}

/// Collapse immediately (Esc / copy success). Always cancels pending timers.
export function collapseNow(): void {
  generation += 1;
  clearTimeout(collapseTimer);
  dock.state = "collapsed";
  void api.collapseDock().catch((err) => {
    dock.error = localizeError(err);
  });
}

export function selectedResult(): Snippet | null {
  return dock.results[dock.selectedIndex] ?? null;
}

export function moveSelection(delta: number): void {
  if (dock.results.length === 0) return;
  dock.selectedIndex = Math.min(
    Math.max(dock.selectedIndex + delta, 0),
    dock.results.length - 1,
  );
}

function finishCopy(): void {
  dock.copied = true;
  dock.error = "";
  clearTimeout(copiedTimer);
  copiedTimer = setTimeout(() => {
    dock.copied = false;
    collapseNow(); // copy success -> collapse (default behavior)
  }, 350);
}

async function performCopy(id: number, values: Record<string, string>): Promise<string> {
  const res = await api.copySnippet(id, values);
  return res.text;
}

/// Enter/click-copy on a result: template snippets open the variable dialog;
/// plain and system-variable snippets copy immediately (system variables are
/// rendered by the backend at copy time). usage_count only increments after
/// a successful clipboard write.
export async function executeSelected(): Promise<void> {
  const snippet = selectedResult();
  if (!snippet) return; // Enter with no results does nothing
  try {
    const variables = await api.listVariables(snippet.id);
    if (variables.length > 0) {
      dock.variableDialog = { id: snippet.id, title: snippet.title, variables };
      dock.error = "";
      return;
    }
    await performCopy(snippet.id, {});
    finishCopy();
  } catch (err) {
    dock.error = localizeError(err);
  }
}

export function cancelVariableDialog(): void {
  dock.variableDialog = null;
}

export async function confirmVariableDialog(
  values: Record<string, string>,
): Promise<void> {
  const dialog = dock.variableDialog;
  if (!dialog) return;
  // close immediately so a double Enter cannot copy twice; restore on error
  dock.variableDialog = null;
  try {
    await performCopy(dialog.id, values);
    finishCopy();
  } catch (err) {
    dock.error = localizeError(err);
    dock.variableDialog = dialog;
  }
}

export function resetDock(): void {
  clearTimeout(collapseTimer);
  clearTimeout(copiedTimer);
  clearTimeout(searchTimer);
  generation += 1;
  dock.query = "";
  dock.favorites = [];
  dock.recent = [];
  dock.results = [];
  dock.selectedIndex = 0;
  dock.state = "hidden";
  dock.variableDialog = null;
  dock.copied = false;
  dock.error = "";
  dock.loading = false;
}

/// Full search entry point for tests and programmatic queries.
export async function searchSnippets(query: string): Promise<void> {
  dock.query = query;
  await runSearch();
}
