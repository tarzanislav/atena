import { useEffect, useState } from "react";

import { api, type Hit, type Tally } from "../api";
import { ErrorNotice } from "../components/ErrorNotice";
import { HitItem } from "../components/HitItem";
import {
  ConfirmSaveAll,
  PrivateItemDialog,
  RefusedNote,
  SaveActions,
  SavedList,
  SavingStatus,
} from "../components/PrivateLibrary";
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
          <span className="small">{i18n.t("private-copy-right")}</span>
        </div>
      </div>
    </div>
  );
}

/** "My ATENA": the private overview of the sections drawn out of the overview, and what is saved. */
export function PrivateOverviewScreen() {
  const i18n = useI18n();
  const { taxonomy, navigate, privateStatus, library } = useApp();
  const [tally, setTally] = useState<Tally | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [reading, setReading] = useState<string | null>(null);

  useEffect(() => {
    api.tally(SLOVAK).then(setTally, setError);
  }, []);

  const label = (id: string) =>
    taxonomy.schemes.find((s) => s.id === "section")?.concepts.find((c) => c.id === id)?.label ?? id;
  const count = (id: string) => tally?.sections.find((c) => c.key === `section/${id}`);
  const copies = library.items.filter((i) => i.kind === "copy").length;

  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("private-overview-title")}</h1>
        <p className="muted">{i18n.t("private-overview-of", { name: privateStatus.name ?? "" })}</p>
      </div>
      <PrivateNote />
      <SavingStatus />
      <ErrorNotice error={error} />
      <div className="private-tiles">
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
      <div className="row spread">
        <h2>{i18n.t("private-saved-title")}</h2>
        <span className="small muted">
          {i18n.t("private-saved-counts", { all: library.items.length, copies, references: library.items.length - copies })}
        </span>
      </div>
      <SavedList section={null} onRead={setReading} />
      {reading && <PrivateItemDialog id={reading} onClose={() => setReading(null)} />}
    </div>
  );
}

/** One section drawn out of the overview: its Slovak documents, newest first, to save for oneself;
 * and what is saved from it. */
export function PrivateSectionScreen({ section }: { section: (typeof PRIVATE_SECTIONS)[number] }) {
  const i18n = useI18n();
  const { taxonomy, navigate, library } = useApp();
  const [subsection, setSubsection] = useState<string | null>(null);
  const [view, setView] = useState<"documents" | "saved">("documents");
  const [hits, setHits] = useState<Hit[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmAll, setConfirmAll] = useState(false);
  const [reading, setReading] = useState<string | null>(null);
  const concepts = taxonomy.schemes.find((s) => s.id === "section")?.concepts ?? [];
  const title = concepts.find((c) => c.id === section)?.label ?? section;
  const children = concepts.filter((c) => c.broader === section);
  const filter = `section/${subsection ?? section}`;

  useEffect(() => {
    setHits(null);
    setSelected(new Set());
    api.newest([...SLOVAK, filter], LISTED).then(setHits, setError);
  }, [filter]);

  const unsaved = (hits ?? []).filter((h) => !library.byDocument.has(h.documentId));
  const busy = library.saving?.running ?? false;
  const toggle = (id: number) => {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  };
  const allSelected = unsaved.length > 0 && unsaved.every((h) => selected.has(h.documentId));

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
      <SavingStatus />
      <div className="segmented" role="tablist">
        <button className="segment" role="tab" aria-selected={view === "documents"} onClick={() => setView("documents")}>
          {i18n.t("private-view-documents")}
        </button>
        <button className="segment" role="tab" aria-selected={view === "saved"} onClick={() => setView("saved")}>
          {i18n.t("private-view-saved")}
        </button>
      </div>

      {view === "saved" && <SavedList section={section} onRead={setReading} />}

      {view === "documents" && (
        <>
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
              <div className="row spread list-toolbar">
                <label className="check" style={{ alignItems: "center" }}>
                  <input
                    type="checkbox"
                    checked={allSelected}
                    disabled={unsaved.length === 0}
                    onChange={() => setSelected(allSelected ? new Set() : new Set(unsaved.map((h) => h.documentId)))}
                  />
                  <span className="small">{i18n.t("private-select-all")}</span>
                </label>
                <div className="row" style={{ gap: 8 }}>
                  <button
                    className="button"
                    disabled={busy || selected.size === 0}
                    onClick={() => {
                      library.save([...selected]);
                      setSelected(new Set());
                    }}
                  >
                    <Icon name="save" size={16} />
                    {i18n.t("private-save-selected", { count: selected.size })}
                  </button>
                  <button className="button" disabled={busy || unsaved.length === 0} onClick={() => setConfirmAll(true)}>
                    <Icon name="save" size={16} />
                    {i18n.t("private-save-all", { count: hits.length })}
                  </button>
                  <button className="button link" onClick={() => navigate("search", { filters: [filter] })}>
                    {i18n.t("private-search-all")}
                    <Icon name="arrowRight" size={16} />
                  </button>
                </div>
              </div>
              {hits.map((hit) => {
                const saved = library.byDocument.has(hit.documentId);
                return (
                  <HitItem
                    key={hit.documentId}
                    hit={hit}
                    leading={
                      <input
                        type="checkbox"
                        className="hit-check"
                        aria-label={i18n.t("private-select", { title: hit.title })}
                        checked={selected.has(hit.documentId)}
                        disabled={saved}
                        onChange={() => toggle(hit.documentId)}
                      />
                    }
                    actions={<SaveActions hit={hit} onRead={setReading} />}
                    note={<RefusedNote documentId={hit.documentId} />}
                  />
                );
              })}
            </section>
          )}
        </>
      )}
      {confirmAll && hits && (
        <ConfirmSaveAll
          count={hits.length}
          saved={hits.length - unsaved.length}
          onConfirm={() => library.save(unsaved.map((h) => h.documentId))}
          onClose={() => setConfirmAll(false)}
        />
      )}
      {reading && <PrivateItemDialog id={reading} onClose={() => setReading(null)} />}
    </div>
  );
}
