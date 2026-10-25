// Dates, numbers and names in the interface language (Intl); nothing here is a text of its own.

import type { Translator } from "./i18n";

export function date(i18n: Translator, iso: string | null, style: "medium" | "long" = "medium") {
  if (!iso) return "";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : new Intl.DateTimeFormat(i18n.intl, { dateStyle: style }).format(d);
}

export function dateTime(i18n: Translator, iso: string | null) {
  if (!iso) return "";
  const d = new Date(iso);
  return Number.isNaN(d.getTime())
    ? ""
    : new Intl.DateTimeFormat(i18n.intl, { dateStyle: "medium", timeStyle: "short" }).format(d);
}

export function today(i18n: Translator) {
  return new Intl.DateTimeFormat(i18n.intl, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  }).format(new Date());
}

export function number(i18n: Translator, n: number) {
  return new Intl.NumberFormat(i18n.intl).format(n);
}

/** The size of a file: kB or MB in the interface language. */
export function fileSize(i18n: Translator, bytes: number) {
  const mb = bytes / (1024 * 1024);
  const format = new Intl.NumberFormat(i18n.intl, { maximumFractionDigits: 1 });
  return mb >= 1 ? `${format.format(mb)} MB` : `${format.format(Math.max(1, Math.round(bytes / 1024)))} kB`;
}

/** The name of a language code (`sk` → „slovenčina“) in the interface language. */
export function languageOf(i18n: Translator, code: string | null) {
  if (!code) return "";
  try {
    return new Intl.DisplayNames([i18n.intl], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

/** The host of an address, for showing where a link leads. */
export function host(url: string) {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}
