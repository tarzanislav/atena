"""Parser robots.txt podľa RFC 9309 (vlastná implementácia).

- skupiny podľa user-agent (produktový token, bez ohľadu na veľkosť písmen),
- pravidlo s najdlhšou zhodou vyhráva; pri rovnakej dĺžke vyhráva allow,
- podpora zástupných znakov * a $,
- 4xx => všetko povolené, 5xx/nedostupné => všetko zakázané (RFC 9309, 2.3.1),
- spracuje sa najviac 500 KiB.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from urllib.parse import quote, unquote, urlsplit

MAX_BYTES = 500 * 1024


@dataclass
class _Group:
    agents: list[str] = field(default_factory=list)
    rules: list[tuple[bool, str]] = field(default_factory=list)  # (allow, pattern)
    crawl_delay: float | None = None


@dataclass
class RobotsPolicy:
    groups: list[_Group] = field(default_factory=list)
    sitemaps: list[str] = field(default_factory=list)
    mode: str = "parsed"   # parsed | allow_all | disallow_all

    @classmethod
    def allow_all(cls) -> "RobotsPolicy":
        return cls(mode="allow_all")

    @classmethod
    def disallow_all(cls) -> "RobotsPolicy":
        return cls(mode="disallow_all")

    def _group_for(self, agent_token: str) -> _Group | None:
        token = agent_token.lower()
        matched = [g for g in self.groups if token in g.agents]
        if not matched:
            matched = [g for g in self.groups if "*" in g.agents]
        if not matched:
            return None
        merged = _Group(agents=[token])
        for g in matched:   # RFC: viac skupín pre rovnaký agent sa zlúči
            merged.rules.extend(g.rules)
            if g.crawl_delay is not None:
                merged.crawl_delay = g.crawl_delay
        return merged

    def crawl_delay(self, agent_token: str) -> float | None:
        g = self._group_for(agent_token) if self.mode == "parsed" else None
        return g.crawl_delay if g else None

    def can_fetch(self, agent_token: str, url: str) -> bool:
        if self.mode == "allow_all":
            return True
        if self.mode == "disallow_all":
            return False
        parts = urlsplit(url)
        path = parts.path or "/"
        if parts.query:
            path += "?" + parts.query
        path = _norm(path)
        if path == "/robots.txt":
            return True
        g = self._group_for(agent_token)
        if g is None:
            return True
        best_len, best_allow = -1, True
        for allow, pat in g.rules:
            if pat == "":
                continue
            if _matches(pat, path):
                ln = len(pat)
                if ln > best_len or (ln == best_len and allow and not best_allow):
                    best_len, best_allow = ln, allow
        return best_allow


def _norm(path: str) -> str:
    # jednotné percentové kódovanie pre porovnanie (RFC 9309, 2.2.2)
    return quote(unquote(path), safe="/?&=*$:@!,;+~%-._")


def _matches(pattern: str, path: str) -> bool:
    pattern = _norm(pattern)
    anchored = pattern.endswith("$")
    if anchored:
        pattern = pattern[:-1]
    rx = "".join(".*" if ch == "*" else re.escape(ch) for ch in pattern)
    rx = "^" + rx + ("$" if anchored else "")
    return re.match(rx, path) is not None


def parse(text: str | bytes) -> RobotsPolicy:
    if isinstance(text, bytes):
        text = text[:MAX_BYTES].decode("utf-8", errors="replace")
    else:
        text = text[:MAX_BYTES]
    policy = RobotsPolicy()
    current: _Group | None = None
    last_was_agent = False
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line or ":" not in line:
            continue
        key, value = line.split(":", 1)
        key, value = key.strip().lower(), value.strip()
        if key == "user-agent":
            if current is None or not last_was_agent:
                current = _Group()
                policy.groups.append(current)
            current.agents.append(value.split("/")[0].strip().lower() or "*")
            last_was_agent = True
            continue
        if key == "sitemap":
            policy.sitemaps.append(value)
            continue
        last_was_agent = False
        if current is None:
            continue
        if key in ("allow", "disallow"):
            current.rules.append((key == "allow", value))
        elif key == "crawl-delay":
            try:
                current.crawl_delay = float(value)
            except ValueError:
                pass
    return policy


def from_status(status: int, body: bytes | None) -> RobotsPolicy:
    if 200 <= status < 300 and body is not None:
        return parse(body)
    if 400 <= status < 500:
        return RobotsPolicy.allow_all()
    return RobotsPolicy.disallow_all()
