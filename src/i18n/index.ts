import { createI18n } from "vue-i18n";
import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";

export type AppLocale = "zh-CN" | "en-US";

export const i18n = createI18n({
  legacy: false,
  locale: "en-US",
  fallbackLocale: "en-US",
  messages: {
    "en-US": enUS,
    "zh-CN": zhCN,
  },
});

/// Translation helper usable outside components (store, format utils).
export function t(key: string, params?: Record<string, unknown>): string {
  return i18n.global.t(key, params ?? {});
}

export function applyLocale(locale: AppLocale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}

/// Default language for first launch: Chinese environments get zh-CN,
/// everything else en-US.
export function detectLocale(): AppLocale {
  const language = (navigator.language || "en-US").toLowerCase();
  return language.startsWith("zh") ? "zh-CN" : "en-US";
}

/// Map stable backend error messages to localized UI text.
export function localizeError(err: unknown): string {
  const msg = err instanceof Error ? err.message : String(err);
  if (msg === "title is required") return t("errors.titleRequired");
  const missing = msg.match(/^missing variable values: (.+)$/);
  if (missing) return t("errors.missingVariables", { variables: missing[1] });
  if (msg.startsWith("invalid shortcut")) return t("errors.shortcutInvalid");
  if (msg.startsWith("failed to register shortcut"))
    return t("errors.shortcutRegistrationFailed");
  if (msg.startsWith("failed to bind shortcut handler"))
    return t("errors.shortcutRegistrationFailed");
  return msg;
}
