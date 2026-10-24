import { useCallback, useEffect, useState, type FormEvent } from "react";

import { api, type Facet, type SearchResult } from "../api";
import { LabelTag } from "../components/Badges";
import { ErrorNotice } from "../components/ErrorNotice";
import { HitItem } from "../components/HitItem";
import { useApp, type SearchStart } from "../context";
import { languageOf, number } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";

/** Facet namespaces offered for narrowing, in this order. */
const FACETS = ["section", "school-type", "resource-type", "topic", "country", "authority", "language", "source"];

export function SearchScreen({ start }: { start: SearchStart }) {
  const i18n = useI18n();
  const { conceptLabel } = useApp();
  const [text, setText] = useState(start.text ?? "");
  const [filters, setFilters] = useState<string[]>(start.filters ?? []);
  const [page, setPage] = useState(1);
  const [asked, setAsked] = useState<{ text: string; filters: string[]; page: number } | null>(null);
  const [result, setResult] = useState<SearchResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [sourceNames, setSourceNames] = useState<Map<string, string>>(new Map());

  useEffect(() => {
    api.sources().then(
      (all) => setSourceNames(new Map(all.map((s) => [s.id, s.name]))),
      () => undefined,
    );
  }, []);

  const run = useCallback(
    async (query: { text: string; filters: string[]; page: number }) => {
      if (!query.text.trim() && query.filters.length === 0) {
        setResult(null);
        setAsked(null);
        return;
      }
      setBusy(true);
      setError(null);
      try {
        // the words may be in any language: concepts are recognised in all of them
        setResult(await api.search({ ...query, language: null }));
        setAsked(query);
      } catch (e) {
        setError(e);
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  // a search started elsewhere (a section tile) runs at once
  useEffect(() => {
    if (start.text || start.filters?.length) void run({ text: start.text ?? "", filters: start.filters ?? [], page: 1 });
  }, [start, run]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setPage(1);
    void run({ text, filters, page: 1 });
  };

  const toggle = (filter: string) => {
    const next = filters.includes(filter) ? filters.filter((f) => f !== filter) : [...filters, filter];
    setFilters(next);
    setPage(1);
    void run({ text, filters: next, page: 1 });
  };

  const go = (to: number) => {
    setPage(to);
    void run({ text: asked?.text ?? text, filters, page: to });
  };

  const facetText = (f: Facet) => {
    if (f.namespace === "language") return languageOf(i18n, f.value);
    if (f.namespace === "source") return sourceNames.get(f.value) ?? f.value;
    return conceptLabel(`${f.namespace}/${f.value}`);
  };
  const filterText = (filter: string) => {
    const [namespace, value] = filter.split("/", 2);
    if (namespace === "language") return languageOf(i18n, value);
    if (namespace === "source") return sourceNames.get(value) ?? value;
    return conceptLabel(filter);
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("search-title")}</h1>
      </div>
      <form className="row" onSubmit={submit} role="search">
        <label className="searchbox">
          <Icon name="search" />
          <span className="visually-hidden">{i18n.t("search-title")}</span>
          <input
            type="search"
            value={text}
            autoFocus
            maxLength={500}
            placeholder={i18n.t("search-placeholder")}
            onChange={(e) => setText(e.target.value)}
          />
        </label>
        <button className="button primary" type="submit" disabled={busy}>
          {i18n.t("search-action")}
        </button>
        {busy && <div className="spinner" />}
      </form>

      {filters.length > 0 && (
        <div className="row" style={{ gap: 6 }}>
          <span className="small muted">{i18n.t("search-filters")}</span>
          {filters.map((f) => (
            <button key={f} className="chip" aria-pressed="true" onClick={() => toggle(f)} title={i18n.t("search-filter-remove")}>
              {filterText(f)}
              <Icon name="dismiss" size={14} />
            </button>
          ))}
        </div>
      )}

      <ErrorNotice error={error} />

      {!result && !error && (
        <div className="card lavender">
          <p>{i18n.t("search-hint")}</p>
          <p className="small muted">{i18n.t("search-hint-syntax")}</p>
        </div>
      )}

      {result && (
        <div className="row" style={{ alignItems: "flex-start", gap: 24, flexWrap: "nowrap" }}>
          <aside className="facets" aria-label={i18n.t("search-narrow")}>
            {FACETS.map((namespace) => {
              const values = result.facets.filter((f) => f.namespace === namespace);
              if (values.length === 0) return null;
              return (
                <div key={namespace}>
                  <h3 style={{ marginBottom: 6 }}>{i18n.label("facet", namespace)}</h3>
                  {values.map((f) => {
                    const filter = `${f.namespace}/${f.value}`;
                    return (
                      <button
                        key={filter}
                        className="facet-value"
                        aria-pressed={filters.includes(filter)}
                        onClick={() => toggle(filter)}
                      >
                        <span>{facetText(f)}</span>
                        <span className="muted">{number(i18n, f.count)}</span>
                      </button>
                    );
                  })}
                </div>
              );
            })}
          </aside>
          <div className="item-body" style={{ gap: 12 }}>
            <p>
              <strong>{i18n.t("search-found", { count: result.total })}</strong>
              {!result.allWords && <span className="muted"> {i18n.t("search-some-words")}</span>}
            </p>
            {result.concepts.length > 0 && (
              <div className="row" style={{ gap: 6 }}>
                <span className="small muted">{i18n.t("search-concepts")}</span>
                {result.concepts.map((c) => (
                  <LabelTag key={c} label={c} text={conceptLabel(c)} />
                ))}
              </div>
            )}
            {result.total === 0 && <div className="card empty">{i18n.t("search-nothing")}</div>}
            {result.hits.length > 0 && (
              <div className="card tight">
                {result.hits.map((hit) => (
                  <HitItem key={hit.documentId} hit={hit} />
                ))}
              </div>
            )}
            {result.pages > 1 && (
              <div className="row" style={{ justifyContent: "center" }}>
                <button className="button" disabled={page <= 1 || busy} onClick={() => go(page - 1)}>
                  <Icon name="arrowLeft" size={16} />
                  {i18n.t("search-previous")}
                </button>
                <span className="muted">{i18n.t("search-page", { page, pages: result.pages })}</span>
                <button className="button" disabled={page >= result.pages || busy} onClick={() => go(page + 1)}>
                  {i18n.t("search-next")}
                  <Icon name="arrowRight" size={16} />
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
