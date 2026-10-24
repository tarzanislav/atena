"""Webové rozhranie a otvorené API ATENA (FastAPI + šablóny Jinja2, bez JS frameworku).

Server renderuje HTML – rýchle, prístupné (WCAG), funguje aj na slabom pripojení a vo
vyhľadávačoch. API /api/v1 je určené pre iné aplikácie (napr. ŠkVP: sekcia Novinky
z úradných zdrojov = /api/v1/novinky?autorita=A1,A2).
"""
from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from html import escape
from importlib import resources
from pathlib import Path
from urllib.parse import urlencode

from fastapi import Depends, FastAPI, Form, HTTPException, Query, Request
from fastapi.responses import HTMLResponse, JSONResponse, RedirectResponse, Response
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from .. import __version__
from ..db import connect
from ..search import Filters, search
from ..taxonomy import default_taxonomy
from ..versioning import unified_diff

BASE = Path(str(resources.files("atena").joinpath("web")))
app = FastAPI(title="ATENA", version=__version__, docs_url="/api/docs", openapi_url="/api/openapi.json")
app.mount("/static", StaticFiles(directory=BASE / "static"), name="static")
templates = Jinja2Templates(directory=BASE / "templates")
TAX = default_taxonomy()

# Sekcie -> filtre (stačí zhoda s ľubovoľnou dvojicou)
SECTIONS: dict[str, dict] = {
    "zmeny": {"any": [("tema", ["legislativa", "zmeny"]), ("typ-obsahu", ["predpis", "usmernenie"])]},
    "kurikulum": {"any": [("tema", ["kurikularna-reforma", "vzdelavacie-oblasti"])]},
    "prax": {"any": [("tema", ["aktivizacia", "ukazky-hodin", "metody-prax", "strategie-prax", "hodnotenie", "hry",
                               "fazy-procesu"]), ("metoda", None), ("strategia", None), ("forma", None)]},
    "gramotnosti": {"any": [("gramotnost", None), ("tema", ["citatelska", "urovne-porozumenia", "kriticke-myslenie"])]},
    "inkluzia": {"any": [("tema", ["inkluzia", "nadanie", "wellbeing", "cudzinci"])]},
    "digital": {"any": [("tema", ["digitalne", "ai"])]},
    "smery": {"any": [("smer", None), ("tema", ["alternativne"])]},
    "neformalne": {"any": [("tema", ["neformalne", "makke-zrucnosti", "celozivotne", "profesijny-rozvoj"])]},
    "riadenie": {"any": [("tema", ["riadenie", "rodicia"])]},
    "vyskum": {"any": [("tema", ["vyskum", "statistika", "trendy"])]},
    "svet": {"exclude_country": ["SK"]},
}
SCHOOL_TOGGLES = [c for c in TAX.scheme_concepts("druh-skoly") if c.broader is None]
PROFILE_COOKIE = "atena_profil"


def db():
    conn = connect()
    try:
        yield conn
    finally:
        conn.close()


def profile(request: Request) -> dict:
    try:
        p = json.loads(request.cookies.get(PROFILE_COOKIE, "{}"))
        return p if isinstance(p, dict) else {}
    except ValueError:
        return {}


def hidden_types(p: dict) -> list[str]:
    out = []
    for cid in p.get("vypnute", []):
        out.append(cid)
        out.extend(TAX.descendants("druh-skoly", cid))
    return out


def label_name(scheme: str, cid: str) -> str:
    c = TAX.get(scheme, cid)
    return c.label("sk") if c else cid


def ctx(request: Request, **kw) -> dict:
    base = {"request": request, "version": __version__, "sections": TAX.children("sekcia", None),
            "label": label_name, "tax": TAX, "profile": profile(request)}
    base.update(kw)
    return base


def _filters_from_query(request: Request, p: dict) -> Filters:
    qp = request.query_params
    concepts = {}
    for scheme in ("tema", "druh-skoly", "typ-obsahu", "gramotnost", "metoda", "strategia", "smer", "pre-koho"):
        vals = qp.getlist(scheme)
        if vals:
            concepts[scheme] = vals
    return Filters(country=qp.getlist("krajina"), authority=qp.getlist("autorita"), language=qp.getlist("jazyk"),
                   source=qp.getlist("zdroj"), concepts=concepts, hidden_school_types=hidden_types(p),
                   changed_only=qp.get("zmeny") == "1")


# ------------------------------------------------------------------- stránky
@app.get("/", response_class=HTMLResponse)
def home(request: Request, conn=Depends(db)):
    p = profile(request)
    hidden = hidden_types(p)
    official = search(conn, "", Filters(authority=["A1", "A2"], hidden_school_types=hidden, country=["SK"]), limit=8)
    changes = search(conn, "", Filters(changed_only=True, hidden_school_types=hidden), limit=8)
    latest = search(conn, "", Filters(hidden_school_types=hidden), limit=12)
    counts = conn.execute("""SELECT (SELECT count(*) FROM document) docs, (SELECT count(*) FROM source WHERE enabled) sources,
                             (SELECT count(*) FROM document_version WHERE change_kind='substantive') changes,
                             (SELECT max(at) FROM fetch_log) last_fetch""").fetchone()
    return templates.TemplateResponse(request, "home.html", ctx(request, official=official, changes=changes,
                                                                latest=latest, counts=counts))


@app.get("/hladat", response_class=HTMLResponse)
def search_page(request: Request, q: str = "", strana: int = 1, conn=Depends(db)):
    f = _filters_from_query(request, profile(request))
    res = search(conn, q, f, limit=20, offset=(max(strana, 1) - 1) * 20)
    next_qs = urlencode([(k, v) for k, v in request.query_params.multi_items() if k != "strana"] + [("strana", strana + 1)])
    return templates.TemplateResponse(request, "search.html", ctx(request, res=res, q=q, page=strana, next_qs=next_qs))


@app.get("/sekcia/{sid}", response_class=HTMLResponse)
def section(request: Request, sid: str, q: str = "", conn=Depends(db)):
    concept = TAX.get("sekcia", sid)
    if not concept:
        raise HTTPException(404, "Sekcia neexistuje")
    p = profile(request)
    spec = SECTIONS.get(sid.split(".")[0], {})
    f = _filters_from_query(request, p)
    f.any_concepts = spec.get("any", [])
    f.exclude_country = spec.get("exclude_country", [])
    res = search(conn, q, f, limit=30)
    return templates.TemplateResponse(request, "section.html", ctx(request, concept=concept, res=res, q=q,
                                                                   subsections=TAX.children("sekcia", sid)))


@app.get("/dokument/{doc_id}", response_class=HTMLResponse)
def document(request: Request, doc_id: int, conn=Depends(db)):
    d = conn.execute("""SELECT d.*, s.name source_name, s.authority, s.country, s.url source_url
                        FROM document d JOIN source s ON s.id=d.source_id WHERE d.id=%s""", (doc_id,)).fetchone()
    if not d:
        raise HTTPException(404, "Dokument neexistuje")
    versions = conn.execute("""SELECT id, version_no, fetched_at, change_kind, change, title, summary
                               FROM document_version WHERE document_id=%s ORDER BY version_no DESC""", (doc_id,)).fetchall()
    labels = conn.execute("""SELECT * FROM label WHERE document_id=%s AND status<>'rejected'
                             ORDER BY scheme, score DESC""", (doc_id,)).fetchall()
    tr = conn.execute("""SELECT * FROM translation WHERE version_id=%s ORDER BY created_at DESC LIMIT 1""",
                      (d["current_version_id"],)).fetchone() if d["current_version_id"] else None
    return templates.TemplateResponse(request, "document.html", ctx(request, d=d, versions=versions, labels=labels, tr=tr))


@app.get("/dokument/{doc_id}/rozdiel/{vno}", response_class=HTMLResponse)
def diff(request: Request, doc_id: int, vno: int, conn=Depends(db)):
    rows = conn.execute("""SELECT version_no, body, title, summary, fetched_at FROM document_version
                           WHERE document_id=%s AND version_no IN (%s, %s) ORDER BY version_no""",
                        (doc_id, vno - 1, vno)).fetchall()
    if len(rows) != 2:
        raise HTTPException(404, "Porovnanie nie je dostupné")
    old, new = rows
    lines = unified_diff(old["body"] or old["summary"], new["body"] or new["summary"])
    d = conn.execute("SELECT id, title, url FROM document WHERE id=%s", (doc_id,)).fetchone()
    return templates.TemplateResponse(request, "diff.html", ctx(request, d=d, old=old, new=new, lines=lines))


@app.get("/zdroje", response_class=HTMLResponse)
def sources(request: Request, conn=Depends(db)):
    rows = conn.execute("""SELECT s.*, (SELECT count(*) FROM document d WHERE d.source_id=s.id) docs,
                                  (SELECT max(last_checked) FROM endpoint e WHERE e.source_id=s.id) checked,
                                  (SELECT string_agg(DISTINCT kind, ', ') FROM endpoint e WHERE e.source_id=s.id) kinds,
                                  (SELECT max(fail_count) FROM endpoint e WHERE e.source_id=s.id) fails
                           FROM source s ORDER BY s.priority, s.id""").fetchall()
    return templates.TemplateResponse(request, "sources.html", ctx(request, rows=rows))


@app.get("/pojmy", response_class=HTMLResponse)
def glossary(request: Request, schema: str = "tema"):
    if schema not in TAX.schemes:
        raise HTTPException(404)
    return templates.TemplateResponse(request, "glossary.html", ctx(request, schema=schema,
                                                                    concepts=TAX.scheme_concepts(schema)))


@app.get("/profil", response_class=HTMLResponse)
def profile_page(request: Request):
    return templates.TemplateResponse(request, "profile.html", ctx(request, toggles=SCHOOL_TOGGLES))


@app.post("/profil")
async def profile_save(request: Request):
    form = await request.form()
    on = set(form.getlist("druh"))
    p = {"vypnute": [c.id for c in SCHOOL_TOGGLES if c.id not in on]}
    r = RedirectResponse("/profil?ulozene=1", status_code=303)
    r.set_cookie(PROFILE_COOKIE, json.dumps(p), max_age=3600 * 24 * 365, samesite="lax", httponly=True)
    return r


# ---------------------------------------------------------------- redakcia
def require_editor(request: Request):
    token = os.environ.get("ATENA_EDITOR_TOKEN")
    given = request.headers.get("x-atena-token") or request.cookies.get("atena_editor")
    if not token or given != token:
        raise HTTPException(403, "Redakčné akcie vyžadujú prihlásenie redaktora")
    return "editor"


@app.post("/api/v1/labels/{doc_id}")
def decide_label(doc_id: int, scheme: str = Form(...), concept_id: str = Form(...), decision: str = Form(...),
                 who: str = Depends(require_editor), conn=Depends(db)):
    if decision not in ("confirmed", "rejected"):
        raise HTTPException(400, "decision = confirmed | rejected")
    conn.execute("""INSERT INTO label(document_id,scheme,concept_id,origin,status,decided_by,decided_at)
                    VALUES (%s,%s,%s,'editor',%s,%s,now())
                    ON CONFLICT (document_id,scheme,concept_id) DO UPDATE SET status=EXCLUDED.status,
                      origin='editor', decided_by=EXCLUDED.decided_by, decided_at=now()""",
                 (doc_id, scheme, concept_id, decision, who))
    conn.execute("""INSERT INTO audit_log(actor,action,entity,entity_id,detail) VALUES (%s,'label',%s,%s,%s)""",
                 (who, "document", str(doc_id), json.dumps({"scheme": scheme, "concept": concept_id, "decision": decision})))
    conn.commit()
    return {"ok": True}


# --------------------------------------------------------------------- API
def _jsonable(rows):
    return json.loads(json.dumps(rows, default=lambda o: o.isoformat() if isinstance(o, datetime) else str(o)))


@app.get("/api/v1/search")
def api_search(request: Request, q: str = "", limit: int = Query(20, le=100), offset: int = 0, conn=Depends(db)):
    res = search(conn, q, _filters_from_query(request, {}), limit=limit, offset=offset)
    return JSONResponse(_jsonable(res))


@app.get("/api/v1/documents/{doc_id}")
def api_document(doc_id: int, conn=Depends(db)):
    d = conn.execute("""SELECT d.id, d.url, d.title, d.language, d.published_at, d.first_seen, d.last_changed,
                               d.status, d.rights_mode, s.id source_id, s.name source_name, s.authority, s.country
                        FROM document d JOIN source s ON s.id=d.source_id WHERE d.id=%s""", (doc_id,)).fetchone()
    if not d:
        raise HTTPException(404)
    d["versions"] = conn.execute("""SELECT version_no, fetched_at, change_kind, change, summary FROM document_version
                                    WHERE document_id=%s ORDER BY version_no DESC""", (doc_id,)).fetchall()
    d["labels"] = conn.execute("""SELECT scheme, concept_id, score, origin, status FROM label
                                  WHERE document_id=%s AND status<>'rejected'""", (doc_id,)).fetchall()
    return JSONResponse(_jsonable(d))


@app.get("/api/v1/sources")
def api_sources(conn=Depends(db)):
    return JSONResponse(_jsonable(conn.execute("SELECT * FROM source ORDER BY id").fetchall()))


@app.get("/api/v1/taxonomy")
def api_taxonomy():
    return {s: [{"id": c.id, "broader": c.broader, "labels": c.labels} for c in TAX.scheme_concepts(s)]
            for s in TAX.schemes}


@app.get("/api/v1/novinky")
def api_news(autorita: str = "A1,A2", krajina: str = "SK", limit: int = Query(30, le=200), conn=Depends(db)):
    """Novinky len z vybraných úrovní autority – napr. pre ŠkVP (iba úradné zdroje, žiadne médiá)."""
    rows = conn.execute("""SELECT d.id, d.title, d.url, d.published_at, d.last_changed, v.summary, v.change_kind,
                                  s.name source_name, s.authority, s.country
                           FROM document d JOIN document_version v ON v.id=d.current_version_id
                           JOIN source s ON s.id=d.source_id
                           WHERE s.authority = ANY(%s) AND s.country = ANY(%s) AND d.status='active'
                           ORDER BY coalesce(d.published_at, d.last_changed) DESC LIMIT %s""",
                        (autorita.split(","), krajina.split(","), limit)).fetchall()
    return JSONResponse(_jsonable(rows))


@app.get("/feed/zmeny.atom")
def atom(request: Request, autorita: str = "A1,A2", conn=Depends(db)):
    rows = conn.execute("""SELECT d.id, d.title, d.url, d.last_changed, v.summary, v.change_kind, s.name
                           FROM document d JOIN document_version v ON v.id=d.current_version_id
                           JOIN source s ON s.id=d.source_id WHERE s.authority = ANY(%s)
                           ORDER BY d.last_changed DESC LIMIT 50""", (autorita.split(","),)).fetchall()
    base = str(request.base_url).rstrip("/")
    now = datetime.now(timezone.utc).isoformat()
    entries = "".join(
        f"<entry><id>{base}/dokument/{r['id']}</id><title>{escape(r['title'])}</title>"
        f"<link href=\"{escape(r['url'])}\"/><updated>{r['last_changed'].isoformat()}</updated>"
        f"<summary>{escape(r['name'])}: {escape(r['summary'] or '')}</summary></entry>" for r in rows)
    xml = (f"<?xml version='1.0' encoding='utf-8'?><feed xmlns='http://www.w3.org/2005/Atom'>"
           f"<id>{base}/feed/zmeny.atom</id><title>ATENA – zmeny z úradných zdrojov</title><updated>{now}</updated>{entries}</feed>")
    return Response(xml, media_type="application/atom+xml")


@app.get("/zdravie")
def health(conn=Depends(db)):
    conn.execute("SELECT 1")
    return {"ok": True, "version": __version__}


templates.env.globals["urlencode"] = urlencode
