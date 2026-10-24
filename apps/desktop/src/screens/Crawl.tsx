import { useCallback, useEffect, useState } from "react";

import { api, type Crawl, type Overview } from "../api";
import { ErrorNotice } from "../components/ErrorNotice";
import { useApp } from "../context";
import { dateTime, number } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";

export function CrawlScreen() {
  const i18n = useI18n();
  const { refreshBadges, settings } = useApp();
  const [crawl, setCrawl] = useState<Crawl | null>(null);
  const [overview, setOverview] = useState<Overview | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [c, o] = await Promise.all([api.crawlStatus(), api.overview()]);
      setCrawl(c);
      setOverview(o);
    } catch (e) {
      setError(e);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 2000);
    return () => clearInterval(timer);
  }, [refresh]);

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      setCrawl(await api.crawlStart());
      refreshBadges();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const stop = async () => {
    setBusy(true);
    try {
      setCrawl(await api.crawlStop());
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
      refreshBadges();
    }
  };

  const outcomes = Object.entries(crawl?.outcomes ?? {}).sort((a, b) => b[1] - a[1]);
  const state = !crawl ? "" : crawl.stopping ? "stopping" : crawl.running ? "running" : "stopped";

  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("crawl-title")}</h1>
        <p className="muted">{i18n.t("crawl-intro")}</p>
      </div>
      <ErrorNotice error={error} />
      <div className={crawl?.running ? "card green" : "card lavender"}>
        <div className="row spread">
          <div className="item-body">
            <h2>{state && i18n.t(`crawl-state-${state}`)}</h2>
            {crawl?.started && (
              <span className="small">{i18n.t("crawl-started", { time: dateTime(i18n, crawl.started) })}</span>
            )}
          </div>
          {crawl?.running ? (
            <button className="button" disabled={busy || crawl.stopping} onClick={() => void stop()}>
              {crawl.stopping ? <div className="spinner" /> : <Icon name="stop" size={16} />}
              {i18n.t("crawl-stop")}
            </button>
          ) : (
            <button className="button primary" disabled={busy} onClick={() => void start()}>
              <Icon name="play" size={16} />
              {i18n.t("crawl-start")}
            </button>
          )}
        </div>
        {crawl && crawl.started && (
          <div className="grid tiles">
            <div className="stat">
              <span className="stat-value">{number(i18n, crawl.checks)}</span>
              <span className="stat-label">{i18n.t("crawl-checks")}</span>
            </div>
            <div className="stat">
              <span className="stat-value">{number(i18n, crawl.hosts)}</span>
              <span className="stat-label">{i18n.t("crawl-hosts")}</span>
            </div>
            {overview && (
              <div className="stat">
                <span className="stat-value">{number(i18n, overview.endpointsDue)}</span>
                <span className="stat-label">{i18n.t("crawl-due")}</span>
              </div>
            )}
            {crawl.storeErrors > 0 && (
              <div className="stat">
                <span className="stat-value">{number(i18n, crawl.storeErrors)}</span>
                <span className="stat-label">{i18n.t("crawl-store-errors")}</span>
              </div>
            )}
          </div>
        )}
      </div>

      {outcomes.length > 0 && (
        <div className="card">
          <h3>{i18n.t("crawl-outcomes")}</h3>
          <table className="table">
            <tbody>
              {outcomes.map(([outcome, count]) => (
                <tr key={outcome}>
                  <td>{i18n.label("check", outcome)}</td>
                  <td style={{ textAlign: "right" }}>{number(i18n, count)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="card">
        <h3>
          <span className="row" style={{ gap: 8 }}>
            <Icon name="shield" />
            {i18n.t("crawl-rules-title")}
          </span>
        </h3>
        <ul style={{ margin: 0, paddingLeft: 20, display: "flex", flexDirection: "column", gap: 6 }}>
          {["robots", "pace", "open", "privacy", "safety"].map((rule) => (
            <li key={rule}>{i18n.t(`crawl-rule-${rule}`)}</li>
          ))}
        </ul>
        <p className="small muted">{i18n.t("crawl-concurrency", { count: settings.concurrency })}</p>
      </div>
    </div>
  );
}
