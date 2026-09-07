import { invoke } from "@tauri-apps/api/core";
import type {
  CategoryInfo,
  CopyResult,
  Snippet,
  SnippetInput,
  ViewName,
  ScratchItem,
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
export const toggleDockPin=(id:number)=>call<Snippet>("toggle_dock_pin",{id});

export function copySnippet(id: number, values: Record<string, string>): Promise<CopyResult> {
  return call<CopyResult>("copy_snippet", { id, values });
}

export function listVariables(id: number): Promise<string[]> {
  return call<string[]>("list_variables", { id });
}

export function listCategories(): Promise<CategoryInfo[]> {
  return call<CategoryInfo[]>("list_categories");
}
export const createCategory=(name:string)=>call<void>("create_category",{name});
export const renameCategory=(oldName:string,newName:string)=>call<void>("rename_category",{oldName,newName});
export const deleteCategory=(name:string)=>call<void>("delete_category",{name});
export const listScratchItems=()=>call<ScratchItem[]>("list_scratch_items");
export const createScratchItem=(content:string)=>call<ScratchItem>("create_scratch_item",{content});
export const updateScratchItem=(id:number,content:string)=>call<void>("update_scratch_item",{id,content});
export const deleteScratchItem=(id:number)=>call<void>("delete_scratch_item",{id});
export const clearScratchItems=()=>call<void>("clear_scratch_items");
export const copyText=(text:string)=>call<void>("copy_text",{text});
export const listTextVariables=(text:string)=>call<string[]>("list_text_variables",{text});
export const copyTemplateText=(text:string,values:Record<string,string>,snippetId?:number)=>call<string>("copy_template_text",{text,values,snippetId:snippetId??null});

export interface Settings {
  language: string | null;
  launcher: {
    enabled: boolean;
    shortcut: string;
  };
  dock: {
    enabled: boolean;
    position: "left" | "right";
    autoHide: boolean;
    alwaysOnTop: boolean;
    collapsedWidth: number;
    expandedWidth: number;
    hideDelay: number;
  };
  startup: {
    launchOnBoot: boolean;
    startHidden: boolean;
  };
}

export function getSettings(): Promise<Settings> {
  return call<Settings>("get_settings");
}

export function setLanguage(language: string): Promise<Settings> {
  return call<Settings>("set_language", { language });
}

export function updateLauncherSettings(launcher: {
  enabled: boolean;
  shortcut: string;
}): Promise<Settings> {
  return call<Settings>("update_launcher_settings", { launcher });
}

export function updateDockSettings(dock: Settings["dock"]): Promise<Settings> {
  return call<Settings>("apply_dock_settings", { dock });
}

export function updateStartupSettings(
  startup: Settings["startup"],
): Promise<Settings> {
  return call<Settings>("update_startup_settings", { startup });
}

export function expandDock(): Promise<void> {
  return call<void>("expand_dock");
}

export function collapseDock(): Promise<void> {
  return call<void>("collapse_dock");
}

export interface ImportSummary {
  added: number;
  skippedDuplicates: number;
  failed: number;
}

export function exportSnippets(path: string): Promise<number> {
  return call<number>("export_snippets", { path });
}

export function importSnippets(path: string): Promise<ImportSummary> {
  return call<ImportSummary>("import_snippets", { path });
}

export function showLauncher(): Promise<void> {
  return call<void>("show_launcher");
}

export function hideLauncher(): Promise<void> {
  return call<void>("hide_launcher");
}
export interface PackPreview{package:{id:string;name:string;version:string;description:string;author:string;homepage:string};categoryCount:number;recipeCount:number;textCount:number;commandGroupCount:number;types:string[];tags:string[];conflicts:number}
export interface PackImportResult{added:number;updated:number;skipped:number;failed:number;categoriesAdded:number;categoriesReused:number}
export const previewRecipePack=(path:string)=>call<PackPreview>("preview_recipe_pack",{path});
export const importRecipePack=(path:string,strategy:string)=>call<PackImportResult>("import_recipe_pack",{path,strategy});
export const exportRecipePack=(path:string,packageMeta:Record<string,unknown>,category:string|null,ids:number[])=>call<number>("export_recipe_pack",{path,package:packageMeta,category,ids});
