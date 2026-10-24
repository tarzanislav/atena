import re
import tomllib
from pathlib import Path

from atena.registry import interval_from_text, load_sources
from atena.taxonomy import default_taxonomy

ROOT = Path(__file__).resolve().parents[1]


def test_sources_are_valid():
    src = load_sources()
    tax = default_taxonomy()
    assert len(src) >= 180
    for s in src:
        assert tax.get("autorita", s["authority"]), s["id"]
        assert tax.get("krajina", s["country"]), (s["id"], s["country"])
        assert s["url"].startswith("http"), s["id"]
        assert s["status"] != "Vyradiť"


def test_frequencies():
    assert interval_from_text("1× za hodinu (kanál)") == 3600
    assert interval_from_text("týždenne") == 7 * 86400
    assert interval_from_text("jednorazovo") == 365 * 86400


# Požiadavka projektu: žiadne licencie s poplatkami ani obmedzením komerčného použitia.
ALLOWED = {
    "fastapi": "MIT", "uvicorn": "BSD-3-Clause", "jinja2": "BSD-3-Clause", "python-multipart": "Apache-2.0",
    "httpx": "BSD-3-Clause", "psycopg": "LGPL-3.0 (knižnica bez úprav – komerčne povolené)", "pyyaml": "MIT",
    "defusedxml": "PSF-2.0", "feedparser": "BSD-2-Clause", "trafilatura": "Apache-2.0", "pypdf": "BSD-3-Clause",
    "python-docx": "MIT", "py3langid": "BSD-3-Clause", "pytest": "MIT",
}


def test_dependency_licences_are_free_for_commercial_use():
    data = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
    deps = data["project"]["dependencies"] + data["project"]["optional-dependencies"]["dev"]
    for d in deps:
        name = re.split(r"[\[<>=!~ ]", d, maxsplit=1)[0].lower()
        assert name in ALLOWED, f"Nová závislosť {name} – najprv over licenciu a doplň ju do zoznamu"
    assert data["project"]["license"] == "Apache-2.0"
