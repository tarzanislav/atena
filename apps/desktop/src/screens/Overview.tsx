import { useEffect, useState } from "react";

import { api, type Hit, type Overview } from "../api";
import { ErrorNotice } from "../components/ErrorNotice";
import { HitItem } from "../components/HitItem";
import { useApp } from "../context";
import { dateTime, number, today } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { SECTION_COLOURS, SECTION_TILES } from "../sections";

export function OverviewScreen() {
  const i18n = useI18n();
  const { taxonomy, navigate } = useApp();
  const [overview, setOverview] = useState<Overview | null>(null);
  const [newest, setNewest] = useState<Hit[]>([]);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    api.overview().then(setOverview, setError);
    api.newest([], 12).then(setNewest, setError);
  }, []);

  const sections = taxonomy.schemes.find((s) => s.id === "section");
  const tiles = SECTION_TILES.map((id) => {
    const concept = sections?.concepts.find((c) => c.id === id);
    const children = sections?.concepts.filter((c) => c.broader === id) ?? [];
    return { id, label: concept?.label ?? id, children };
  });

  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("overview-title")}</h1>
        <p className="muted">{today(i18n)}</p>
      </div>
      <ErrorNotice error={error} />

      {overview && overview.documents === 0 && (
        <div className="card peach">
          <div className="row" style={{ alignItems: "flex-start" }}>
            <Icon name="info" />
            <div className="item-body">
              <strong>{i18n.t("overview-empty-title")}</strong>
              <span>{i18n.t("overview-empty-text")}</span>
            </div>
          </div>
          <div>
            <button className="button primary" onClick={() => navigate("crawl")}>
              <Icon name="play" size={16} />
              {i18n.t("overview-empty-action")}
            </button>
          </div>
        </div>
      )}

      {overview && (
        <div className="grid tiles">
          <div className="card stat">
            <span className="stat-value">{number(i18n, overview.documents)}</span>
            <span className="stat-label">{i18n.t("overview-documents", { count: overview.documents })}</span>
            <span className="small muted">{i18n.t("overview-versions", { count: overview.versions })}</span>
          </div>
          <div className="card stat">
            <span className="stat-value">{number(i18n, overview.searchable)}</span>
            <span className="stat-label">{i18n.t("overview-searchable")}</span>
          </div>
          <div className="card stat">
            <span className="stat-value">
              {number(i18n, overview.sourcesCrawled)}
              <span className="muted" style={{ fontSize: 18, fontWeight: 400 }}>
                {" "}
                / {number(i18n, overview.sources)}
              </span>
            </span>
            <span className="stat-label">{i18n.t("overview-sources")}</span>
          </div>
          <button className="card stat tile" onClick={() => navigate("proposals")} style={{ ["--colour" as string]: "#e3a96b" }}>
            <span className="stat-value">{number(i18n, overview.proposals)}</span>
            <span className="stat-label">{i18n.t("overview-proposals", { count: overview.proposals })}</span>
          </button>
        </div>
      )}

      {overview && (
        <p className="small muted">
          {overview.lastCheck
            ? i18n.t("overview-last-check", { time: dateTime(i18n, overview.lastCheck), checks: overview.checksLastDay })
            : i18n.t("overview-never-checked")}
        </p>
      )}

      {newest.length > 0 && (
        <section className="card tight" aria-label={i18n.t("overview-newest")}>
          <div className="row spread" style={{ padding: "8px 14px 6px" }}>
            <h2>{i18n.t("overview-newest")}</h2>
            <button className="button link" onClick={() => navigate("search", {})}>
              {i18n.t("overview-search-all")}
              <Icon name="arrowRight" size={16} />
            </button>
          </div>
          {newest.map((hit) => (
            <HitItem key={hit.documentId} hit={hit} withSnippet={false} />
          ))}
        </section>
      )}

      <div className="row spread">
        <h2>{i18n.t("overview-sections")}</h2>
        {taxonomy.draft && <span className="draft">{i18n.t("translation-draft")}</span>}
      </div>
      <div className="grid sections">
        {tiles.map((tile) => (
          <button
            key={tile.id}
            className="card tile"
            style={{ ["--colour" as string]: SECTION_COLOURS[tile.id] }}
            onClick={() => navigate("search", { filters: [`section/${tile.id}`] })}
          >
            <h3>{tile.label}</h3>
            {tile.children.length > 0 && (
              <span className="row" style={{ gap: 6 }}>
                {tile.children.slice(0, 4).map((c) => (
                  <span key={c.id} className="tag">
                    {c.label}
                  </span>
                ))}
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
