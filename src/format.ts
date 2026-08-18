import { t } from "./i18n";
import type { AppLocale } from "./i18n";

export function formatRelative(ms: number | null): string {
  if (ms === null) return t("time.never");
  const diff = Date.now() - ms;
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return t("time.justNow");
  if (minutes < 60) return t("time.minutesAgo", { m: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("time.hoursAgo", { h: hours });
  const days = Math.floor(hours / 24);
  if (days < 30) return t("time.daysAgo", { d: days });
  return new Date(ms).toLocaleDateString();
}

export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString(formatDateTimeLocale());
}

function formatDateTimeLocale(): string {
  // match the app language instead of the OS locale
  const locale = (document.documentElement.lang || "en-US") as AppLocale;
  return locale === "zh-CN" ? "zh-CN" : "en-US";
}
