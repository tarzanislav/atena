"""Ukladanie dokumentov, verzií a návrhov zaradenia (idempotentne)."""
from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime

from .extract import Extracted
from .taxonomy import Suggestion, suggest
from .textnorm import clean_ws, sha256
from .versioning import compare, text_hash


@dataclass
class StoreResult:
    document_id: int
    version_id: int | None
    change_kind: str          # initial | none | minor | substantive


def make_summary(text: str, limit: int = 420) -> str:
    """Krátka výňatková anotácia (prvé vety) – nie je to preklad ani parafráza AI."""
    t = clean_ws(text.replace("[strana 1]", ""))
    if len(t) <= limit:
        return t
    cut = t[:limit]
    dot = max(cut.rfind(". "), cut.rfind("! "), cut.rfind("? "))
    return (cut[: dot + 1] if dot > 120 else cut.rsplit(" ", 1)[0] + " …").strip()


def save_document(conn, source_id: str, url: str, raw: bytes, ex: Extracted, *,
                  published: datetime | None = None, feed_title: str | None = None,
                  feed_summary: str | None = None, metadata_only: bool = False,
                  etag: str | None = None, last_modified: str | None = None) -> StoreResult:
    title = ex.title or feed_title or url
    body = "" if metadata_only else ex.text
    summary_src = feed_summary if (metadata_only or not ex.text) and feed_summary else ex.text
    summary = make_summary(summary_src or "")
    row = conn.execute("SELECT id, current_version_id FROM document WHERE url=%s", (url,)).fetchone()
    if row is None:
        row = conn.execute(
            """INSERT INTO document(source_id,url,title,language,published_at,rights_mode,etag,last_modified)
               VALUES (%s,%s,%s,%s,%s,%s,%s,%s) RETURNING id, current_version_id""",
            (source_id, url, title, ex.language, published, "metadata_only" if metadata_only else "fulltext_internal",
             etag, last_modified)).fetchone()
    doc_id, cur_vid = row["id"], row["current_version_id"]
    old = None
    if cur_vid:
        old = conn.execute("SELECT version_no, body, title, summary, text_hash FROM document_version WHERE id=%s",
                           (cur_vid,)).fetchone()
    comparable_new = body if not metadata_only else (title + "\n" + summary)
    comparable_old = None
    if old is not None:
        comparable_old = old["body"] if not metadata_only else (old["title"] + "\n" + old["summary"])
    ch = compare(comparable_old, comparable_new)
    if ch.kind == "none" and old is not None and old["title"] == title:
        conn.execute("UPDATE document SET last_seen=now(), status='active', etag=%s, last_modified=%s WHERE id=%s",
                     (etag, last_modified, doc_id))
        return StoreResult(doc_id, None, "none")
    vno = (old["version_no"] + 1) if old else 1
    kind = ch.kind if ch.kind != "none" else "minor"
    v = conn.execute(
        """INSERT INTO document_version(document_id,version_no,content_hash,text_hash,title,summary,body,mime,byte_size,change_kind,change)
           VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s) RETURNING id""",
        (doc_id, vno, sha256(raw), text_hash(comparable_new), title, summary, body, ex.mime, len(raw), kind,
         json.dumps(ch.summary(), ensure_ascii=False))).fetchone()
    conn.execute(
        """UPDATE document SET current_version_id=%s, title=%s, language=coalesce(%s, language),
               published_at=coalesce(published_at, %s), last_seen=now(),
               last_changed=CASE WHEN %s IN ('initial','substantive') THEN now() ELSE last_changed END,
               status='active', etag=%s, last_modified=%s WHERE id=%s""",
        (v["id"], title, ex.language, published, kind, etag, last_modified, doc_id))
    apply_suggestions(conn, doc_id, suggest(title, ex.text or summary, ex.language))
    return StoreResult(doc_id, v["id"], kind)


def apply_suggestions(conn, doc_id: int, suggestions: list[Suggestion]) -> None:
    """Nové návrhy prepíšu staré automatické návrhy; rozhodnutia redaktora sa nikdy neprepisujú."""
    conn.execute("DELETE FROM label WHERE document_id=%s AND origin='auto' AND status='suggested'", (doc_id,))
    for s in suggestions:
        conn.execute(
            """INSERT INTO label(document_id,scheme,concept_id,score,origin,status,evidence)
               VALUES (%s,%s,%s,%s,'auto','suggested',%s) ON CONFLICT DO NOTHING""",
            (doc_id, s.scheme, s.concept_id, s.score, s.evidence))


def mark_gone(conn, url: str) -> None:
    # zánik stránky = kontrola, nie automatické vyhlásenie za zrušený dokument
    conn.execute("UPDATE document SET status='gone' WHERE url=%s", (url,))
