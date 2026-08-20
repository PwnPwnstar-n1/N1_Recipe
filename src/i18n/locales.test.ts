import { describe, expect, it } from "vitest";
import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";

function flattenKeys(
  obj: Record<string, unknown>,
  prefix = "",
): string[] {
  const keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      keys.push(...flattenKeys(value as Record<string, unknown>, path));
    } else {
      keys.push(path);
    }
  }
  return keys;
}

describe("i18n locales", () => {
  it("zh-CN and en-US expose exactly the same keys", () => {
    const zh = flattenKeys(zhCN).sort();
    const en = flattenKeys(enUS).sort();
    expect(zh).toEqual(en);
  });

  it("both locales define the launcher and shortcut keys", () => {
    const zh = flattenKeys(zhCN);
    const en = flattenKeys(enUS);
    for (const required of [
      "launcher.placeholder",
      "launcher.empty",
      "launcher.copied",
      "settings.quickLauncher",
      "settings.enableLauncher",
      "settings.shortcut",
      "settings.openLauncher",
      "errors.shortcutInvalid",
      "errors.shortcutRegistrationFailed",
    ]) {
      expect(zh).toContain(required);
      expect(en).toContain(required);
    }
  });
});
