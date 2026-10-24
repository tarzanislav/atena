// What every screen needs: the settings, the taxonomy in the interface language, navigation and
// the document card.

import { createContext, useContext } from "react";

import type { Settings, Taxonomy } from "./api";

export type Screen = "overview" | "search" | "sources" | "proposals" | "crawl" | "settings";

export interface SearchStart {
  text?: string;
  filters?: string[];
}

export interface AppContextValue {
  settings: Settings;
  saveSettings: (settings: Settings) => Promise<void>;
  taxonomy: Taxonomy;
  /** The label of `scheme/concept` in the interface language. */
  conceptLabel: (key: string) => string;
  /** The broader concept of `scheme/concept`, if any. */
  broaderOf: (key: string) => string | null;
  navigate: (screen: Screen, search?: SearchStart) => void;
  openDocument: (id: number) => void;
  /** Proposals waiting and whether the crawler runs, for the navigation. */
  refreshBadges: () => void;
  version: string;
}

export const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) throw new Error("useApp outside the app");
  return value;
}
