"""Ochrana pred SSRF – zberač nesmie siahnuť na interné/súkromné adresy.

Kontroluje sa schéma, port, všetky IP adresy z DNS a každé presmerovanie zvlášť.
"""
from __future__ import annotations

import ipaddress
import socket
from urllib.parse import urlsplit


class BlockedURL(Exception):
    pass


ALLOWED_SCHEMES = {"http", "https"}
ALLOWED_PORTS = {None, 80, 443, 8080, 8443}


def _ip_allowed(ip: str) -> bool:
    addr = ipaddress.ip_address(ip)
    if isinstance(addr, ipaddress.IPv6Address) and addr.ipv4_mapped:
        addr = addr.ipv4_mapped
    return addr.is_global and not addr.is_multicast


def check_url(url: str, allow_private: bool = False, resolver=socket.getaddrinfo) -> None:
    parts = urlsplit(url)
    if parts.scheme not in ALLOWED_SCHEMES:
        raise BlockedURL(f"Nepovolená schéma: {parts.scheme!r}")
    if not parts.hostname:
        raise BlockedURL("URL bez hostiteľa")
    if parts.username or parts.password:
        raise BlockedURL("URL s prihlasovacími údajmi nie je povolená")
    if allow_private:
        return
    if parts.port not in ALLOWED_PORTS:
        raise BlockedURL(f"Nepovolený port: {parts.port}")
    host = parts.hostname
    try:
        ipaddress.ip_address(host)
        ips = [host]
    except ValueError:
        if host == "localhost" or host.endswith(".localhost") or host.endswith(".internal") or host.endswith(".local"):
            raise BlockedURL(f"Interný hostiteľ: {host}")
        try:
            infos = resolver(host, parts.port or (443 if parts.scheme == "https" else 80), proto=socket.IPPROTO_TCP)
        except socket.gaierror as e:
            raise BlockedURL(f"DNS zlyhalo pre {host}: {e}") from e
        ips = sorted({i[4][0] for i in infos})
    if not ips:
        raise BlockedURL(f"Žiadna adresa pre {host}")
    bad = [ip for ip in ips if not _ip_allowed(ip)]
    if bad:
        raise BlockedURL(f"Neverejná adresa {bad[0]} pre {host}")
