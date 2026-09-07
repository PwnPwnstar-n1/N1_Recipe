import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import {
  cancelVariableDialog,
  collapseNow,
  confirmVariableDialog,
  dock,
  executeSelected,
  loadContent,
  mouseEnterDock,
  mouseLeaveDock,
  moveSelection,
  resetDock,
  searchSnippets,
  setDockSettings,
  setDockState,
} from "./dockStore";
import type { Snippet } from "./types";

vi.mock("./api", () => ({
  listSnippets: vi.fn(),
  searchSnippets: vi.fn(),
  listVariables: vi.fn(),
  copySnippet: vi.fn(),
  expandDock: vi.fn(),
  collapseDock: vi.fn(),
  getSettings: vi.fn(),
}));

const mockedApi = vi.mocked(api);

function snippet(id: number, title: string, favorite = false): Snippet {
  return {
    id,
    title,
    content: "",
    type: "TEXT",
    category: "",
    language: "",
    tags: [],
    aliases: [],
    favorite,
    usageCount: 0,
    lastUsedAt: null,
    createdAt: 1,
    updatedAt: 1,
    kind: "TEXT",
    commands: [],
  };
}

const defaultDockSettings: api.Settings["dock"] = {
  enabled: true,
  position: "right",
  autoHide: true,
  alwaysOnTop: true,
  collapsedWidth: 8,
  expandedWidth: 360,
  hideDelay: 400,
};

beforeEach(() => {
  vi.useFakeTimers();
  resetDock();
  setDockSettings({ ...defaultDockSettings });
  mockedApi.listSnippets.mockReset();
  mockedApi.searchSnippets.mockReset();
  mockedApi.listVariables.mockReset();
  mockedApi.copySnippet.mockReset();
  mockedApi.expandDock.mockReset();
  mockedApi.collapseDock.mockReset();
  mockedApi.listSnippets.mockResolvedValue([]);
  mockedApi.searchSnippets.mockResolvedValue([]);
  mockedApi.listVariables.mockResolvedValue([]);
  mockedApi.expandDock.mockResolvedValue(undefined);
  mockedApi.collapseDock.mockResolvedValue(undefined);
  mockedApi.copySnippet.mockImplementation(async (id) => {
    const s = snippet(id, "t");
    return { text: "rendered", snippet: { ...s, usageCount: 1, lastUsedAt: 2 } };
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("dock hover state machine", () => {
  it("collapsed -> expanded on mouseenter", async () => {
    setDockState("collapsed");
    mouseEnterDock();
    await vi.advanceTimersByTimeAsync(0);
    expect(dock.state).toBe("expanded");
    expect(mockedApi.expandDock).toHaveBeenCalled();
  });

  it("enter while already expanded does not re-issue the command", async () => {
    setDockState("expanded");
    mouseEnterDock();
    expect(mockedApi.expandDock).not.toHaveBeenCalled();
    // a leave/enter cycle while the collapse timer is pending also skips
    // the redundant expand (the window never actually collapsed)
    mouseLeaveDock();
    mouseEnterDock();
    expect(mockedApi.expandDock).not.toHaveBeenCalled();
  });

  it("expanded -> collapse pending -> collapsed after hideDelay", async () => {
    setDockState("expanded");
    mouseLeaveDock();
    await vi.advanceTimersByTimeAsync(399);
    expect(dock.state).toBe("expanded");
    expect(mockedApi.collapseDock).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(dock.state).toBe("collapsed");
    expect(mockedApi.collapseDock).toHaveBeenCalledTimes(1);
  });

  it("mouse re-enter within the delay cancels the collapse", async () => {
    setDockState("expanded");
    mouseLeaveDock();
    await vi.advanceTimersByTimeAsync(200);
    mouseEnterDock();
    await vi.advanceTimersByTimeAsync(600);
    expect(dock.state).toBe("expanded");
    expect(mockedApi.collapseDock).not.toHaveBeenCalled();
  });

  it("rapid enter/leave does not produce stale collapses", async () => {
    setDockState("expanded");
    mouseLeaveDock(); // timer A
    mouseEnterDock(); // cancels A
    mouseLeaveDock(); // timer B
    await vi.advanceTimersByTimeAsync(400);
    expect(mockedApi.collapseDock).toHaveBeenCalledTimes(1);
    expect(dock.state).toBe("collapsed");
  });

  it("autoHide=false keeps the dock expanded on leave", async () => {
    setDockSettings({ ...defaultDockSettings, autoHide: false });
    setDockState("expanded");
    mouseLeaveDock();
    await vi.advanceTimersByTimeAsync(1000);
    expect(dock.state).toBe("expanded");
    expect(mockedApi.collapseDock).not.toHaveBeenCalled();
  });

  it("open variable dialog pins the dock open on leave", async () => {
    setDockState("expanded");
    dock.variableDialog = { id: 1, title: "t", variables: ["name"] };
    mouseLeaveDock();
    await vi.advanceTimersByTimeAsync(1000);
    expect(dock.state).toBe("expanded");
    expect(mockedApi.collapseDock).not.toHaveBeenCalled();
  });

  it("collapseNow cancels pending timers", async () => {
    setDockState("expanded");
    mouseLeaveDock();
    collapseNow();
    await vi.advanceTimersByTimeAsync(1000);
    expect(mockedApi.collapseDock).toHaveBeenCalledTimes(1);
    expect(dock.state).toBe("collapsed");
  });
});

describe("dock content", () => {
  it("idle content shows only explicitly Dock-pinned recipes", async () => {
    mockedApi.listSnippets.mockImplementation(async (view) => {
      if (view === "favorites") return [snippet(1, "fav", true), snippet(2, "both", true)];
      return [snippet(2, "both"), snippet(3, "recent")];
    });
    await loadContent();
    expect(dock.results.map((s) => s.id)).toEqual([2, 3]);
    expect(dock.selectedIndex).toBe(0);
  });

  it("query switches to backend search and resets the index", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(7, "a"), snippet(8, "b")]);
    await searchSnippets("rtl");
    expect(mockedApi.searchSnippets).toHaveBeenCalledWith("rtl");
    expect(dock.results.map((s) => s.id)).toEqual([7, 8]);
    expect(dock.selectedIndex).toBe(0);
  });

  it("moveSelection clamps to the valid range", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "a"), snippet(2, "b")]);
    await searchSnippets("a");
    moveSelection(5);
    expect(dock.selectedIndex).toBe(1);
    moveSelection(-10);
    expect(dock.selectedIndex).toBe(0);
  });
});

describe("dock copy flow", () => {
  it("plain snippet copies and collapses after feedback", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "greet")]);
    await searchSnippets("g");
    await executeSelected();
    expect(mockedApi.copySnippet).toHaveBeenCalledWith(1, {});
    expect(dock.copied).toBe(true);
    await vi.advanceTimersByTimeAsync(350);
    expect(mockedApi.collapseDock).toHaveBeenCalled();
    expect(dock.copied).toBe(false);
    expect(dock.state).toBe("collapsed");
  });

  it("template snippet opens the dialog instead of copying", async () => {
    mockedApi.searchSnippets.mockResolvedValue([snippet(1, "greet")]);
    await searchSnippets("g");
    mockedApi.listVariables.mockResolvedValue(["name"]);
    await executeSelected();
    expect(mockedApi.copySnippet).not.toHaveBeenCalled();
    expect(dock.variableDialog).toEqual({ id: 1, title: "greet", variables: ["name"] });
  });

  it("dialog confirm copies with values", async () => {
    dock.variableDialog = { id: 7, title: "greet", variables: ["name"] };
    await confirmVariableDialog({ name: "N1" });
    expect(mockedApi.copySnippet).toHaveBeenCalledWith(7, { name: "N1" });
    expect(dock.variableDialog).toBeNull();
    expect(dock.copied).toBe(true);
  });

  it("dialog cancel copies nothing", () => {
    dock.variableDialog = { id: 7, title: "greet", variables: ["name"] };
    cancelVariableDialog();
    expect(dock.variableDialog).toBeNull();
    expect(mockedApi.copySnippet).not.toHaveBeenCalled();
  });

  it("copy failure sets an error, keeps dialog, no collapse", async () => {
    dock.variableDialog = { id: 7, title: "greet", variables: ["name"] };
    mockedApi.copySnippet.mockRejectedValue(new Error("clipboard write failed"));
    await confirmVariableDialog({ name: "N1" });
    expect(dock.error).toContain("clipboard write failed");
    expect(dock.variableDialog).toEqual({ id: 7, title: "greet", variables: ["name"] });
    await vi.advanceTimersByTimeAsync(1000);
    expect(mockedApi.collapseDock).not.toHaveBeenCalled();
  });

  it("enter with no results does nothing", async () => {
    await searchSnippets("zzz");
    await executeSelected();
    expect(mockedApi.copySnippet).not.toHaveBeenCalled();
    expect(mockedApi.listVariables).not.toHaveBeenCalled();
  });
});
