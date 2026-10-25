import { useCallback, useEffect, useMemo, useState } from "react";

import { api, type PrivateStatus, type Settings, type Startup, type Taxonomy } from "./api";
import { DocumentDialog } from "./components/DocumentDialog";
import { useLibrary } from "./library";
import { ErrorNotice } from "./components/ErrorNotice";
import { AppContext, PRIVATE_SECTIONS, type AppContextValue, type Screen, type SearchStart } from "./context";
import { UnlockDialog } from "./components/PrivateAccount";
import { I18nProvider, useI18n } from "./i18n";
import { Icon, type IconName } from "./icons";
import { CrawlScreen } from "./screens/Crawl";
import { OverviewScreen } from "./screens/Overview";
import { PrivateOverviewScreen, PrivateSectionScreen } from "./screens/Private";
import { ProposalsScreen } from "./screens/Proposals";
import { SearchScreen } from "./screens/Search";
import { SettingsScreen } from "./screens/Settings";
import { SourcesScreen } from "./screens/Sources";
import { ToolsScreen } from "./screens/Tools";

/** Light, dark or as the system is set – applied at once (`null`: someone else decides). */
function useTheme(theme: Settings["theme"] | null) {
  useEffect(() => {
    if (theme === null) return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || (theme === "system" && media.matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);
}

/** Waits for the backend (the database starts with the app), then shows the app. */
export function Root() {
  const [startup, setStartup] = useState<Startup | null>(null);
  useEffect(() => {
    let stopped = false;
    const poll = async () => {
      try {
        const s = await api.startup();
        if (stopped) return;
        setStartup(s);
        if (s.state === "starting") setTimeout(poll, 400);
      } catch {
        if (!stopped) setTimeout(poll, 1000);
      }
    };
    void poll();
    return () => {
      stopped = true;
    };
  }, []);
  const language = startup?.settings.language ?? "sk";
  const ready = startup?.state === "ready";
  // once ready, the workspace applies the settings as they change
  useTheme(ready ? null : (startup?.settings.theme ?? "system"));
  useEffect(() => {
    if (!ready) document.documentElement.lang = language;
  }, [language, ready]);
  return (
    <I18nProvider code={language}>
      {startup?.state === "ready" ? (
        <Ready initial={startup.settings} version={startup.version} />
      ) : (
        <Splash startup={startup} />
      )}
    </I18nProvider>
  );
}

function Splash({ startup }: { startup: Startup | null }) {
  const i18n = useI18n();
  return (
    <div className="splash">
      <div className="brand-tile" aria-hidden="true" style={{ width: 64, height: 64, fontSize: 32, borderRadius: 16 }}>
        A
      </div>
      <h1>ATENA</h1>
      {startup?.state === "failed" ? (
        <div style={{ maxWidth: 620 }} className="item-body">
          <ErrorNotice error={startup.error} />
          <p className="muted">{i18n.t("startup-failed-hint")}</p>
        </div>
      ) : (
        <>
          <div className="row">
            <div className="spinner" />
            <span>{i18n.t("startup-starting")}</span>
          </div>
          <p className="muted small" style={{ maxWidth: 520 }}>
            {i18n.t("startup-first-time")}
          </p>
        </>
      )}
    </div>
  );
}

const NAVIGATION: { screen: Screen; icon: IconName }[] = [
  { screen: "overview", icon: "home" },
  { screen: "search", icon: "search" },
  { screen: "tools", icon: "toolbox" },
  { screen: "sources", icon: "database" },
  { screen: "proposals", icon: "lightbulb" },
  { screen: "crawl", icon: "globeSearch" },
  { screen: "settings", icon: "settings" },
];

function Ready({ initial, version }: { initial: Settings; version: string }) {
  const [settings, setSettings] = useState(initial);
  return (
    <I18nProvider code={settings.language}>
      <Themed settings={settings} />
      <Workspace settings={settings} setSettings={setSettings} version={version} />
    </I18nProvider>
  );
}

function Themed({ settings }: { settings: Settings }) {
  useTheme(settings.theme);
  useEffect(() => {
    document.documentElement.lang = settings.language;
  }, [settings.language]);
  return null;
}

function Workspace({
  settings,
  setSettings,
  version,
}: {
  settings: Settings;
  setSettings: (s: Settings) => void;
  version: string;
}) {
  const i18n = useI18n();
  const [screen, setScreen] = useState<Screen>("overview");
  const [searchStart, setSearchStart] = useState<SearchStart>({});
  const [documentId, setDocumentId] = useState<number | null>(null);
  const [taxonomy, setTaxonomy] = useState<Taxonomy>({ draft: false, schemes: [] });
  const [badges, setBadges] = useState({ proposals: 0, crawling: false });
  const [privateStatus, setPrivateStatus] = useState<PrivateStatus>({
    exists: false,
    name: null,
    unlocked: false,
    remembered: false,
    rememberAvailable: false,
  });
  // where to go once the private account is open, and whether it stays open outside its screens
  const [unlockFor, setUnlockFor] = useState<Screen | null>(null);
  const [rememberPrivate, setRememberPrivate] = useState(false);

  useEffect(() => {
    api.privateStatus().then(setPrivateStatus, () => undefined);
  }, []);

  const library = useLibrary();
  const { refresh: refreshLibrary, clear: clearLibrary } = library;
  useEffect(() => {
    if (privateStatus.unlocked) refreshLibrary();
    else clearLibrary();
  }, [privateStatus.unlocked, refreshLibrary, clearLibrary]);

  useEffect(() => {
    api.taxonomy(settings.language).then(setTaxonomy, () => undefined);
  }, [settings.language]);

  const refreshBadges = useCallback(() => {
    api.overview().then(
      (o) => setBadges({ proposals: o.proposals, crawling: o.crawling }),
      () => undefined,
    );
  }, []);
  useEffect(() => {
    refreshBadges();
    const timer = setInterval(refreshBadges, 15000);
    return () => clearInterval(timer);
  }, [refreshBadges]);

  const concepts = useMemo(() => {
    const map = new Map<string, { label: string; broader: string | null; scheme: string }>();
    for (const scheme of taxonomy.schemes) {
      for (const c of scheme.concepts) {
        map.set(c.key, { label: c.label, broader: c.broader ? `${scheme.id}/${c.broader}` : null, scheme: scheme.id });
      }
    }
    return map;
  }, [taxonomy]);

  const isPrivate = (s: Screen) => s.startsWith("private-");
  /** Moves to a screen; the private screens need the open account, and leaving them closes it
   * unless the user chose to stay signed in (until ATENA closes, or permanently). */
  const go = (next: Screen, search?: SearchStart) => {
    if (isPrivate(next) && !privateStatus.unlocked) {
      if (!privateStatus.exists) {
        setScreen("settings");
      } else {
        setUnlockFor(next);
      }
      return;
    }
    if (!isPrivate(next) && isPrivate(screen) && privateStatus.unlocked && !rememberPrivate && !privateStatus.remembered) {
      api.privateLock().then(setPrivateStatus, () => undefined);
    }
    if (search) setSearchStart({ ...search });
    setScreen(next);
  };

  const conceptLabelOf = (section: string) => concepts.get(`section/${section}`)?.label ?? section;

  const context: AppContextValue = {
    settings,
    saveSettings: async (next) => {
      setSettings(await api.setSettings(next));
    },
    taxonomy,
    conceptLabel: (key) => concepts.get(key)?.label ?? key.split("/").pop() ?? key,
    broaderOf: (key) => concepts.get(key)?.broader ?? null,
    navigate: (next, search) => go(next, search),
    openDocument: setDocumentId,
    refreshBadges,
    library,
    privateStatus,
    setPrivateStatus: (status) => {
      setPrivateStatus(status);
      if (!status.unlocked) {
        setRememberPrivate(false);
        if (isPrivate(screen)) setScreen("settings");
      }
    },
    version,
  };

  return (
    <AppContext.Provider value={context}>
      <div className="shell">
        <nav className="nav" aria-label={i18n.t("nav-label")}>
          <div className="brand">
            <div className="brand-tile" aria-hidden="true">
              A
            </div>
            <div>
              <div className="brand-name">ATENA</div>
              <div className="brand-claim">{i18n.t("app-claim")}</div>
            </div>
          </div>
          {NAVIGATION.map(({ screen: s, icon }) => (
            <button
              key={s}
              className="nav-item"
              aria-current={screen === s ? "page" : undefined}
              onClick={() => {
                if (s === "search") setSearchStart({});
                go(s);
              }}
            >
              <Icon name={icon} />
              <span>{i18n.t(`nav-${s}`)}</span>
              {s === "proposals" && badges.proposals > 0 && <span className="nav-badge">{badges.proposals}</span>}
              {s === "crawl" && badges.crawling && <span className="nav-dot" title={i18n.t("crawl-running")} />}
            </button>
          ))}
          <div className={privateStatus.unlocked ? "nav-private open" : "nav-private"} role="group" aria-label={i18n.t("private-group")}>
            <div className="nav-private-head">
              <Icon name={privateStatus.unlocked ? "lockOpen" : "lockClosed"} size={16} />
              <span>{i18n.t("private-group")}</span>
            </div>
            {(["private-overview", ...PRIVATE_SECTIONS.map((id) => `private-${id}` as const)] as Screen[]).map((s) => (
              <button
                key={s}
                className="nav-item"
                aria-current={screen === s ? "page" : undefined}
                aria-disabled={!privateStatus.unlocked}
                title={privateStatus.exists ? undefined : i18n.t("private-create-first")}
                onClick={() => go(s)}
              >
                <span>{s === "private-overview" ? i18n.t("private-overview-title") : conceptLabelOf(s.slice(8))}</span>
              </button>
            ))}
            {privateStatus.unlocked && (
              <button
                className="nav-item small"
                onClick={() => {
                  api.privateLock().then(context.setPrivateStatus, () => undefined);
                }}
              >
                <Icon name="lockClosed" size={16} />
                <span>{i18n.t("private-lock")}</span>
              </button>
            )}
          </div>
          <div className="nav-foot">{i18n.t("app-version", { version })}</div>
        </nav>
        <main className="main">
          {screen === "overview" && <OverviewScreen />}
          {screen === "search" && <SearchScreen key={JSON.stringify(searchStart)} start={searchStart} />}
          {screen === "tools" && <ToolsScreen />}
          {screen === "sources" && <SourcesScreen />}
          {screen === "proposals" && <ProposalsScreen />}
          {screen === "crawl" && <CrawlScreen />}
          {screen === "settings" && <SettingsScreen />}
          {screen === "private-overview" && privateStatus.unlocked && <PrivateOverviewScreen />}
          {PRIVATE_SECTIONS.map(
            (id) =>
              screen === `private-${id}` &&
              privateStatus.unlocked && <PrivateSectionScreen key={id} section={id} />,
          )}
        </main>
      </div>
      {documentId !== null && <DocumentDialog id={documentId} onClose={() => setDocumentId(null)} />}
      {unlockFor !== null && (
        <UnlockDialog
          onClose={() => setUnlockFor(null)}
          onUnlocked={(status, remember) => {
            setPrivateStatus(status);
            setRememberPrivate(remember);
            setScreen(unlockFor);
            setUnlockFor(null);
          }}
        />
      )}
    </AppContext.Provider>
  );
}
