// The commands of the app (apps/desktop/src-tauri/src/commands.rs) with the shapes of their
// answers (crates/atena-app/src/views.rs). Everything is checked again in the backend.

import { invoke } from "@tauri-apps/api/core";

/** An error as the backend sends it: a code the interface translates and a technical detail. */
export interface AppError {
  code: string;
  detail: string;
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "code" in e && "detail" in e;
}

export type Theme = "system" | "light" | "dark";

export interface Settings {
  language: string;
  theme: Theme;
  editor: string;
  shareProposals: boolean;
  concurrency: number;
}

export interface Startup {
  state: "starting" | "ready" | "failed" | "closing";
  error: AppError | null;
  settings: Settings;
  version: string;
}

export interface Overview {
  sources: number;
  sourcesCrawled: number;
  proposals: number;
  documents: number;
  versions: number;
  searchable: number;
  endpoints: number;
  endpointsDue: number;
  lastCheck: string | null;
  checksLastDay: number;
  waitingToShare: number;
  crawling: boolean;
}

export interface Allowed {
  crawl: boolean;
  storeText: boolean;
  showFullText: boolean;
  restrictions: string[];
}

export interface Source {
  id: string;
  name: string;
  url: string;
  country: string;
  authority: string;
  licence: string;
  terms: string;
  register: boolean;
  state: string;
  enabled: boolean;
  documents: number;
  lastCheck: string | null;
  allowed: Allowed;
}

export interface Concept {
  key: string;
  id: string;
  label: string;
  broader: string | null;
}

export interface Scheme {
  id: string;
  label: string;
  concepts: Concept[];
}

export interface Taxonomy {
  draft: boolean;
  schemes: Scheme[];
}

export interface Segment {
  text: string;
  marked: boolean;
}

export interface Hit {
  documentId: number;
  title: string;
  url: string;
  sourceId: string;
  sourceName: string;
  country: string;
  authority: string;
  licence: string;
  language: string | null;
  published: string | null;
  labels: string[];
  snippet: Segment[] | null;
  fullText: boolean;
}

export interface Facet {
  namespace: string;
  value: string;
  count: number;
}

export interface SearchResult {
  total: number;
  page: number;
  pages: number;
  allWords: boolean;
  concepts: string[];
  hits: Hit[];
  facets: Facet[];
}

export interface SearchQuery {
  text: string;
  language: string | null;
  filters: string[];
  page: number;
}

export interface Version {
  number: number;
  fetchedAt: string;
  change: string;
  paragraphsAdded: number;
  paragraphsRemoved: number;
  numbersChanged: boolean;
}

export interface DocumentCard {
  id: number;
  title: string;
  url: string;
  sourceId: string;
  sourceName: string;
  country: string;
  authority: string;
  licence: string;
  language: string | null;
  published: string | null;
  lastChanged: string;
  labels: string[];
  text: string | null;
  versions: Version[];
}

export interface ProposalDetail {
  licence: string;
  terms: string;
  country: string | null;
  rightsPage: string;
  finding: string;
  reasons: string[];
  problem: string | null;
  kind: string | null;
  title: string | null;
  checked: string;
  allowed: Allowed;
}

export interface Proposal {
  id: string;
  name: string;
  url: string;
  state: string;
  shared: boolean;
  country: string;
  licence: string;
  terms: string;
  enabled: boolean;
  proposal: ProposalDetail | null;
}

export interface Proposed {
  line: number;
  url: string;
  id: string | null;
  outcome: string;
  registerId: string | null;
  title: string | null;
  country: string | null;
  licence: string;
  terms: string;
  reasons: string[];
  problem: string | null;
}

export interface Rejected {
  line: number;
  text: string;
  reason: string;
}

export interface ImportResult {
  proposed: Proposed[];
  rejected: Rejected[];
  keptForSharing: number;
}

export interface Received {
  status: "read" | "read-before" | "not-a-list";
  list: string | null;
  proposed: Proposed[];
  rejected: Rejected[];
}

export interface Pending {
  list: string;
  addresses: number;
  entries: number[];
}

export interface Confirmed {
  licence: string;
  terms: string;
  country: string;
  crawl: boolean;
}

export interface Choice {
  licence?: string | null;
  terms?: string | null;
  country?: string | null;
}

export interface DecisionOptions {
  licences: string[];
  terms: string[];
  countries: string[];
}

export interface SyncResult {
  sources: number;
  removed: number;
  merged: number;
  endpointsAdded: number;
  endpointsDisabled: number;
  purged: number;
  restricted: number;
}

export interface Erasure {
  documents: number;
  versions: number;
}

export interface Crawl {
  running: boolean;
  stopping: boolean;
  started: string | null;
  checks: number;
  hosts: number;
  storeErrors: number;
  outcomes: Record<string, number>;
}

export const api = {
  startup: () => invoke<Startup>("startup"),
  overview: () => invoke<Overview>("overview"),
  sources: () => invoke<Source[]>("sources"),
  taxonomy: (language: string) => invoke<Taxonomy>("taxonomy", { language }),
  search: (query: SearchQuery) => invoke<SearchResult>("search", { query }),
  newest: (filters: string[], limit: number) => invoke<Hit[]>("newest", { filters, limit }),
  document: (id: number) => invoke<DocumentCard | null>("document", { id }),
  importAddresses: (text: string, country: string | null) =>
    invoke<ImportResult>("import_addresses", { text, country }),
  proposals: (state: string | null) => invoke<Proposal[]>("proposals", { state }),
  decisionOptions: () => invoke<DecisionOptions>("decision_options"),
  confirm: (id: string, choice: Choice, reason: string) =>
    invoke<Confirmed>("confirm_proposal", { id, choice, reason }),
  reject: (id: string, reason: string) => invoke<void>("reject_proposal", { id, reason }),
  pendingShare: () => invoke<Pending | null>("pending_share"),
  handedOver: (entries: number[]) => invoke<void>("handed_over", { entries }),
  receiveList: (text: string) => invoke<Received>("receive_list", { text }),
  syncSharedRegister: () => invoke<SyncResult>("sync_shared_register"),
  forget: (url: string, prefix: boolean, reason: string) =>
    invoke<Erasure>("forget", { url, prefix, reason }),
  crawlStart: () => invoke<Crawl>("crawl_start"),
  crawlStop: () => invoke<Crawl>("crawl_stop"),
  crawlStatus: () => invoke<Crawl>("crawl_status"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<Settings>("set_settings", { settings }),
  openLink: (url: string) => invoke<void>("open_link", { url }),
};
