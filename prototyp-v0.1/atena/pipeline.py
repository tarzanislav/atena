"""Zberový proces: kontrola vstupu (kanál / stránka) -> dokumenty -> verzie -> návrh zaradenia.

Poradie krokov je od najlacnejšieho: podmienený GET, odtlačok, až potom extrakcia.
Zlyhanie jedného vstupu nikdy nezastaví ostatné.
"""
from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass, field

from .config import Settings, settings as default_settings
from .crawl.feeds import child_links, discover, looks_like_feed, parse_feed
from .crawl.fetcher import Fetcher, FetchResult
from .extract import extract
from .store import mark_gone, save_document

log = logging.getLogger("atena.pipeline")

EXTRACTABLE = ("text/html", "application/xhtml+xml", "application/pdf", "text/plain",
               "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "")


@dataclass
class RunStats:
    endpoints: int = 0
    fetched: int = 0
    not_modified: int = 0
    new_docs: int = 0
    substantive: int = 0
    minor: int = 0
    unchanged: int = 0
    blocked: int = 0
    errors: int = 0
    messages: list[str] = field(default_factory=list)

    def as_dict(self):
        d = dict(self.__dict__)
        d["messages"] = self.messages[-20:]
        return d


def _log_fetch(conn, endpoint_id, res: FetchResult):
    outcome = "ok" if res.ok else ("not_modified" if res.not_modified else (res.blocked_reason or "error"))
    from .textnorm import sha256
    conn.execute(
        """INSERT INTO fetch_log(endpoint_id,url,final_url,status,outcome,bytes,content_hash,elapsed_ms,error)
           VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s)""",
        (endpoint_id, res.url, res.final_url, res.status, outcome, len(res.content) if res.content else 0,
         sha256(res.content) if res.content else None, res.elapsed_ms, res.error))


async def _process_document(conn, fetcher: Fetcher, source: dict, url: str, stats: RunStats, *,
                            endpoint_id=None, feed_item=None, prefetched: FetchResult | None = None):
    doc = conn.execute("SELECT etag, last_modified FROM document WHERE url=%s", (url,)).fetchone()
    res = prefetched or await fetcher.get(url, etag=doc["etag"] if doc else None,
                                          last_modified=doc["last_modified"] if doc else None)
    if prefetched is None:
        _log_fetch(conn, endpoint_id, res)
    if res.not_modified:
        stats.not_modified += 1
        conn.execute("UPDATE document SET last_seen=now() WHERE url=%s", (url,))
        return
    if res.blocked_reason:
        stats.blocked += 1
        stats.messages.append(f"{url}: {res.error}")
        return
    if res.status in (404, 410):
        mark_gone(conn, url)
        stats.errors += 1
        return
    if not res.ok:
        stats.errors += 1
        stats.messages.append(f"{url}: {res.error}")
        return
    if res.content_type not in EXTRACTABLE and not res.content_type.startswith("text/"):
        stats.messages.append(f"{url}: preskočený typ {res.content_type}")
        return
    stats.fetched += 1
    try:
        ex = extract(res.content, res.content_type, res.final_url)
    except Exception as e:   # poškodený súbor => karanténa (záznam chyby), nie pád
        stats.errors += 1
        stats.messages.append(f"{url}: extrakcia zlyhala – {type(e).__name__}: {e}")
        return
    # výhrada TDM alebo noindex => ukladať len metadáta a odkaz
    metadata_only = res.tdm_reserved or res.noindex or not source.get("store_fulltext", True)
    sr = save_document(conn, source["id"], url, res.content, ex,
                       published=feed_item.published if feed_item else None,
                       feed_title=feed_item.title if feed_item else None,
                       feed_summary=feed_item.summary if feed_item else None,
                       metadata_only=metadata_only, etag=res.etag, last_modified=res.last_modified)
    if sr.change_kind == "initial":
        stats.new_docs += 1
    elif sr.change_kind == "substantive":
        stats.substantive += 1
    elif sr.change_kind == "minor":
        stats.minor += 1
    else:
        stats.unchanged += 1


async def check_endpoint(conn, fetcher: Fetcher, ep: dict, cfg: Settings, stats: RunStats):
    source = conn.execute("SELECT * FROM source WHERE id=%s", (ep["source_id"],)).fetchone()
    if not source or not source["enabled"]:
        return
    stats.endpoints += 1
    res = await fetcher.get(ep["url"], etag=ep["etag"], last_modified=ep["last_modified"])
    _log_fetch(conn, ep["id"], res)
    ok = res.ok or res.not_modified
    conn.execute(
        """UPDATE endpoint SET last_checked=now(), last_status=%s, last_error=%s,
             etag=coalesce(%s, etag), last_modified=coalesce(%s, last_modified),
             fail_count=CASE WHEN %s THEN 0 ELSE fail_count+1 END,
             next_check=now() + make_interval(secs => CASE WHEN %s THEN interval_s
                                                     ELSE least(interval_s, 3600 * power(2, least(fail_count, 6))) END)
           WHERE id=%s""",
        (res.status, res.error, res.etag, res.last_modified, ok, ok, ep["id"]))
    if res.not_modified:
        stats.not_modified += 1
        return
    if not res.ok:
        stats.blocked += 1 if res.blocked_reason else 0
        stats.errors += 0 if res.blocked_reason else 1
        stats.messages.append(f"{ep['url']}: {res.error}")
        return
    if ep["kind"] == "rss" or looks_like_feed(res.content, res.content_type):
        _title, items = parse_feed(res.content, res.final_url)
        for it in items[: cfg.max_items_per_feed]:
            await _process_document(conn, fetcher, source, it.url, stats, endpoint_id=ep["id"], feed_item=it)
            conn.commit()
        return
    # stránka: sama je sledovaným dokumentom + podstránky v tej istej sekcii
    await _process_document(conn, fetcher, source, ep["url"], stats, endpoint_id=ep["id"], prefetched=res)
    conn.commit()
    if res.content_type in ("text/html", "application/xhtml+xml") and cfg.max_child_links > 0:
        page = discover(res.content, res.final_url)
        for f in page.feeds[:1]:
            # objavený kanál sa pridá ako nový vstup – lacnejšie sledovanie do budúcna
            conn.execute("""INSERT INTO endpoint(source_id,kind,url,interval_s) VALUES (%s,'rss',%s,3600)
                            ON CONFLICT DO NOTHING""", (source["id"], f))
        for url, _text in child_links(page, res.final_url, cfg.max_child_links):
            if conn.execute("SELECT 1 FROM document WHERE url=%s", (url,)).fetchone():
                continue   # existujúce podstránky sa obnovujú vlastným plánom, nie pri každej kontrole
            await _process_document(conn, fetcher, source, url, stats, endpoint_id=ep["id"])
            conn.commit()


async def run_due(conn, cfg: Settings | None = None, source_id: str | None = None, limit: int = 50,
                  force: bool = False, transport=None) -> RunStats:
    cfg = cfg or default_settings
    stats = RunStats()
    q = "SELECT * FROM endpoint e WHERE EXISTS (SELECT 1 FROM source s WHERE s.id=e.source_id AND s.enabled)"
    args: list = []
    if source_id:
        q += " AND source_id=%s"
        args.append(source_id)
    if not force:
        q += " AND next_check <= now()"
    q += " ORDER BY next_check LIMIT %s"
    args.append(limit)
    eps = conn.execute(q, args).fetchall()
    async with Fetcher(cfg, transport=transport) as fetcher:
        for ep in eps:
            try:
                await check_endpoint(conn, fetcher, ep, cfg, stats)
                conn.commit()
            except Exception as e:
                conn.rollback()
                stats.errors += 1
                stats.messages.append(f"{ep['url']}: {type(e).__name__}: {e}")
                log.exception("Vstup %s zlyhal", ep["url"])
        await refresh_documents(conn, fetcher, cfg, stats, source_id=source_id, force=force)
    return stats


async def refresh_documents(conn, fetcher: Fetcher, cfg: Settings, stats: RunStats, source_id: str | None = None,
                            force: bool = False):
    """Znovu skontroluje známe dokumenty (podmienený GET) – zachytí zmenu aj na nezmenenej URL."""
    q = """SELECT s.*, d.url AS doc_url FROM document d JOIN source s ON s.id=d.source_id
           WHERE s.enabled AND d.status <> 'withdrawn'"""
    args: list = []
    if source_id:
        q += " AND s.id=%s"
        args.append(source_id)
    if not force:
        q += " AND d.last_seen < now() - make_interval(secs => %s)"
        args.append(cfg.doc_refresh_s)
    q += " ORDER BY d.last_seen LIMIT %s"
    args.append(cfg.max_doc_refresh)
    for row in conn.execute(q, args).fetchall():
        try:
            await _process_document(conn, fetcher, row, row["doc_url"], stats)
            conn.commit()
        except Exception as e:
            conn.rollback()
            stats.errors += 1
            stats.messages.append(f"{row['doc_url']}: {type(e).__name__}: {e}")


def run_due_sync(conn, **kw) -> RunStats:
    return asyncio.run(run_due(conn, **kw))
