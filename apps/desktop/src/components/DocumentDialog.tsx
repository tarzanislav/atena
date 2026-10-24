import { useEffect, useState } from "react";

import { api, type DocumentCard } from "../api";
import { useApp } from "../context";
import { date, dateTime, languageOf } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { Authority, LabelTag } from "./Badges";
import { Dialog } from "./Dialog";
import { ErrorNotice } from "./ErrorNotice";

/** The card of a document: where it comes from, its versions and, where allowed, its text. */
export function DocumentDialog({ id, onClose }: { id: number; onClose: () => void }) {
  const i18n = useI18n();
  const { conceptLabel } = useApp();
  const [card, setCard] = useState<DocumentCard | null | undefined>(undefined);
  const [error, setError] = useState<unknown>(null);
  useEffect(() => {
    api.document(id).then(setCard, setError);
  }, [id]);
  const title = card?.title || i18n.t("document-untitled");
  return (
    <Dialog title={card === undefined ? i18n.t("document-title") : title} onClose={onClose}>
      <ErrorNotice error={error} />
      {card === undefined && !error && <div className="spinner" />}
      {card === null && <p className="muted">{i18n.t("document-not-shown")}</p>}
      {card && (
        <>
          <div className="meta">
            <Authority value={card.authority} />
            <span>{card.sourceName}</span>
            <span aria-hidden="true">·</span>
            <span>{conceptLabel(`country/${card.country}`)}</span>
            {card.language && (
              <>
                <span aria-hidden="true">·</span>
                <span>{languageOf(i18n, card.language)}</span>
              </>
            )}
            {card.published && (
              <>
                <span aria-hidden="true">·</span>
                <span>{i18n.t("document-published", { date: date(i18n, card.published) })}</span>
              </>
            )}
          </div>
          <div className="row">
            <button className="button primary" onClick={() => void api.openLink(card.url)}>
              <Icon name="open" size={16} />
              {i18n.t("action-open-original")}
            </button>
            <span className="small muted" style={{ overflowWrap: "anywhere" }}>
              {card.url}
            </span>
          </div>
          {card.labels.length > 0 && (
            <div className="row" style={{ gap: 6 }}>
              {card.labels
                .filter((l) => !l.startsWith("country/") && !l.startsWith("authority/"))
                .map((l) => (
                  <LabelTag key={l} label={l} text={conceptLabel(l)} />
                ))}
            </div>
          )}
          <p className="small muted">{i18n.t("document-labels-are-suggestions")}</p>
          <h3>{i18n.t("document-text")}</h3>
          {card.text ? (
            <div className="text-block">{card.text}</div>
          ) : (
            <p className="muted">
              {i18n.t("document-text-not-shown", { licence: i18n.label("licence", card.licence) })}
            </p>
          )}
          <h3>
            <span className="row" style={{ gap: 8 }}>
              <Icon name="history" />
              {i18n.t("document-versions", { count: card.versions.length })}
            </span>
          </h3>
          <table className="table">
            <thead>
              <tr>
                <th>{i18n.t("version-number")}</th>
                <th>{i18n.t("version-captured")}</th>
                <th>{i18n.t("version-change")}</th>
              </tr>
            </thead>
            <tbody>
              {card.versions.map((v) => (
                <tr key={v.number}>
                  <td>{v.number}</td>
                  <td>{dateTime(i18n, v.fetchedAt)}</td>
                  <td>
                    {i18n.label("change", v.change)}
                    {(v.paragraphsAdded > 0 || v.paragraphsRemoved > 0) && (
                      <span className="muted">
                        {" "}
                        {i18n.t("version-paragraphs", { added: v.paragraphsAdded, removed: v.paragraphsRemoved })}
                      </span>
                    )}
                    {v.numbersChanged && <span className="muted"> · {i18n.t("version-numbers-changed")}</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </Dialog>
  );
}
