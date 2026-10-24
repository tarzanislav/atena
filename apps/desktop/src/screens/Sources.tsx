import { useEffect, useMemo, useState } from "react";

import { api, type Source, type SyncResult } from "../api";
import { Authority, YesNo } from "../components/Badges";
import { Dialog } from "../components/Dialog";
import { ErrorNotice } from "../components/ErrorNotice";
import { useApp } from "../context";
import { date, number } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";

type Show = "all" | "crawled" | "not-crawled" | "imported";

export function SourcesScreen() {
  const i18n = useI18n();
  const { conceptLabel } = useApp();
  const [sources, setSources] = useState<Source[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [show, setShow] = useState<Show>("all");
  const [filter, setFilter] = useState("");
  const [syncing, setSyncing] = useState(false);
  const [asking, setAsking] = useState(false);
  const [synced, setSynced] = useState<SyncResult | null>(null);

  const load = () => api.sources().then(setSources, setError);
  useEffect(() => {
    void load();
  }, []);

  const shown = useMemo(() => {
    const words = filter.trim().toLocaleLowerCase(i18n.intl);
    return (sources ?? []).filter((s) => {
      if (show === "crawled" && !s.enabled) return false;
      if (show === "not-crawled" && s.enabled) return false;
      if (show === "imported" && s.register) return false;
      return (
        !words ||
        `${s.name} ${s.id} ${s.url} ${s.country}`.toLocaleLowerCase(i18n.intl).includes(words)
      );
    });
  }, [sources, show, filter, i18n.intl]);

  const sync = async () => {
    setAsking(false);
    setSyncing(true);
    setError(null);
    try {
      setSynced(await api.syncSharedRegister());
      await load();
    } catch (e) {
      setError(e);
    } finally {
      setSyncing(false);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <div className="row spread">
          <h1>{i18n.t("sources-title")}</h1>
          <button className="button" onClick={() => setAsking(true)} disabled={syncing}>
            {syncing ? <div className="spinner" /> : <Icon name="arrowSync" size={16} />}
            {i18n.t("sources-sync")}
          </button>
        </div>
        <p className="muted">{i18n.t("sources-intro")}</p>
      </div>
      <ErrorNotice error={error} />
      {synced && (
        <div className="notice ok" role="status">
          <Icon name="checkmark" />
          <span>
            {i18n.t("sources-synced", {
              sources: synced.sources,
              added: synced.endpointsAdded,
              purged: synced.purged,
              restricted: synced.restricted,
            })}
          </span>
        </div>
      )}

      <div className="row">
        {(["all", "crawled", "not-crawled", "imported"] as Show[]).map((s) => (
          <button key={s} className="chip" aria-pressed={show === s} onClick={() => setShow(s)}>
            {i18n.t(`sources-show-${s}`)}
          </button>
        ))}
        <input
          className="input"
          type="search"
          placeholder={i18n.t("sources-filter")}
          aria-label={i18n.t("sources-filter")}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          style={{ marginLeft: "auto", minWidth: 240 }}
        />
      </div>

      {sources === null && !error && <div className="spinner" />}
      {sources && (
        <div className="card" style={{ padding: "8px 10px", overflowX: "auto" }}>
          <table className="table">
            <thead>
              <tr>
                <th>{i18n.t("sources-column-source")}</th>
                <th>{i18n.t("sources-column-country")}</th>
                <th>{i18n.t("sources-column-rights")}</th>
                <th title={i18n.t("allowed-crawl")}>{i18n.t("sources-column-crawl")}</th>
                <th title={i18n.t("allowed-text")}>{i18n.t("sources-column-text")}</th>
                <th title={i18n.t("allowed-show")}>{i18n.t("sources-column-show")}</th>
                <th>{i18n.t("sources-column-documents")}</th>
              </tr>
            </thead>
            <tbody>
              {shown.map((s) => (
                <tr key={s.id}>
                  <td>
                    <div className="item-body">
                      <button className="title-button" onClick={() => void api.openLink(s.url)} title={s.url}>
                        <strong>{s.name}</strong>
                      </button>
                      <span className="meta">
                        <Authority value={s.authority} />
                        <span>{s.id}</span>
                        {!s.register && <span className="tag">{i18n.label("state", s.state)}</span>}
                        {s.lastCheck && <span>{i18n.t("sources-checked", { date: date(i18n, s.lastCheck) })}</span>}
                      </span>
                      {s.allowed.restrictions.length > 0 && (
                        <span className="small muted">
                          {s.allowed.restrictions.map((r) => i18n.label("restriction", r)).join(" · ")}
                        </span>
                      )}
                    </div>
                  </td>
                  <td className="country">{conceptLabel(`country/${s.country}`)}</td>
                  <td className="small rights">
                    <div>{i18n.label("licence", s.licence)}</div>
                    <div className="muted">{i18n.label("terms", s.terms)}</div>
                  </td>
                  <td>
                    <YesNo value={s.enabled && s.allowed.crawl} label={i18n.t("allowed-crawl")} />
                  </td>
                  <td>
                    <YesNo value={s.allowed.storeText} label={i18n.t("allowed-text")} />
                  </td>
                  <td>
                    <YesNo value={s.allowed.showFullText} label={i18n.t("allowed-show")} />
                  </td>
                  <td>{number(i18n, s.documents)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {shown.length === 0 && <div className="empty">{i18n.t("sources-none")}</div>}
        </div>
      )}

      {asking && (
        <Dialog title={i18n.t("sources-sync")} onClose={() => setAsking(false)} narrow>
          <p>{i18n.t("sources-sync-explain")}</p>
          <p className="small muted">{i18n.t("sources-sync-privacy")}</p>
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="button" onClick={() => setAsking(false)}>
              {i18n.t("action-cancel")}
            </button>
            <button className="button primary" onClick={() => void sync()}>
              {i18n.t("sources-sync-confirm")}
            </button>
          </div>
        </Dialog>
      )}
    </div>
  );
}
