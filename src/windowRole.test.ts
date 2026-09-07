import { describe, expect, it, vi } from "vitest";
import { resolveWindowRole } from "./windowRole";

describe("window role routing", () => {
  it.each(["main", "launcher", "dock"] as const)("routes exact %s label", (label) => {
    expect(resolveWindowRole(label)).toBe(label);
  });

  it("routes an unknown label to the safe main UI, never Dock", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => undefined);
    expect(resolveWindowRole("N1_Recipe")).toBe("main");
    expect(error).toHaveBeenCalledOnce();
    error.mockRestore();
  });
});
