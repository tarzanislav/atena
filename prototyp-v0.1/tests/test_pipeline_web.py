"""Koniec-koncov test: testovací web -> zber -> verzie -> zaradenie -> vyhľadávanie -> web a API."""
import asyncio
import dataclasses

from atena.config import Settings
from atena.pipeline import run_due
from atena.registry import sync_sources, sync_taxonomy
from atena.search import Filters, search

PAGE = """<html lang="sk"><head><title>{title}</title></head><body><nav>Menu Domov</nav><article>
<h1>{title}</h1><p>{body}</p><p>Materská škola a základná škola môžu metodiku využiť pri rozvoji čitateľskej gramotnosti
a čítania s porozumením. Učiteľ zaradí aktivizačné metódy a skupinovú prácu.</p></article></body></html>"""

BODY1 = "Termín podania žiadostí je 20. 3. 2026. Metodika obsahuje pracovné listy a scenáre hodín pre učiteľov."


def feed(base):
    return f"""<?xml version="1.0"?><rss version="2.0"><channel><title>Test</title>
<item><title>Metodika čítania</title><link>{base}/clanok-1</link><pubDate>Fri, 16 Oct 2026 10:00:00 +0000</pubDate></item>
<item><title>Súkromné</title><link>{base}/private/tajne</link></item>
<item><title>Vyhradené</title><link>{base}/vyhradene</link></item>
</channel></rss>"""


def cfg():
    return dataclasses.replace(Settings(), allow_private_networks=True, per_host_delay_s=0.0, max_child_links=5)


def setup_site(site):
    site.set("/robots.txt", "User-agent: *\nDisallow: /private/\n", "text/plain")
    site.set("/feed/", feed(site.base), "application/rss+xml")
    site.set("/clanok-1", PAGE.format(title="Metodika čítania s porozumením", body=BODY1))
    site.set("/private/tajne", PAGE.format(title="Tajné", body="nemá sa zobraziť"))
    site.set("/vyhradene", PAGE.format(title="Vyhradené", body="Autor si vyhradil vyťažovanie textov a dát. " * 3),
             headers={"tdm-reservation": "1"})


def test_end_to_end(conn, site):
    setup_site(site)
    sync_taxonomy(conn)
    sync_sources(conn, [{"id": "T1", "name": "Testovací úrad", "url": site.base + "/", "country": "SK",
                         "languages": ["sk"], "authority": "A2", "priority": "P0", "connector": "wp",
                         "feed": site.base + "/feed/", "frequency": "1× za hodinu", "status": "Ponechať"}])
    st = asyncio.run(run_due(conn, cfg=cfg(), force=True))
    assert st.blocked >= 1                        # robots.txt zakázal /private/
    assert "/private/tajne" not in site.hits
    docs = {r["url"].replace(site.base, ""): r for r in conn.execute("SELECT * FROM document").fetchall()}
    assert "/clanok-1" in docs and "/private/tajne" not in docs
    assert docs["/vyhradene"]["rights_mode"] == "metadata_only"   # výhrada TDM => len metadáta
    body = conn.execute("SELECT body FROM document_version v JOIN document d ON d.current_version_id=v.id "
                        "WHERE d.url LIKE '%%/vyhradene'").fetchone()["body"]
    assert body == ""
    labels = {(r["scheme"], r["concept_id"]) for r in conn.execute(
        "SELECT scheme, concept_id FROM label WHERE document_id=%s", (docs["/clanok-1"]["id"],))}
    assert ("tema", "citatelska") in labels and ("druh-skoly", "ms") in labels

    # druhý beh: nič sa nezmenilo => žiadna nová verzia (podmienený GET / odtlačok)
    st2 = asyncio.run(run_due(conn, cfg=cfg(), force=True))
    assert st2.new_docs == 0 and st2.substantive == 0
    # zmena dátumu => podstatná zmena a nová verzia
    site.set("/clanok-1", PAGE.format(title="Metodika čítania s porozumením", body=BODY1.replace("20. 3.", "27. 3.")))
    st3 = asyncio.run(run_due(conn, cfg=cfg(), force=True))
    assert st3.substantive == 1
    v = conn.execute("SELECT version_no, change_kind FROM document_version WHERE document_id=%s ORDER BY version_no",
                     (docs["/clanok-1"]["id"],)).fetchall()
    assert [x["change_kind"] for x in v] == ["initial", "substantive"]

    # vyhľadávanie: skloňovaný tvar a bez diakritiky
    assert search(conn, "čitateľskej gramotnosti")["total"] >= 1
    assert search(conn, "citatelska gramotnost")["total"] >= 1
    assert search(conn, "", Filters(concepts={"tema": ["citatelska"]}))["total"] >= 1
    # profil: vypnuté MŠ aj ZŠ => dokument zaradený len do nich sa skryje
    hidden = search(conn, "", Filters(hidden_school_types=["ms", "zs1", "zs2", "zakladne", "predprimarne"]))
    assert all(it["url"] != site.base + "/clanok-1" for it in hidden["items"])

    # web a API
    from fastapi.testclient import TestClient
    from atena.web.app import app
    c = TestClient(app)
    for path in ["/", "/hladat?q=metodika", "/sekcia/gramotnosti", "/sekcia/prax", "/zdroje", "/pojmy",
                 "/pojmy?schema=druh-skoly", "/profil", f"/dokument/{docs['/clanok-1']['id']}",
                 f"/dokument/{docs['/clanok-1']['id']}/rozdiel/2", "/feed/zmeny.atom", "/api/v1/taxonomy", "/zdravie"]:
        r = c.get(path)
        assert r.status_code == 200, (path, r.text[:500])
    assert "27. 3. 2026" in c.get(f"/dokument/{docs['/clanok-1']['id']}/rozdiel/2").text
    news = c.get("/api/v1/novinky?autorita=A1,A2").json()
    assert any(n["url"].endswith("/clanok-1") for n in news)
    assert c.get("/api/v1/search?q=metodika").json()["total"] >= 1
    # profil sa uloží do cookie a skryje vypnuté druhy škôl
    r = c.post("/profil", data={"druh": ["stredne", "svvp"]}, follow_redirects=False)
    assert r.status_code == 303 and "atena_profil" in r.headers["set-cookie"]
    # redakčné rozhodnutie bez tokenu je zakázané
    assert c.post(f"/api/v1/labels/{docs['/clanok-1']['id']}",
                  data={"scheme": "tema", "concept_id": "citatelska", "decision": "confirmed"}).status_code == 403
