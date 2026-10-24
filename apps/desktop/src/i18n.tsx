// Every text of the interface comes from the Fluent files in ../locales; Slovak is the reference.
// A new language is a new file and a line in locales/index.json – no change of the program.

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import { createContext, useContext, useMemo, type ReactNode } from "react";

import languages from "../locales/index.json";

const files = import.meta.glob("../locales/*.ftl", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

export interface Language {
  code: string;
  /** The tag for dates, numbers and plural rules. */
  intl: string;
  /** Translated by a machine; awaits review by a native speaker. */
  draft: boolean;
}

export const LANGUAGES: Language[] = languages;
export const REFERENCE = "sk";

function language(code: string): Language {
  return LANGUAGES.find((l) => l.code === code) ?? LANGUAGES[0];
}

const bundles = new Map<string, FluentBundle>();

function bundle(code: string): FluentBundle {
  let b = bundles.get(code);
  if (!b) {
    // no isolation marks: the core languages are written left to right
    b = new FluentBundle(language(code).intl, { useIsolating: false });
    const text = files[`../locales/${code}.ftl`] ?? "";
    for (const error of b.addResource(new FluentResource(text))) {
      console.warn(`locales/${code}.ftl: ${error.message}`);
    }
    bundles.set(code, b);
  }
  return b;
}

/** Where a missing text is looked up: Czech readers read Slovak; everyone else reads English. */
function chain(code: string): string[] {
  const order = code === "cs" ? [code, REFERENCE, "en"] : [code, "en", REFERENCE];
  return [...new Set(order)].filter((c) => LANGUAGES.some((l) => l.code === c));
}

export type Args = Record<string, FluentVariable>;

export interface Translator {
  /** The interface language. */
  code: string;
  /** Its tag for Intl. */
  intl: string;
  /** The text of message `id`; the id itself when no language has it. */
  t: (id: string, args?: Args) => string;
  /** Whether a message exists (for codes that may be new). */
  has: (id: string) => boolean;
  /** A code's text (`prefix-code`), or the code itself when there is none. */
  label: (prefix: string, value: string, args?: Args) => string;
}

export function translator(code: string): Translator {
  const order = chain(code).map(bundle);
  const t = (id: string, args?: Args) => {
    for (const b of order) {
      const message = b.getMessage(id);
      if (message?.value) {
        return b.formatPattern(message.value, args);
      }
    }
    return id;
  };
  const has = (id: string) => order.some((b) => b.hasMessage(id));
  return {
    code,
    intl: language(code).intl,
    t,
    has,
    label: (prefix, value, args) => {
      const id = `${prefix}-${value.replaceAll("_", "-")}`;
      return has(id) ? t(id, args) : value;
    },
  };
}

/** The name of a language in that language (from its own file). */
export function languageName(code: string): string {
  const b = bundle(code);
  const m = b.getMessage("language-name");
  return m?.value ? b.formatPattern(m.value) : code;
}

const Context = createContext<Translator>(translator(REFERENCE));

export function I18nProvider({ code, children }: { code: string; children: ReactNode }) {
  const value = useMemo(() => translator(code), [code]);
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useI18n(): Translator {
  return useContext(Context);
}
