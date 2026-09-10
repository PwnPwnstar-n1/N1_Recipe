import type { Snippet } from "./types";

export interface RecipeCategoryGroup {
  category: string;
  label: string;
  recipes: Snippet[];
}

export type SelectionState = "none" | "partial" | "all";

export function groupRecipes(snippets: Snippet[]): RecipeCategoryGroup[] {
  const groups = new Map<string, Snippet[]>();
  for (const snippet of snippets) {
    const list = groups.get(snippet.category) ?? [];
    list.push(snippet);
    groups.set(snippet.category, list);
  }
  return [...groups.entries()]
    .map(([category, recipes]) => ({
      category,
      label: category || "未分类",
      recipes: [...recipes].sort((a, b) => a.title.localeCompare(b.title)),
    }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

export function categorySelectionState(
  recipes: Snippet[],
  selected: ReadonlySet<number>,
): SelectionState {
  const count = recipes.reduce((total, recipe) => total + Number(selected.has(recipe.id)), 0);
  if (count === 0) return "none";
  if (count === recipes.length) return "all";
  return "partial";
}

export function setCategorySelection(
  selected: Set<number>,
  recipes: Snippet[],
  checked: boolean,
): void {
  for (const recipe of recipes) {
    if (checked) selected.add(recipe.id);
    else selected.delete(recipe.id);
  }
}

export function selectedRecipeIds(selected: ReadonlySet<number>): number[] {
  return [...selected].sort((a, b) => a - b);
}
