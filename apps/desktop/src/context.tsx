// What every screen needs: the settings, the taxonomy in the interface language, navigation and
// the document card.

import { createContext, useContext } from "react";

import type { PrivateStatus, Settings, Taxonomy } from "./api";
import type { Library } from "./library";

/** The private screens: the private overview and the sections drawn out of the overview. */
export const PRIVATE_SECTIONS = ["changes", "practice", "curriculum", "literacies"] as const;
export type PrivateScreen = "private-overview" | `private-${(typeof PRIVATE_SECTIONS)[number]}`;

export type Screen = "overview" | "search" | "tools" | "sources" | "proposals" | "crawl" | "settings" | PrivateScreen;

export interface SearchStart {
  text?: string;
  filters?: string[];
  /** List everything in the chosen data (Slovak or foreign), without words or filters. */
  browse?: boolean;
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
  /** The private account of this computer. */
  privateStatus: PrivateStatus;
  /** Reads the private account's state again (after creating, opening or closing it). */
  setPrivateStatus: (status: PrivateStatus) => void;
  /** The private library of the open account (empty while it is locked). */
  library: Library;
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
