"""Register zdrojov: načítanie YAML a synchronizácia do databázy (bez zásahu do kódu)."""
from __future__ import annotations

import re
from importlib import resources

import yaml

from .taxonomy import Taxonomy, default_taxonomy

_FREQ = [
    (re.compile(r"hodin"), 3600),
    (re.compile(r"2× denne|2x denne"), 43200),
    (re.compile(r"týžden"), 7 * 86400),   # pred "denne" – "týždenne" obsahuje "denne"
    (re.compile(r"denne"), 86400),
    (re.compile(r"mesač"), 30 * 86400),
    (re.compile(r"štvrťroč"), 90 * 86400),
    (re.compile(r"ročne|pri vydaní|pri zmene|jednorazovo"), 365 * 86400),
]


def interval_from_text(freq: str | None) -> int:
    f = (freq or "").lower()
    for rx, sec in _FREQ:
        if rx.search(f):
            return sec
    return 86400


def load_sources(text: str | None = None) -> list[dict]:
    if text is None:
        text = resources.files("atena").joinpath("data/zdroje.yaml").read_text(encoding="utf-8")
    data = yaml.safe_load(text)
    ids = set()
    for s in data["sources"]:
        for req in ("id", "name", "url", "authority"):
            if not s.get(req):
                raise ValueError(f"Zdroj {s.get('id')} nemá povinné pole {req}")
        if s["id"] in ids:
            raise ValueError(f"Duplicitné ID zdroja {s['id']}")
        ids.add(s["id"])
    return data["sources"]


def sync_sources(conn, sources: list[dict]) -> dict:
    stats = {"sources": 0, "endpoints": 0}
    for s in sources:
        enabled = s.get("status") not in ("Vyradiť",)
        conn.execute(
            """INSERT INTO source(id,name,url,country,languages,authority,priority,connector,frequency,status,note,enabled)
               VALUES (%(id)s,%(name)s,%(url)s,%(country)s,%(languages)s,%(authority)s,%(priority)s,%(connector)s,
                       %(frequency)s,%(status)s,%(note)s,%(enabled)s)
               ON CONFLICT (id) DO UPDATE SET name=EXCLUDED.name,url=EXCLUDED.url,country=EXCLUDED.country,
                 languages=EXCLUDED.languages,authority=EXCLUDED.authority,priority=EXCLUDED.priority,
                 connector=EXCLUDED.connector,frequency=EXCLUDED.frequency,status=EXCLUDED.status,note=EXCLUDED.note,
                 enabled=EXCLUDED.enabled,updated_at=now()""",
            {"id": s["id"], "name": s["name"], "url": s["url"], "country": s.get("country", "WORLD"),
             "languages": s.get("languages") or [], "authority": s["authority"], "priority": s.get("priority", "P1"),
             "connector": s.get("connector", "html"), "frequency": s.get("frequency"), "status": s.get("status"),
             "note": s.get("note"), "enabled": enabled})
        stats["sources"] += 1
        interval = interval_from_text(s.get("frequency"))
        eps = []
        if s.get("feed"):
            eps.append(("rss", s["feed"], interval))
        if s.get("connector") in ("html", "wp", "rss", "katalog", None) or not s.get("feed"):
            # stránka zdroja sa vždy sleduje aj ako dokument (zmena obsahu / príloh)
            eps.append(("html", s["url"], max(interval, 86400)))
        for kind, url, iv in eps:
            conn.execute(
                """INSERT INTO endpoint(source_id,kind,url,interval_s) VALUES (%s,%s,%s,%s)
                   ON CONFLICT (source_id,url) DO UPDATE SET kind=EXCLUDED.kind, interval_s=EXCLUDED.interval_s""",
                (s["id"], kind, url, iv))
            stats["endpoints"] += 1
    conn.commit()
    return stats


def sync_taxonomy(conn, tax: Taxonomy | None = None) -> int:
    import json
    tax = tax or default_taxonomy()
    n = 0
    for (scheme, cid), c in tax.concepts.items():
        conn.execute(
            """INSERT INTO concept(scheme,id,broader,labels,note) VALUES (%s,%s,%s,%s,%s)
               ON CONFLICT (scheme,id) DO UPDATE SET broader=EXCLUDED.broader, labels=EXCLUDED.labels, note=EXCLUDED.note""",
            (scheme, cid, c.broader, json.dumps(c.labels, ensure_ascii=False), c.note))
        n += 1
    conn.commit()
    return n
