"""Slušný HTTP zberač.

Zásady:
* identifikovaný User-Agent s kontaktom, robots.txt (RFC 9309) a Crawl-delay,
* najviac jedna požiadavka na hostiteľa naraz + minimálny odstup,
* podmienené požiadavky (ETag / Last-Modified) – nesťahuje nezmenený obsah,
* limit veľkosti a času, kontrola SSRF pri KAŽDOM presmerovaní,
* rešpektuje Retry-After, pri chybách postupne predlžuje odstup,
* zistí výhradu vyťažovania textov a dát (TDMRep: hlavička alebo meta značka).
"""
from __future__ import annotations

import asyncio
import email.utils
import re
import time
from dataclasses import dataclass, field
from urllib.parse import urljoin, urlsplit

import httpx

from .. import USER_AGENT, USER_AGENT_TOKEN
from ..config import Settings, settings as default_settings
from . import robots as robotslib
from .netguard import BlockedURL, check_url

_META_TDM = re.compile(rb"<meta[^>]+name=[\"']tdm-reservation[\"'][^>]*content=[\"']\s*1", re.I)
_META_TDM2 = re.compile(rb"<meta[^>]+content=[\"']\s*1[\"'][^>]*name=[\"']tdm-reservation[\"']", re.I)
_META_NOINDEX = re.compile(rb"<meta[^>]+name=[\"'](?:robots|atena-bot)[\"'][^>]*content=[\"'][^\"']*(noindex|none)", re.I)


@dataclass
class FetchResult:
    url: str
    final_url: str
    status: int
    headers: dict[str, str] = field(default_factory=dict)
    content: bytes = b""
    not_modified: bool = False
    blocked_reason: str | None = None   # robots | ssrf | too_large | error
    error: str | None = None
    elapsed_ms: int = 0
    tdm_reserved: bool = False
    noindex: bool = False

    @property
    def ok(self) -> bool:
        return self.blocked_reason is None and self.error is None and 200 <= self.status < 300

    @property
    def content_type(self) -> str:
        return self.headers.get("content-type", "").split(";")[0].strip().lower()

    @property
    def etag(self) -> str | None:
        return self.headers.get("etag")

    @property
    def last_modified(self) -> str | None:
        return self.headers.get("last-modified")


def _retry_after(value: str | None) -> float | None:
    if not value:
        return None
    try:
        return max(0.0, float(value))
    except ValueError:
        try:
            dt = email.utils.parsedate_to_datetime(value)
            return max(0.0, dt.timestamp() - time.time())
        except (TypeError, ValueError):
            return None


class Fetcher:
    def __init__(self, cfg: Settings | None = None, transport: httpx.AsyncBaseTransport | None = None):
        self.cfg = cfg or default_settings
        self._client = httpx.AsyncClient(
            headers={"User-Agent": USER_AGENT, "Accept-Language": "sk,cs;q=0.9,en;q=0.8,*;q=0.5"},
            timeout=self.cfg.request_timeout_s, follow_redirects=False, transport=transport)
        self._robots: dict[str, tuple[float, robotslib.RobotsPolicy]] = {}
        self._host_locks: dict[str, asyncio.Lock] = {}
        self._host_next: dict[str, float] = {}
        self._host_penalty: dict[str, float] = {}

    async def aclose(self):
        await self._client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *exc):
        await self.aclose()

    # ------------------------------------------------------------------ hosts
    def _origin(self, url: str) -> str:
        p = urlsplit(url)
        return f"{p.scheme}://{p.netloc}"

    async def _wait_turn(self, origin: str, delay: float):
        lock = self._host_locks.setdefault(origin, asyncio.Lock())
        await lock.acquire()
        now = time.monotonic()
        wait = self._host_next.get(origin, 0) - now
        if wait > 0:
            await asyncio.sleep(wait)
        return lock

    def _release(self, origin: str, lock: asyncio.Lock, delay: float):
        self._host_next[origin] = time.monotonic() + delay + self._host_penalty.get(origin, 0)
        lock.release()

    # ----------------------------------------------------------------- robots
    async def robots_for(self, url: str) -> robotslib.RobotsPolicy:
        origin = self._origin(url)
        cached = self._robots.get(origin)
        if cached and time.monotonic() - cached[0] < 24 * 3600:
            return cached[1]
        res = await self._raw_get(origin + "/robots.txt", {}, max_bytes=robotslib.MAX_BYTES, respect_robots=False)
        if res.blocked_reason == "ssrf":
            policy = robotslib.RobotsPolicy.disallow_all()
        elif res.error and res.status == 0:
            policy = robotslib.RobotsPolicy.disallow_all()   # nedostupný server => opatrne nič
        else:
            policy = robotslib.from_status(res.status, res.content)
        self._robots[origin] = (time.monotonic(), policy)
        return policy

    # ------------------------------------------------------------------- GET
    async def get(self, url: str, etag: str | None = None, last_modified: str | None = None,
                  max_bytes: int | None = None) -> FetchResult:
        headers = {}
        if etag:
            headers["If-None-Match"] = etag
        if last_modified:
            headers["If-Modified-Since"] = last_modified
        return await self._raw_get(url, headers, max_bytes=max_bytes or self.cfg.max_bytes, respect_robots=True)

    async def _raw_get(self, url: str, headers: dict, max_bytes: int, respect_robots: bool) -> FetchResult:
        start = time.monotonic()
        current = url
        for _hop in range(self.cfg.max_redirects + 1):
            try:
                check_url(current, allow_private=self.cfg.allow_private_networks)
            except BlockedURL as e:
                return FetchResult(url, current, 0, blocked_reason="ssrf", error=str(e))
            if respect_robots:
                policy = await self.robots_for(current)
                if not policy.can_fetch(USER_AGENT_TOKEN, current):
                    return FetchResult(url, current, 0, blocked_reason="robots", error="Zakázané v robots.txt")
                delay = max(self.cfg.per_host_delay_s, min(policy.crawl_delay(USER_AGENT_TOKEN) or 0, 60))
            else:
                delay = self.cfg.per_host_delay_s
            origin = self._origin(current)
            lock = await self._wait_turn(origin, delay)
            try:
                async with self._client.stream("GET", current, headers=headers) as resp:
                    status = resp.status_code
                    hdrs = {k.lower(): v for k, v in resp.headers.items()}
                    if status in (301, 302, 303, 307, 308) and "location" in hdrs:
                        nxt = urljoin(current, hdrs["location"])
                        current = nxt
                        continue
                    if status in (429, 503):
                        ra = _retry_after(hdrs.get("retry-after"))
                        self._host_penalty[origin] = min(max(ra or 0, self._host_penalty.get(origin, 1) * 2), 3600)
                        return FetchResult(url, current, status, hdrs, error=f"HTTP {status} – server žiada spomalenie",
                                           elapsed_ms=int((time.monotonic() - start) * 1000))
                    self._host_penalty.pop(origin, None)
                    if status == 304:
                        return FetchResult(url, current, status, hdrs, not_modified=True,
                                           elapsed_ms=int((time.monotonic() - start) * 1000))
                    declared = int(hdrs.get("content-length") or 0)
                    if declared > max_bytes:
                        return FetchResult(url, current, status, hdrs, blocked_reason="too_large",
                                           error=f"Obsah {declared} B prekračuje limit")
                    buf = bytearray()
                    async for chunk in resp.aiter_bytes():
                        buf.extend(chunk)
                        if len(buf) > max_bytes:
                            return FetchResult(url, current, status, hdrs, blocked_reason="too_large",
                                               error="Obsah prekračuje limit")
                    body = bytes(buf)
                    res = FetchResult(url, current, status, hdrs, body,
                                      elapsed_ms=int((time.monotonic() - start) * 1000))
                    res.tdm_reserved = hdrs.get("tdm-reservation", "").strip() == "1" or bool(
                        _META_TDM.search(body[:65536]) or _META_TDM2.search(body[:65536]))
                    res.noindex = "noindex" in hdrs.get("x-robots-tag", "").lower() or bool(_META_NOINDEX.search(body[:65536]))
                    if not (200 <= status < 300):
                        res.error = f"HTTP {status}"
                    return res
            except httpx.HTTPError as e:
                return FetchResult(url, current, 0, error=f"{type(e).__name__}: {e}",
                                   elapsed_ms=int((time.monotonic() - start) * 1000))
            finally:
                self._release(origin, lock, delay)
        return FetchResult(url, current, 0, error="Príliš veľa presmerovaní")
