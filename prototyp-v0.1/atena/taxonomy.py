"""Načítanie taxonómie (SKOS-like YAML) a pravidlový viacjazyčný návrh zaradenia.

Zaradenie je VŽDY len návrh (origin='auto', status='suggested'). Potvrdzuje ho redaktor.
Pravidlá sú zámerne priehľadné – pri každom návrhu sa uloží, ktoré výrazy ho spôsobili,
aby bolo vidieť dôvod (žiadna čierna skrinka).
"""
from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from functools import lru_cache
from importlib import resources
from typing import Iterable

import yaml

from .textnorm import fold, has_cjk, tokens


@dataclass
class Concept:
    scheme: str
    id: str
    labels: dict[str, str]
    broader: str | None = None
    note: str | None = None
    keywords: dict[str, list[list[str]]] = field(default_factory=dict)   # jazyk -> frázy ako zoznam tokenov
    raw_keywords: dict[str, list[str]] = field(default_factory=dict)
    extra: dict = field(default_factory=dict)

    def label(self, lang: str = "sk") -> str:
        return self.labels.get(lang) or self.labels.get("sk") or self.labels.get("en") or self.id


@dataclass
class Taxonomy:
    schemes: dict[str, dict]
    concepts: dict[tuple[str, str], Concept]

    def scheme_concepts(self, scheme: str) -> list[Concept]:
        return [c for (s, _), c in self.concepts.items() if s == scheme]

    def get(self, scheme: str, cid: str) -> Concept | None:
        return self.concepts.get((scheme, cid))

    def children(self, scheme: str, cid: str | None) -> list[Concept]:
        return [c for c in self.scheme_concepts(scheme) if c.broader == cid]

    def descendants(self, scheme: str, cid: str) -> set[str]:
        out, stack = set(), [cid]
        while stack:
            cur = stack.pop()
            for ch in self.children(scheme, cur):
                if ch.id not in out:
                    out.add(ch.id)
                    stack.append(ch.id)
        return out


def _phrase(kw: str) -> list[str]:
    return tokens(kw) if not has_cjk(kw) else [fold(kw)]


def load_taxonomy(text: str | None = None) -> Taxonomy:
    if text is None:
        text = resources.files("atena").joinpath("data/taxonomia.yaml").read_text(encoding="utf-8")
    data = yaml.safe_load(text)
    concepts: dict[tuple[str, str], Concept] = {}
    for sname, sdef in data["schemes"].items():
        for c in sdef.get("concepts", []):
            cid = str(c["id"])
            labels = {"sk": c.get("sk"), "en": c.get("en")}
            labels.update(c.get("labels") or {})
            labels = {k: v for k, v in labels.items() if v}
            kw_raw = c.get("kw") or {}
            kw = {lang: [p for p in (_phrase(k) for k in lst) if p] for lang, lst in kw_raw.items()}
            extra = {k: v for k, v in c.items() if k not in {"id", "sk", "en", "labels", "kw", "broader", "note"}}
            key = (sname, cid)
            if key in concepts:
                raise ValueError(f"Duplicitný pojem {sname}:{cid}")
            concepts[key] = Concept(sname, cid, labels, c.get("broader"), c.get("note"), kw, kw_raw, extra)
    # kontrola väzieb broader
    for (s, cid), c in concepts.items():
        if c.broader and (s, c.broader) not in concepts:
            raise ValueError(f"Pojem {s}:{cid} odkazuje na neexistujúci nadradený {c.broader}")
    return Taxonomy(data["schemes"], concepts)


@lru_cache(maxsize=1)
def default_taxonomy() -> Taxonomy:
    return load_taxonomy()


# ---------------------------------------------------------------------------
# Návrh zaradenia
# ---------------------------------------------------------------------------

@dataclass
class Suggestion:
    scheme: str
    concept_id: str
    score: float
    evidence: list[str]


def _match_phrase(toks: list[str], phrase: list[str]) -> int:
    """Počet výskytov frázy; token kľúčového slova ≥4 znaky sa porovnáva ako začiatok slova
    (od 7 znakov bez posledných dvoch písmen – zachytí pádové koncovky)."""
    n, m = len(toks), len(phrase)
    if m == 0 or n < m:
        return 0
    count = 0
    for i in range(n - m + 1):
        ok = True
        for j in range(m):
            kw, tk = phrase[j], toks[i + j]
            if len(kw) >= 4:
                # ľahký „stemming“ pre skloňované jazyky: čítanie ~ čítania, porozumením ~ porozumenie
                stem = kw[:-2] if len(kw) >= 7 else kw
                if not tk.startswith(stem):
                    ok = False
                    break
            elif tk != kw:
                ok = False
                break
        if ok:
            count += 1
    return count


# Typ obsahu – jednoduché pravidlá podľa názvu, prípony a typu súboru
_TYPE_RULES: list[tuple[str, re.Pattern]] = [
    ("predpis", re.compile(r"\b(zakon c|zakona c|vyhlaska|vyhlasky|nariadenie vlady|z\. ?z\.|sb\.|dz\.u|ustawa|rozporzadzenie|gesetz|verordnung|torveny|rendelet|федеральный закон)", re.I)),
    ("usmernenie", re.compile(r"\b(usmernenie|metodicky pokyn|pedagogicko-organizacne pokyny|guidance|leitfaden|wytyczne)", re.I)),
    ("webinar", re.compile(r"\b(webinar|webinár|webinarz|вебинар)", re.I)),
    ("kurz", re.compile(r"\b(kurz|course|szkolenie|kurs|fortbildung)\b", re.I)),
    ("pracovny-list", re.compile(r"\b(pracovny list|pracovne listy|worksheet|karta pracy|arbeitsblatt)", re.I)),
    ("scenar", re.compile(r"\b(scenar|plan hodiny|priprava na hodinu|lesson plan|scenariusz|unterrichtsentwurf)", re.I)),
    ("metodika", re.compile(r"\b(metodika|metodicka prirucka|prirucka|handbook|teacher guide|poradnik|handreichung)", re.I)),
    ("vyzva", re.compile(r"\b(vyzva|grant|sutaz|call for|konkurs|ausschreibung)", re.I)),
    ("podujatie", re.compile(r"\b(konferencia|seminar|workshop|podujatie|conference|konferencja|tagung)", re.I)),
    ("sprava", re.compile(r"\b(sprava o|analyza|report|raport|bericht)", re.I)),
    ("synteza", re.compile(r"\b(systematic review|meta-analys|metaanaly|systematicky prehlad)", re.I)),
]


def suggest(title: str, text: str, language: str | None, taxonomy: Taxonomy | None = None,
            schemes: Iterable[str] | None = None, threshold: float = 0.45) -> list[Suggestion]:
    """Navrhne pojmy pre dokument. Používajú sa kľúčové slová jazyka dokumentu
    (a pri neznámom jazyku všetkých jazykov), aby sa nemiešali významy medzi jazykmi."""
    tax = taxonomy or default_taxonomy()
    title_t = tokens(title or "")
    body = text or ""
    head_t = tokens(body[:1500])
    rest_t = tokens(body[1500:20000])
    title_f, body_f = fold(title or ""), fold(body[:20000])
    wanted = set(schemes) if schemes else None
    out: list[Suggestion] = []
    for (s, cid), c in tax.concepts.items():
        if wanted and s not in wanted:
            continue
        if not c.keywords:
            continue
        langs = [language] if language and language in c.keywords else (list(c.keywords) if not language else [])
        if language and language not in c.keywords:
            continue
        weight, ev = 0.0, []
        for lang in langs:
            for raw, ph in zip(c.raw_keywords.get(lang, []), c.keywords.get(lang, [])):
                if len(ph) == 1 and has_cjk(ph[0]):
                    hits = 3 * title_f.count(ph[0]) + body_f.count(ph[0])
                else:
                    hits = 3 * _match_phrase(title_t, ph) + 2 * _match_phrase(head_t, ph) + _match_phrase(rest_t, ph)
                if hits:
                    weight += min(hits, 6)
                    ev.append(f"{lang}:{raw}×{hits}")
        if weight:
            score = 1 - math.exp(-weight / 3)
            if score >= threshold:
                out.append(Suggestion(s, cid, round(score, 3), ev))
    # typ obsahu
    if not wanted or "typ-obsahu" in wanted:
        probe = fold((title or "") + " " + body[:600])
        for cid, rx in _TYPE_RULES:
            if rx.search(probe):
                out.append(Suggestion("typ-obsahu", cid, 0.6, [f"vzor:{rx.pattern[:40]}"]))
                break
    out.sort(key=lambda x: (-x.score, x.scheme, x.concept_id))
    return out
