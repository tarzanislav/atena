import { useEffect, useState } from "react";

import { api, type Hit, type PrivateItem, type PrivateRead } from "../api";
import { useApp } from "../context";
import { date, dateTime, host, number } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { Trust } from "./Badges";
import { Dialog } from "./Dialog";
import { ErrorNotice } from "./ErrorNotice";

/** The size of a kept file: kB or MB in the interface language. */
function size(locale: string, bytes: number) {
  const mb = bytes / (1024 * 1024);
  const format = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  return mb >= 1 ? `${format.format(mb)} MB` : `${format.format(Math.max(1, Math.round(bytes / 1024)))} kB`;
}

/** How an item is kept: a reference to the public part, or a copy for the user's own use. */
export function KindTag({ item }: { item: PrivateItem }) {
  const i18n = useI18n();
  return (
    <span className={`saved-tag ${item.kind}`} title={i18n.t(`private-kind-${item.kind}-title`)}>
      <Icon name={item.kind === "copy" ? "lockClosed" : "link"} size={14} />
      {i18n.t(`private-kind-${item.kind}`)}
    </span>
  );
}

/** Save or read one document of a list, and why it was not saved. */
export function SaveActions({ hit, onRead }: { hit: Hit; onRead: (id: string) => void }) {
  const i18n = useI18n();
  const { library } = useApp();
  const item = library.byDocument.get(hit.documentId);
  if (item) {
    return (
      <>
        <KindTag item={item} />
        <button className="button" onClick={() => onRead(item.id)}>
          <Icon name="documentText" size={16} />
          {i18n.t("private-read")}
        </button>
      </>
    );
  }
  return (
    <button
      className="button private-save"
      disabled={library.saving?.running}
      onClick={() => library.save([hit.documentId])}
    >
      <Icon name="save" size={16} />
      {i18n.t("private-save")}
    </button>
  );
}

/** Why a document of the list was not saved. */
export function RefusedNote({ documentId }: { documentId: number }) {
  const i18n = useI18n();
  const { library } = useApp();
  const reason = library.saving?.reasons.get(documentId);
  if (!reason || library.byDocument.has(documentId)) return null;
  return (
    <p className="small refused-note">
      <Icon name="warning" size={14} />
      {i18n.t(`private-reason-${reason}`)}
    </p>
  );
}

/** The progress of saving a list, then its result. */
export function SavingStatus() {
  const i18n = useI18n();
  const { library } = useApp();
  const s = library.saving;
  if (!s) return null;
  return (
    <div className="card saving-status" role="status">
      <div className="row spread">
        <div className="row" style={{ gap: 10 }}>
          {s.running && <div className="spinner" />}
          <strong>
            {s.running
              ? i18n.t("private-saving", { done: s.done, total: s.total })
              : i18n.t("private-saving-done", { done: s.done, total: s.total })}
          </strong>
        </div>
        {s.running ? (
          <button className="button" onClick={library.stop}>
            {i18n.t("private-saving-stop")}
          </button>
        ) : (
          <button className="button link" onClick={library.dismiss} aria-label={i18n.t("action-close")}>
            <Icon name="dismiss" size={16} />
          </button>
        )}
      </div>
      {s.total > 1 && <progress max={s.total} value={s.done} />}
      <p className="small">
        {i18n.t("private-saving-counts", { saved: s.saved, already: s.already, refused: s.refused })}
      </p>
      <ErrorNotice error={s.error} />
    </div>
  );
}

/** Confirms saving a whole list, with the right it rests on. */
export function ConfirmSaveAll({
  count,
  saved,
  onConfirm,
  onClose,
}: {
  count: number;
  saved: number;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const i18n = useI18n();
  return (
    <Dialog title={i18n.t("private-save-all-title")} onClose={onClose} narrow>
      <p>
        <strong>{i18n.t("private-save-all-text", { count: count - saved })}</strong>
        {saved > 0 && <span className="muted"> {i18n.t("private-save-all-saved", { saved })}</span>}
      </p>
      <p className="small">{i18n.t("private-copy-right")}</p>
      <p className="small muted">{i18n.t("private-save-all-pace")}</p>
      <div className="row" style={{ justifyContent: "flex-end" }}>
        <button className="button" onClick={onClose}>
          {i18n.t("action-cancel")}
        </button>
        <button
          className="button primary"
          autoFocus
          onClick={() => {
            onConfirm();
            onClose();
          }}
        >
          <Icon name="save" size={16} />
          {i18n.t("private-save-all-confirm", { count: count - saved })}
        </button>
      </div>
    </Dialog>
  );
}

/** A saved document read offline, with its origin, licence and kept file. */
export function PrivateItemDialog({ id, onClose }: { id: string; onClose: () => void }) {
  const i18n = useI18n();
  const { library } = useApp();
  const [read, setRead] = useState<PrivateRead | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);

  useEffect(() => {
    api.privateRead(id).then(setRead, setError);
  }, [id]);

  const item = read?.item;
  return (
    <Dialog title={item?.title || i18n.t("document-untitled")} onClose={onClose}>
      <ErrorNotice error={error} />
      {!read && !error && <div className="spinner" />}
      {item && (
        <>
          <div className="meta">
            <Trust value={item.trust} authority={item.authority} />
            <span>{item.sourceName}</span>
            <span aria-hidden="true">·</span>
            <span>{host(item.url)}</span>
            <span aria-hidden="true">·</span>
            <span>{i18n.t("private-saved-on", { date: dateTime(i18n, item.savedAt) })}</span>
          </div>
          <div className="private-card card tight-card">
            <span className="row" style={{ gap: 8 }}>
              <KindTag item={item} />
              <span className="small">{i18n.t(`private-kind-${item.kind}-title`)}</span>
            </span>
            <span className="small muted">{i18n.t("private-licence", { licence: i18n.label("licence", item.licence) })}</span>
          </div>
          <div className="row">
            {item.fileBytes !== null && (
              <button className="button primary" onClick={() => void api.privateOpenFile(item.id).catch(setError)}>
                <Icon name="documentText" size={16} />
                {i18n.t("private-open-file", { size: size(i18n.intl, item.fileBytes) })}
              </button>
            )}
            <button className="button" onClick={() => void api.openLink(item.url)}>
              <Icon name="open" size={16} />
              {i18n.t("action-open-original")}
            </button>
            {!confirmRemove ? (
              <button className="button" onClick={() => setConfirmRemove(true)}>
                <Icon name="erase" size={16} />
                {i18n.t("private-remove")}
              </button>
            ) : (
              <button
                className="button danger"
                onClick={() => {
                  void library.remove([item.id]).then(onClose, setError);
                }}
              >
                {i18n.t("private-remove-confirm")}
              </button>
            )}
          </div>
          {item.fileBytes !== null && <p className="small muted">{i18n.t("private-open-file-note")}</p>}
          <h3>{i18n.t("document-text")}</h3>
          {read.text ? (
            <div className="text-block">{read.text}</div>
          ) : (
            <p className="muted">{i18n.t("private-text-gone")}</p>
          )}
        </>
      )}
    </Dialog>
  );
}

/** The saved documents, newest first, narrowed to one private section. */
export function SavedList({ section, onRead }: { section: string | null; onRead: (id: string) => void }) {
  const i18n = useI18n();
  const { library, broaderOf } = useApp();
  // the top section of a label (`section/laws` → `section/changes`)
  const top = (label: string) => {
    let key = label;
    for (let parent = broaderOf(key); parent; parent = broaderOf(key)) key = parent;
    return key;
  };
  const items = section
    ? library.items.filter((item) => item.labels.some((l) => l.startsWith("section/") && top(l) === `section/${section}`))
    : library.items;
  if (items.length === 0) {
    return <div className="card empty">{i18n.t("private-saved-empty")}</div>;
  }
  return (
    <div className="card tight">
      {items.map((item) => (
        <div key={item.id} className="list-item">
          <span className={`bar trust-bar ${item.trust}`} />
          <div className="item-body">
            <button className="title-button item-title" onClick={() => onRead(item.id)}>
              {item.title || i18n.t("document-untitled")}
            </button>
            <div className="meta">
              <Trust value={item.trust} authority={item.authority} />
              <span>{item.sourceName}</span>
              <span aria-hidden="true">·</span>
              <span>{i18n.t("private-saved-on", { date: date(i18n, item.savedAt) })}</span>
              {item.kind === "copy" && (
                <>
                  <span aria-hidden="true">·</span>
                  <span>
                    {item.fileBytes !== null
                      ? size(i18n.intl, item.fileBytes)
                      : i18n.t("private-text-chars", { chars: number(i18n, item.textChars) })}
                  </span>
                </>
              )}
            </div>
          </div>
          <div className="hit-actions">
            <KindTag item={item} />
            <button className="button" onClick={() => onRead(item.id)}>
              <Icon name="documentText" size={16} />
              {i18n.t("private-read")}
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
