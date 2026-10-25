import { useEffect, useState } from "react";

import { api, type ConceptCount, type Hit, type Overview, type Tally } from "../api";
import { ErrorNotice } from "../components/ErrorNotice";
import { HitItem } from "../components/HitItem";
import { ScopeSwitch, scopeFilters } from "../components/Scope";
import { ToolList, inScope, useTools } from "../components/Tools";
import { useApp } from "../context";
import { dateTime, number, today } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { HIDDEN_SUBSECTIONS, SECTION_COLOURS, SECTION_TILES } from "../sections";

/** Documents shown under "Newest": a glance, the rest is in search. */
const NEWEST = 5;

/** "new / all" of a category, with the meaning in the tooltip. */
function Counts({ count, days }: { count: ConceptCount | undefined; days: number }) {
  const i18n = useI18n();
  const total = count?.total ?? 0;
  const fresh = count?.new ?? 0;
  return (
    <span className="counts" title={i18n.t("overview-counts-title", { new: fresh, total, days })}>
      <span className={fresh > 0 ? "counts-new" : undefined}>{number(i18n, fresh)}</span>
      <span className="muted"> / </span>
      <span>{number(i18n, total)}</span>
    </span>
  );
}

export function OverviewScreen() {
  const i18n = useI18n();
  const { taxonomy, navigate, settings } = useApp();
  const [overview, setOverview] = useState<Overview | null>(null);
  const [tally, setTally] = useState<Tally | null>(null);
  const [newest, setNewest] = useState<Hit[]>([]);
  const [error, setError] = useState<unknown>(null);
  const tools = inScope(useTools(), settings);
  const scope = scopeFilters(settings);
  const scopeKey = scope.join(" ");

  useEffect(() => {
    api.overview().then(setOverview, setError);
  }, []);

  useEffect(() => {
    const filters = scopeKey.split(" ");
    let current = true;
    api.tally(filters).then(
      (t) => current && setTally(t),
      (e) => current && setError(e),
    );
    api.newest(filters, NEWEST).then(
      (n) => current && setNewest(n),
      (e) => current && setError(e),
    );
    return () => {
      current = false;
    };
  }, [scopeKey]);

  const counts = new Map((tally?.sections ?? []).map((c) => [c.key, c]));
  const days = tally?.newDays ?? 7;
  const sections = taxonomy.schemes.find((s) => s.id === "section");
  const tiles = SECTION_TILES.map((id) => {
    const concept = sections?.concepts.find((c) => c.id === id);
    const all = sections?.concepts ?? [];
    // subsections, each followed by its own subsections (indented)
    const children = all
      .filter((c) => c.broader === id && !HIDDEN_SUBSECTIONS.has(c.id))
      .flatMap((c) => [
        { concept: c, nested: false },
        ...all.filter((n) => n.broader === c.id).map((n) => ({ concept: n, nested: true })),
      ]);
    return { id, label: concept?.label ?? id, children };
  });
  const open = (key: string) => navigate("search", { filters: [key] });

  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("overview-title")}</h1>
        <p className="muted">{today(i18n)}</p>
      </div>

      <ScopeSwitch tally={tally} />
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

      {overview && tally && (
        <div className="summary">
          <span>
            <strong>{i18n.t("overview-in-scope", { total: tally.total })}</strong>
            {" · "}
            <span className={tally.new > 0 ? "counts-new" : undefined}>
              {i18n.t("overview-new-in-scope", { new: tally.new, days })}
            </span>
          </span>
          <span className="muted">
            {i18n.t("overview-sources-line", { crawled: overview.sourcesCrawled, sources: overview.sources })}
            {" · "}
            {overview.lastCheck
              ? i18n.t("overview-last-check", { time: dateTime(i18n, overview.lastCheck), checks: overview.checksLastDay })
              : i18n.t("overview-never-checked")}
          </span>
          {overview.proposals > 0 && (
            <button className="button link" onClick={() => navigate("proposals")}>
              {number(i18n, overview.proposals)} {i18n.t("overview-proposals", { count: overview.proposals })}
              <Icon name="arrowRight" size={16} />
            </button>
          )}
        </div>
      )}

      <div className="row spread">
        <h2>{i18n.t("overview-sections")}</h2>
        <span className="small muted">
          {i18n.t("overview-counts-legend", { days })}
          {taxonomy.draft && <span className="draft" style={{ marginLeft: 8 }}>{i18n.t("translation-draft")}</span>}
        </span>
      </div>
      <div className="sections">
        {tiles.map((tile) => {
          const key = `section/${tile.id}`;
          const own = counts.get(key);
          return (
            <section
              key={tile.id}
              className={own?.total ? "card section-tile" : "card section-tile empty-tile"}
              style={{ ["--colour" as string]: SECTION_COLOURS[tile.id] }}
              aria-label={tile.label}
            >
              <button className="section-head" onClick={() => open(key)}>
                <h3>{tile.label}</h3>
                <Counts count={own} days={days} />
              </button>
              {tile.children.length > 0 && (
                <ul className="subsections">
                  {tile.children.map(({ concept: c, nested }) => {
                    const count = counts.get(c.key);
                    const classes = ["subsection", count?.total ? "" : "dim", nested ? "nested" : ""].filter(Boolean);
                    return (
                      <li key={c.key}>
                        <button className={classes.join(" ")} onClick={() => open(c.key)}>
                          <span>{c.label}</span>
                          <Counts count={count} days={days} />
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          );
        })}
      </div>

      <ToolList tools={tools} title={i18n.t("tools-title")} />

      {newest.length > 0 && (
        <section className="card tight" aria-label={i18n.t("overview-newest")}>
          <div className="row spread" style={{ padding: "8px 14px 6px" }}>
            <h2>{i18n.t("overview-newest")}</h2>
            <button className="button link" onClick={() => navigate("search", { browse: true })}>
              {i18n.t("overview-search-all")}
              <Icon name="arrowRight" size={16} />
            </button>
          </div>
          {newest.map((hit) => (
            <HitItem key={hit.documentId} hit={hit} withSnippet={false} />
          ))}
        </section>
      )}
    </div>
  );
}
