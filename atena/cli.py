"""Príkazový riadok ATENA.

  atena init                 – vytvorí schému, načíta taxonómiu a register zdrojov
  atena crawl [--zdroj ID]   – skontroluje vstupy, ktorým uplynul interval (alebo --vsetko)
  atena worker               – nepretržitý zber podľa plánu (pre server / kontajner)
  atena serve                – webové rozhranie a API
  atena reclassify           – prepočíta automatické návrhy zaradenia po zmene taxonómie
  atena translate            – preloží anotácie cudzojazyčných dokumentov (ak je nastavený prekladač)
  atena stav                 – prehľad databázy a zdravia zdrojov
"""
from __future__ import annotations

import argparse
import json
import logging
import sys
import time

from .db import connect, init_schema


def cmd_init(a):
    from .registry import load_sources, sync_sources, sync_taxonomy
    conn = connect()
    init_schema(conn)
    n = sync_taxonomy(conn)
    st = sync_sources(conn, load_sources())
    print(f"Schéma pripravená · pojmy: {n} · zdroje: {st['sources']} · vstupy: {st['endpoints']}")


def cmd_crawl(a):
    from .pipeline import run_due_sync
    conn = connect()
    st = run_due_sync(conn, source_id=a.zdroj, limit=a.limit, force=a.vsetko)
    print(json.dumps(st.as_dict(), ensure_ascii=False, indent=2))


def cmd_worker(a):
    from .pipeline import run_due_sync
    log = logging.getLogger("atena.worker")
    while True:
        try:
            conn = connect()
            st = run_due_sync(conn, limit=a.limit)
            if st.endpoints:
                log.info("Kolo zberu: %s", st.as_dict())
            conn.close()
        except Exception:
            log.exception("Kolo zberu zlyhalo – pokračujem o chvíľu")
        time.sleep(a.pauza)


def cmd_serve(a):
    import uvicorn
    uvicorn.run("atena.web.app:app", host=a.host, port=a.port, proxy_headers=True)


def cmd_reclassify(a):
    from .store import apply_suggestions
    from .taxonomy import suggest
    conn = connect()
    rows = conn.execute("""SELECT d.id, d.language, v.title, v.body, v.summary FROM document d
                           JOIN document_version v ON v.id=d.current_version_id""").fetchall()
    for r in rows:
        apply_suggestions(conn, r["id"], suggest(r["title"], r["body"] or r["summary"], r["language"]))
    conn.commit()
    print(f"Prepočítané: {len(rows)} dokumentov")


def cmd_translate(a):
    from .translate import default_translator, needs_translation, translate_version
    tr = default_translator()
    if not tr:
        print("Prekladač nie je nastavený (ATENA_TRANSLATE_URL). Nič sa neprekladá – originály ostávajú dostupné.")
        return
    conn = connect()
    rows = conn.execute("""SELECT d.current_version_id vid, d.language FROM document d
                           WHERE d.current_version_id IS NOT NULL AND NOT EXISTS
                           (SELECT 1 FROM translation t WHERE t.version_id=d.current_version_id)
                           ORDER BY d.last_changed DESC LIMIT %s""", (a.limit,)).fetchall()
    done = 0
    for r in rows:
        if needs_translation(r["language"]):
            translate_version(conn, r["vid"], tr)
            conn.commit()
            done += 1
    print(f"Preložené anotácie: {done}")


def cmd_stav(a):
    conn = connect()
    q = lambda sql: conn.execute(sql).fetchone()
    print(json.dumps({
        "zdroje": q("SELECT count(*) n FROM source WHERE enabled")["n"],
        "vstupy": q("SELECT count(*) n FROM endpoint")["n"],
        "dokumenty": q("SELECT count(*) n FROM document")["n"],
        "verzie": q("SELECT count(*) n FROM document_version")["n"],
        "podstatne_zmeny": q("SELECT count(*) n FROM document_version WHERE change_kind='substantive'")["n"],
        "navrhy_zaradenia": q("SELECT count(*) n FROM label WHERE status='suggested'")["n"],
        "vstupy_s_chybou": q("SELECT count(*) n FROM endpoint WHERE fail_count>0")["n"],
    }, ensure_ascii=False, indent=2))


def main(argv=None):
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s: %(message)s")
    p = argparse.ArgumentParser(prog="atena", description="ATENA – agregátor informácií o vzdelávaní")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("init").set_defaults(fn=cmd_init)
    c = sub.add_parser("crawl")
    c.add_argument("--zdroj")
    c.add_argument("--limit", type=int, default=50)
    c.add_argument("--vsetko", action="store_true", help="ignorovať plán a skontrolovať hneď")
    c.set_defaults(fn=cmd_crawl)
    w = sub.add_parser("worker")
    w.add_argument("--limit", type=int, default=20)
    w.add_argument("--pauza", type=int, default=60)
    w.set_defaults(fn=cmd_worker)
    s = sub.add_parser("serve")
    s.add_argument("--host", default="127.0.0.1")
    s.add_argument("--port", type=int, default=8000)
    s.set_defaults(fn=cmd_serve)
    sub.add_parser("reclassify").set_defaults(fn=cmd_reclassify)
    t = sub.add_parser("translate")
    t.add_argument("--limit", type=int, default=50)
    t.set_defaults(fn=cmd_translate)
    sub.add_parser("stav").set_defaults(fn=cmd_stav)
    a = p.parse_args(argv)
    a.fn(a)


if __name__ == "__main__":
    sys.exit(main())
