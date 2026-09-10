import { describe, expect, it } from "vitest";
import {
  categorySelectionState,
  groupRecipes,
  selectedRecipeIds,
  setCategorySelection,
} from "./recipeExportSelection";
import type { Snippet } from "./types";

function recipe(id: number, category: string, title = `recipe-${id}`): Snippet {
  return {
    id, title, category, content: "", type: "TEXT", language: "", tags: [], aliases: [],
    favorite: false, usageCount: 0, lastUsedAt: null, createdAt: 1, updatedAt: 1,
    kind: "TEXT", commands: [],
  };
}

describe("recipe export selection", () => {
  const pwn = [recipe(3, "Pwn", "ORW"), recipe(1, "Pwn", "ELF")];

  it("groups categories and retains uncategorized recipes", () => {
    const groups = groupRecipes([...pwn, recipe(2, "", "中文标题")]);
    expect(groups.flatMap((group) => group.recipes).map((item) => item.id).sort()).toEqual([1, 2, 3]);
    expect(groups.find((group) => group.category === "")?.label).toBe("未分类");
  });

  it("supports none, all, and indeterminate category states", () => {
    const selected = new Set<number>();
    expect(categorySelectionState(pwn, selected)).toBe("none");
    setCategorySelection(selected, pwn, true);
    expect(categorySelectionState(pwn, selected)).toBe("all");
    selected.delete(1);
    expect(categorySelectionState(pwn, selected)).toBe("partial");
    setCategorySelection(selected, pwn, false);
    expect(categorySelectionState(pwn, selected)).toBe("none");
  });

  it("produces a stable exact ID collection for multi-category selection", () => {
    expect(selectedRecipeIds(new Set([9, 2, 5]))).toEqual([2, 5, 9]);
  });
});
