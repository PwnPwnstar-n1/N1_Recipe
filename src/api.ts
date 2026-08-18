import { invoke } from "@tauri-apps/api/core";
import type {
  CategoryInfo,
  CopyResult,
  Snippet,
  SnippetInput,
  ViewName,
} from "./types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    const msg =
      typeof err === "string"
        ? err
        : err instanceof Error
          ? err.message
          : JSON.stringify(err);
    throw new Error(msg);
  }
}

export function listSnippets(view: ViewName, category?: string): Promise<Snippet[]> {
  return call<Snippet[]>("list_snippets", { view, category: category ?? null });
}

export function searchSnippets(query: string): Promise<Snippet[]> {
  return call<Snippet[]>("search_snippets", { query });
}

export function getSnippet(id: number): Promise<Snippet> {
  return call<Snippet>("get_snippet", { id });
}

export function createSnippet(input: SnippetInput): Promise<Snippet> {
  return call<Snippet>("create_snippet", { input });
}

export function updateSnippet(id: number, input: SnippetInput): Promise<Snippet> {
  return call<Snippet>("update_snippet", { id, input });
}

export function deleteSnippet(id: number): Promise<void> {
  return call<void>("delete_snippet", { id });
}

export function toggleFavorite(id: number): Promise<Snippet> {
  return call<Snippet>("toggle_favorite", { id });
}

export function copySnippet(id: number): Promise<CopyResult> {
  return call<CopyResult>("copy_snippet", { id });
}

export function listCategories(): Promise<CategoryInfo[]> {
  return call<CategoryInfo[]>("list_categories");
}
