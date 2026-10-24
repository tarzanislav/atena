import { api, type Hit } from "../api";
import { useApp } from "../context";
import { date, host, languageOf } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { colourOf } from "../sections";
import { Authority, LabelTag } from "./Badges";

/** Labels worth showing on a hit: sections, topics, school types, content types. */
const SHOWN = ["section/", "topic/", "school-type/", "resource-type/", "approach/", "literacy/"];

/** A document found: title, where it comes from, labels and, where allowed, a snippet. */
export function HitItem({ hit, withSnippet = true }: { hit: Hit; withSnippet?: boolean }) {
  const i18n = useI18n();
  const { conceptLabel, broaderOf, openDocument } = useApp();
  const labels = hit.labels.filter((l) => SHOWN.some((p) => l.startsWith(p))).slice(0, 5);
  return (
    <div className="list-item">
      <span className="bar" style={{ ["--colour" as string]: colourOf(hit.labels, broaderOf) }} />
      <div className="item-body">
        <button className="title-button item-title" onClick={() => openDocument(hit.documentId)}>
          {hit.title || i18n.t("document-untitled")}
        </button>
        <div className="meta">
          <Authority value={hit.authority} />
          <span>{hit.sourceName}</span>
          {hit.published && (
            <>
              <span aria-hidden="true">·</span>
              <span>{date(i18n, hit.published)}</span>
            </>
          )}
          {hit.language && (
            <>
              <span aria-hidden="true">·</span>
              <span>{languageOf(i18n, hit.language)}</span>
            </>
          )}
          <span aria-hidden="true">·</span>
          <span>{host(hit.url)}</span>
        </div>
        {withSnippet &&
          (hit.snippet ? (
            <p className="snippet">
              {hit.snippet.map((s, i) => (s.marked ? <mark key={i}>{s.text}</mark> : <span key={i}>{s.text}</span>))}
            </p>
          ) : (
            <p className="small muted">{i18n.t("hit-no-text")}</p>
          ))}
        {labels.length > 0 && (
          <div className="row" style={{ gap: 6 }}>
            {labels.map((l) => (
              <LabelTag key={l} label={l} text={conceptLabel(l)} />
            ))}
          </div>
        )}
      </div>
      <div>
        <button className="button" onClick={() => void api.openLink(hit.url)} title={hit.url}>
          <Icon name="open" size={16} />
          {i18n.t("action-open-original")}
        </button>
      </div>
    </div>
  );
}
