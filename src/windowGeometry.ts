import {
  availableMonitors,
  getCurrentWindow,
  PhysicalPosition,
  PhysicalSize,
  primaryMonitor,
  type Monitor,
} from "@tauri-apps/api/window";

export type PersistedWindowRole = "main" | "launcher";
export type Geometry = { x: number; y: number; width: number; height: number };
export type MonitorGeometry = {
  position: { x: number; y: number };
  size: { width: number; height: number };
  scaleFactor: number;
};

export const WINDOW_GEOMETRY_KEYS = {
  main: "snipdeck.geometry.main",
  launcher: "snipdeck.geometry.launcher",
} as const;

const ROLE_LIMITS = {
  main: { defaultWidth: 1200, defaultHeight: 800, minWidth: 960, minHeight: 620 },
  launcher: { defaultWidth: 720, defaultHeight: 520, minWidth: 420, minHeight: 260 },
} as const;

const MIN_VISIBLE_WIDTH = 80;
const MIN_VISIBLE_HEIGHT = 40;

function finiteGeometry(value: unknown): value is Geometry {
  if (!value || typeof value !== "object") return false;
  const geometry = value as Record<string, unknown>;
  return [geometry.x, geometry.y, geometry.width, geometry.height].every(
    (part) => typeof part === "number" && Number.isFinite(part),
  );
}

function monitorWorkArea(monitor: MonitorGeometry): Geometry {
  return {
    x: monitor.position.x,
    y: monitor.position.y,
    width: monitor.size.width,
    height: monitor.size.height,
  };
}

function visibleIntersection(geometry: Geometry, area: Geometry): { width: number; height: number } {
  return {
    width: Math.max(0, Math.min(geometry.x + geometry.width, area.x + area.width) - Math.max(geometry.x, area.x)),
    height: Math.max(0, Math.min(geometry.y + geometry.height, area.y + area.height) - Math.max(geometry.y, area.y)),
  };
}

export function isValidGeometry(role: PersistedWindowRole, geometry: unknown): geometry is Geometry {
  if (!finiteGeometry(geometry)) return false;
  const limits = ROLE_LIMITS[role];
  return geometry.width >= limits.minWidth && geometry.height >= limits.minHeight;
}

export type GeometryPlan = { geometry: Geometry; recovered: boolean; reason: string };

/** Validate persisted physical-pixel geometry and return a visible, work-area-clamped plan. */
export function planWindowGeometry(
  role: PersistedWindowRole,
  saved: unknown,
  monitors: MonitorGeometry[],
): GeometryPlan | null {
  if (monitors.length === 0) return null;
  const limits = ROLE_LIMITS[role];
  const source = isValidGeometry(role, saved) ? saved : null;
  const sourceMonitor = source
    ? monitors.find((monitor) => {
        const intersection = visibleIntersection(source, monitorWorkArea(monitor));
        return intersection.width >= MIN_VISIBLE_WIDTH && intersection.height >= MIN_VISIBLE_HEIGHT;
      })
    : undefined;
  const monitor = sourceMonitor ?? monitors[0];
  const work = monitorWorkArea(monitor);
  const scale = Number.isFinite(monitor.scaleFactor) && monitor.scaleFactor > 0 ? monitor.scaleFactor : 1;
  const defaultWidth = Math.round(limits.defaultWidth * scale);
  const defaultHeight = Math.round(limits.defaultHeight * scale);
  const minimumWidth = Math.round(limits.minWidth * scale);
  const minimumHeight = Math.round(limits.minHeight * scale);

  const invalidForMonitor = source
    ? source.width < minimumWidth || source.height < minimumHeight
    : true;
  if (!source || !sourceMonitor || invalidForMonitor) {
    const width = Math.min(Math.max(minimumWidth, defaultWidth), work.width);
    const height = Math.min(Math.max(minimumHeight, defaultHeight), work.height);
    return {
      geometry: {
        x: Math.round(work.x + (work.width - width) / 2),
        y: Math.round(work.y + (work.height - height) / 2),
        width,
        height,
      },
      recovered: true,
      reason: source && !sourceMonitor ? "off-screen" : "invalid-or-missing",
    };
  }

  const width = Math.min(Math.max(source.width, minimumWidth), work.width);
  const height = Math.min(Math.max(source.height, minimumHeight), work.height);
  const x = Math.max(work.x, Math.min(source.x, work.x + work.width - width));
  const y = Math.max(work.y, Math.min(source.y, work.y + work.height - height));
  const geometry = { x: Math.round(x), y: Math.round(y), width: Math.round(width), height: Math.round(height) };
  const recovered = Object.keys(geometry).some((key) => geometry[key as keyof Geometry] !== source[key as keyof Geometry]);
  return { geometry, recovered, reason: recovered ? "clamped-to-work-area" : "valid" };
}

function serializableMonitor(monitor: Monitor): MonitorGeometry {
  return {
    position: { x: monitor.workArea.position.x, y: monitor.workArea.position.y },
    size: { width: monitor.workArea.size.width, height: monitor.workArea.size.height },
    scaleFactor: monitor.scaleFactor,
  };
}

export async function restoreAndTrackWindow(label: string): Promise<() => void> {
  if (label === "dock") return () => {};
  if (label !== "main" && label !== "launcher") {
    console.error("N1_Recipe: geometry tracking rejected unknown window label", label);
    return () => {};
  }

  const role: PersistedWindowRole = label;
  const win = getCurrentWindow();
  const key = WINDOW_GEOMETRY_KEYS[role];
  let raw: string | null = null;
  let saved: unknown = null;

  try {
    raw = localStorage.getItem(key);
    saved = raw ? JSON.parse(raw) : null;
  } catch {
    saved = null;
  }

  try {
    const [available, primary] = await Promise.all([availableMonitors(), primaryMonitor()]);
    const monitors = primary
      ? [primary, ...available.filter((monitor) =>
          monitor.position.x !== primary.position.x || monitor.position.y !== primary.position.y)]
      : available;
    const plan = planWindowGeometry(role, saved, monitors.map(serializableMonitor));
    if (plan) {
      await win.setSize(new PhysicalSize(plan.geometry.width, plan.geometry.height));
      await win.setPosition(new PhysicalPosition(plan.geometry.x, plan.geometry.y));
      if (plan.recovered) localStorage.setItem(key, JSON.stringify(plan.geometry));
      console.info("N1_Recipe window geometry", {
        version: "0.2.1",
        label: role,
        rawGeometry: raw,
        validation: plan.reason,
        finalGeometry: plan.geometry,
        monitorScale: monitors[0]?.scaleFactor ?? null,
        recovered: plan.recovered,
      });
    }
  } catch (error) {
    console.warn("N1_Recipe: window geometry restore failed", { label: role, error: String(error) });
  }

  let timer: number | undefined;
  const save = () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(async () => {
      try {
        const [position, size, minimized, visible] = await Promise.all([
          win.outerPosition(),
          win.innerSize(),
          win.isMinimized(),
          win.isVisible(),
        ]);
        const geometry = { x: position.x, y: position.y, width: size.width, height: size.height };
        if (!minimized && visible && isValidGeometry(role, geometry)) {
          localStorage.setItem(key, JSON.stringify(geometry));
        }
      } catch {
        // A window can disappear while the debounced event is pending.
      }
    }, 250);
  };
  const [removeMoved, removeResized] = await Promise.all([win.onMoved(save), win.onResized(save)]);
  return () => {
    removeMoved();
    removeResized();
    window.clearTimeout(timer);
  };
}
