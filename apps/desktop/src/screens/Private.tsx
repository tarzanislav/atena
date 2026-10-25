import { useEffect, useState } from "react";

import { api, type Hit, type Tally } from "../api";
import { ErrorNotice } from "../components/ErrorNotice";
import { HitItem } from "../components/HitItem";
import { PRIVATE_SECTIONS, useApp } from "../context";
import { number } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { SECTION_COLOURS } from "../sections";

/** The private part works with Slovak data only (the author's decision of 25 Oct 2026). */
const SLOVAK = ["region/sk"];
/** Documents listed in a private section. */
const LISTED = 40;

/** What the private part is, and the right it rests on, said plainly on every private screen. */
function PrivateNote() {
  const i18n = useI18n();
  return (
    <div className="card private-card">
      <div className="row" style={{ alignItems: "flex-start" }}>
        <Icon name="lockClosed" />
        <div className="item-body">
          <strong>{i18n.t("private-note-title")}</strong>
          <span className="small">{i18n.t("private-note-text")}</span>
        </div>
      </div>
    </div>
  );
}

/** "My ATENA": the private overview of the sections drawn out of the overview. */
export function PrivateOverviewScreen() {
  const i18n = useI18n();
  const { taxonomy, navigate, privateStatus } = useApp();
  const [tally, setTally] = useState<Tally | null>(null);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    api.tally(SLOVAK).then(setTally, setError);
  }, []);

  const label = (id: string) =>
    taxonomy.schemes.find((s) => s.id === "section")?.concepts.find((c) => c.id === id)?.label ?? id;
  const count = (id: string) => tally?.sections.find((c) => c.key === `section/${id}`);

  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("private-overview-title")}</h1>
        <p className="muted">{i18n.t("private-overview-of", { name: privateStatus.name ?? "" })}</p>
      </div>
      <PrivateNote />
      <ErrorNotice error={error} />
      <div className="sections">
        {PRIVATE_SECTIONS.map((id) => {
          const c = count(id);
          return (
            <button
              key={id}
              className="card section-tile tile"
              style={{ ["--colour" as string]: SECTION_COLOURS[id] }}
              onClick={() => navigate(`private-${id}`)}
            >
              <span className="section-head">
                <h3>{label(id)}</h3>
                <span className="counts" title={i18n.t("overview-counts-title", { new: c?.new ?? 0, total: c?.total ?? 0, days: tally?.newDays ?? 7 })}>
                  <span className={c?.new ? "counts-new" : undefined}>{number(i18n, c?.new ?? 0)}</span>
                  <span className="muted"> / </span>
                  {number(i18n, c?.total ?? 0)}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

/** One section drawn out of the overview: its Slovak documents, newest first. */
export function PrivateSectionScreen({ section }: { section: (typeof PRIVATE_SECTIONS)[number] }) {
  const i18n = useI18n();
  const { taxonomy, navigate } = useApp();
  const [subsection, setSubsection] = useState<string | null>(null);
  const [hits, setHits] = useState<Hit[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const concepts = taxonomy.schemes.find((s) => s.id === "section")?.concepts ?? [];
  const title = concepts.find((c) => c.id === section)?.label ?? section;
  const children = concepts.filter((c) => c.broader === section);
  const filter = `section/${subsection ?? section}`;

  useEffect(() => {
    setHits(null);
    api.newest([...SLOVAK, filter], LISTED).then(setHits, setError);
  }, [filter]);

  return (
    <div className="page">
      <div className="page-head">
        <p className="small muted">{i18n.t("private-group")}</p>
        <h1 className="row" style={{ gap: 10 }}>
          <span className="tag-dot" style={{ ["--colour" as string]: SECTION_COLOURS[section], width: 14, height: 14 }} />
          {title}
        </h1>
      </div>
      <PrivateNote />
      {children.length > 0 && (
        <div className="row" style={{ gap: 6 }}>
          <button className="chip" aria-pressed={subsection === null} onClick={() => setSubsection(null)}>
            {i18n.t("private-all")}
          </button>
          {children.map((c) => (
            <button key={c.id} className="chip" aria-pressed={subsection === c.id} onClick={() => setSubsection(c.id)}>
              {c.label}
            </button>
          ))}
        </div>
      )}
      <ErrorNotice error={error} />
      {hits === null && !error && <div className="spinner" />}
      {hits && hits.length === 0 && <div className="card empty">{i18n.t("private-empty")}</div>}
      {hits && hits.length > 0 && (
        <section className="card tight">
          <div className="row spread" style={{ padding: "8px 14px 6px" }}>
            <h2>{i18n.t("overview-newest")}</h2>
            <button className="button link" onClick={() => navigate("search", { filters: [filter] })}>
              {i18n.t("private-search-all")}
              <Icon name="arrowRight" size={16} />
            </button>
          </div>
          {hits.map((hit) => (
            <HitItem key={hit.documentId} hit={hit} />
          ))}
        </section>
      )}
    </div>
  );
}
