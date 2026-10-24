"""Verzie dokumentov a rozlíšenie podstatnej zmeny od kozmetickej."""
from __future__ import annotations

import difflib
import re
from dataclasses import dataclass, field

from .textnorm import normalize_for_hash, paragraphs, sha256

_NUM = re.compile(r"\d+(?:[.,]\d+)*")
_DATE = re.compile(r"\b\d{1,2}\.\s?\d{1,2}\.\s?\d{4}\b|\b\d{4}-\d{2}-\d{2}\b")


@dataclass
class Change:
    kind: str                     # initial | none | minor | substantive
    ratio: float = 1.0
    added: list[str] = field(default_factory=list)
    removed: list[str] = field(default_factory=list)
    numbers_changed: bool = False

    def summary(self) -> dict:
        return {"kind": self.kind, "ratio": round(self.ratio, 4), "added": self.added[:12],
                "removed": self.removed[:12], "added_count": len(self.added), "removed_count": len(self.removed),
                "numbers_changed": self.numbers_changed}


def text_hash(text: str) -> str:
    return sha256(normalize_for_hash(text))


def compare(old: str | None, new: str) -> Change:
    if old is None:
        return Change("initial")
    if text_hash(old) == text_hash(new):
        return Change("none")
    a = [normalize_for_hash(p) for p in paragraphs(old)]
    b = [normalize_for_hash(p) for p in paragraphs(new)]
    sm = difflib.SequenceMatcher(a=a, b=b, autojunk=False)
    added, removed = [], []
    for op, i1, i2, j1, j2 in sm.get_opcodes():
        if op in ("replace", "delete"):
            removed.extend(a[i1:i2])
        if op in ("replace", "insert"):
            added.extend(b[j1:j2])
    ratio = difflib.SequenceMatcher(a=normalize_for_hash(old), b=normalize_for_hash(new), autojunk=False).quick_ratio()
    nums_old = set(_NUM.findall(" ".join(removed))) | set(_DATE.findall(" ".join(removed)))
    nums_new = set(_NUM.findall(" ".join(added))) | set(_DATE.findall(" ".join(added)))
    numbers_changed = nums_old != nums_new
    changed_chars = sum(len(x) for x in added) + sum(len(x) for x in removed)
    # Čísla a dátumy sú v právnych a organizačných textoch vždy podstatné.
    substantive = numbers_changed or changed_chars >= 40
    return Change("substantive" if substantive else "minor", ratio, added, removed, numbers_changed)


def unified_diff(old: str, new: str, context: int = 1) -> list[tuple[str, str]]:
    """Riadky pre zobrazenie rozdielu: ('+'|'-'|' ', text)."""
    out = []
    for line in difflib.unified_diff(paragraphs(old), paragraphs(new), lineterm="", n=context):
        if line.startswith(("---", "+++", "@@")):
            if line.startswith("@@"):
                out.append(("@", "…"))
            continue
        out.append((line[0], line[1:]))
    return out
