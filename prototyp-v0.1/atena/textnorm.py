"""Normalizácia textu pre porovnávanie, odtlačky a zaraďovanie (viacjazyčne)."""
from __future__ import annotations

import hashlib
import re
import unicodedata

_WS = re.compile(r"\s+")
_TOKEN = re.compile(r"[^\W_]+(?:[-'][^\W_]+)*", re.UNICODE)
_CJK = re.compile(r"[㐀-鿿豈-﫿]")


def fold(text: str) -> str:
    """Malé písmená bez diakritiky (č→c, ä→a, ő→o). Rovnako sa upraví text aj kľúčové slovo."""
    text = unicodedata.normalize("NFKD", text.casefold())
    return "".join(ch for ch in text if not unicodedata.combining(ch))


def clean_ws(text: str) -> str:
    return _WS.sub(" ", text).strip()


def normalize_for_hash(text: str) -> str:
    """Odstráni rozdiely, ktoré nie sú zmenou obsahu (medzery, zalomenia, typografické úvodzovky)."""
    t = unicodedata.normalize("NFKC", text)
    t = t.replace(" ", " ").replace("„", '"').replace("“", '"').replace("”", '"')
    t = t.replace("–", "-").replace("—", "-")
    return clean_ws(t)


def sha256(text: str | bytes) -> str:
    if isinstance(text, str):
        text = text.encode("utf-8")
    return hashlib.sha256(text).hexdigest()


def tokens(text: str) -> list[str]:
    return _TOKEN.findall(fold(text))


def has_cjk(text: str) -> bool:
    return bool(_CJK.search(text))


def paragraphs(text: str) -> list[str]:
    parts = re.split(r"\n\s*\n|\n", text)
    return [clean_ws(p) for p in parts if clean_ws(p)]
