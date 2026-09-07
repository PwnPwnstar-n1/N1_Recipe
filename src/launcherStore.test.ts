import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import {
  cancelVariableDialog,
  confirmVariableDialog,
  executeSelected,
  launcher,
  moveSelection,
  resetLauncher,
  searchSnippets,
} from "./launcherStore";
import type { Snippet } from "./types";

vi.mock("./api", () => ({
  searchSnippets: vi.fn(),
  listVariables: vi.fn(),
  copySnippet: vi.fn(),
  hideLauncher: vi.fn(),
}));

const mockedApi = vi.mocked(api);

function snippet(id: number, title: string): Snippet {
  return {
    id,
    title,
    content: "",
    type: "TEXT",
    category: "",
    language: "",
    tags: [],
    aliases: [],
    favorite: false,
    usageCount: 0,
    lastUsedAt: null,
    createdAt: 1,
    updatedAt: 1,
    kind: "TEXT",
    commands: [],
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  resetLauncher();
  mockedApi.searchSnippets.mockReset();
  mockedApi.listVariables.mockReset();
  mockedApi.copySnippet.mockReset();
  mockedApi.hideLauncher.mockReset();
  mockedApi.searchSnippets.mockResolvedValue([]);
  mockedApi.listVariables.mockResolvedValue([]);
  mockedApi.hideLauncher.mockResolvedValue(undefined);
  mockedApi.copySnippet.mockImplementation(async (id) => {
    const s = snippet(id, "t");
    return { text: "rendered", snippet: { ...s, usageCount: 1, lastUsedAt: 2 } };
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("launcher search state", () => {
  it("query updates results", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "a"), snippet(2, "b")]);
    await searchSnippets("a");
    expect(launcher.results.map((s) => s.id)).toEqual([1, 2]);
    expect(launcher.selectedIndex).toBe(0);
  });

  it("selectedIndex resets to 0 when results change", async () => {
    const many = Array.from({ length: 10 }, (_, i) => snippet(i + 1, `s${i}`));
    mockedApi.searchSnippets.mockResolvedValue(many);
    await searchSnippets("s");
    launcher.selectedIndex = 8;

    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "a"), snippet(2, "b")]);
    await searchSnippets("a");
    expect(launcher.selectedIndex).toBe(0);
  });

  it("moveSelection clamps to valid range", async () => {
    mockedApi.searchSnippets.mockResolvedValue([
      snippet(1, "a"),
      snippet(2, "b"),
      snippet(3, "c"),
    ]);
    await searchSnippets("a");
    launcher.selectedIndex = 2;
    moveSelection(1);
    expect(launcher.selectedIndex).toBe(2);
    moveSelection(-10);
    expect(launcher.selectedIndex).toBe(0);
  });

  it("empty results keep a legal index and Enter does nothing", async () => {
    await searchSnippets("zzz");
    expect(launcher.results).toHaveLength(0);
    expect(launcher.selectedIndex).toBe(0);
    await executeSelected();
    expect(mockedApi.copySnippet).not.toHaveBeenCalled();
    expect(mockedApi.listVariables).not.toHaveBeenCalled();
  });
});

describe("launcher copy flow", () => {
  it("plain snippet copies immediately and hides after feedback", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "greet")]);
    await searchSnippets("g");
    mockedApi.listVariables.mockResolvedValue([]);

    await executeSelected();
    expect(mockedApi.copySnippet).toHaveBeenCalledWith(1, {});
    expect(launcher.copied).toBe(true);
    expect(launcher.variableDialog).toBeNull();

    await vi.advanceTimersByTimeAsync(400);
    expect(mockedApi.hideLauncher).toHaveBeenCalled();
    expect(launcher.copied).toBe(false);
  });

  it("template snippet opens the dialog instead of copying", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "greet")]);
    await searchSnippets("g");
    mockedApi.listVariables.mockResolvedValue(["name"]);

    await executeSelected();
    expect(mockedApi.copySnippet).not.toHaveBeenCalled();
    expect(launcher.variableDialog).toEqual({
      id: 1,
      title: "greet",
      variables: ["name"],
    });
  });

  it("dialog confirm copies with values then hides", async () => {
    launcher.variableDialog = { id: 7, title: "greet", variables: ["name"] };
    await confirmVariableDialog({ name: "N1" });
    expect(mockedApi.copySnippet).toHaveBeenCalledWith(7, { name: "N1" });
    expect(launcher.variableDialog).toBeNull();
    expect(launcher.copied).toBe(true);
    await vi.advanceTimersByTimeAsync(400);
    expect(mockedApi.hideLauncher).toHaveBeenCalled();
  });

  it("dialog cancel copies nothing", () => {
    launcher.variableDialog = { id: 7, title: "greet", variables: ["name"] };
    cancelVariableDialog();
    expect(launcher.variableDialog).toBeNull();
    expect(mockedApi.copySnippet).not.toHaveBeenCalled();
  });

  it("copy failure sets an error and does not hide", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "greet")]);
    await searchSnippets("g");
    mockedApi.listVariables.mockResolvedValue([]);
    mockedApi.copySnippet.mockRejectedValue(new Error("clipboard write failed"));

    await executeSelected();
    expect(launcher.error).toContain("clipboard write failed");
    expect(launcher.copied).toBe(false);
    await vi.advanceTimersByTimeAsync(600);
    expect(mockedApi.hideLauncher).not.toHaveBeenCalled();
  });

  it("dialog copy failure keeps the dialog open for retry", async () => {
    launcher.variableDialog = { id: 7, title: "greet", variables: ["name"] };
    mockedApi.copySnippet.mockRejectedValue(new Error("clipboard write failed"));

    await confirmVariableDialog({ name: "N1" });
    expect(launcher.error).toContain("clipboard write failed");
    expect(launcher.variableDialog).toEqual({
      id: 7,
      title: "greet",
      variables: ["name"],
    });
  });
});
