import { describe, expect, it } from "vitest";
import {
  isValidGeometry,
  planWindowGeometry,
  WINDOW_GEOMETRY_KEYS,
  type MonitorGeometry,
} from "./windowGeometry";

const monitor: MonitorGeometry = {
  position: { x: 0, y: 0 },
  size: { width: 1920, height: 1040 },
  scaleFactor: 1,
};

describe("window geometry isolation and recovery", () => {
  it("uses distinct keys and never defines a frontend Dock key", () => {
    expect(WINDOW_GEOMETRY_KEYS.main).toBe("snipdeck.geometry.main");
    expect(WINDOW_GEOMETRY_KEYS.launcher).toBe("snipdeck.geometry.launcher");
    expect(WINDOW_GEOMETRY_KEYS.main).not.toBe(WINDOW_GEOMETRY_KEYS.launcher);
    expect("dock" in WINDOW_GEOMETRY_KEYS).toBe(false);
  });

  it.each([
    { x: 0, y: 0, width: 0, height: 800 },
    { x: 0, y: 0, width: -1, height: 800 },
    { x: 0, y: 0, width: Number.NaN, height: 800 },
    { x: 0, y: 0, width: 8, height: 1040 },
    { x: 0, y: 0, width: 959, height: 620 },
  ])("recovers an invalid or Dock-sized main geometry: %j", (saved) => {
    expect(isValidGeometry("main", saved)).toBe(false);
    const plan = planWindowGeometry("main", saved, [monitor]);
    expect(plan?.recovered).toBe(true);
    expect(plan?.geometry.width).toBeGreaterThanOrEqual(960);
    expect(plan?.geometry.height).toBeGreaterThanOrEqual(620);
  });

  it("centers an off-screen main on an available work area", () => {
    const plan = planWindowGeometry(
      "main",
      { x: 8000, y: -5000, width: 1200, height: 800 },
      [monitor],
    );
    expect(plan).toEqual({
      geometry: { x: 360, y: 120, width: 1200, height: 800 },
      recovered: true,
      reason: "off-screen",
    });
  });

  it("applies logical minimum/default sizes in physical pixels at 125% DPI", () => {
    const scaled = { ...monitor, size: { width: 2400, height: 1300 }, scaleFactor: 1.25 };
    const plan = planWindowGeometry("main", { x: 0, y: 0, width: 960, height: 620 }, [scaled]);
    expect(plan?.geometry.width).toBe(1500);
    expect(plan?.geometry.height).toBe(1000);
    expect(plan?.recovered).toBe(true);
  });

  it("keeps valid main and launcher geometry unchanged", () => {
    const main = { x: 100, y: 80, width: 1200, height: 800 };
    const launcher = { x: 300, y: 200, width: 720, height: 520 };
    expect(planWindowGeometry("main", main, [monitor])).toEqual({ geometry: main, recovered: false, reason: "valid" });
    expect(planWindowGeometry("launcher", launcher, [monitor])).toEqual({ geometry: launcher, recovered: false, reason: "valid" });
  });

  it("clamps oversized geometry to the monitor work area", () => {
    const plan = planWindowGeometry("main", { x: -100, y: -100, width: 3000, height: 2000 }, [monitor]);
    expect(plan?.geometry).toEqual({ x: 0, y: 0, width: 1920, height: 1040 });
    expect(plan?.recovered).toBe(true);
  });
});
