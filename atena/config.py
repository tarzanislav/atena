"""Konfigurácia z premenných prostredia (žiadne tajomstvá v kóde)."""
from __future__ import annotations

import os
from dataclasses import dataclass, field


def _bool(name: str, default: bool) -> bool:
    v = os.environ.get(name)
    if v is None:
        return default
    return v.strip().lower() in {"1", "true", "ano", "áno", "yes", "on"}


@dataclass
class Settings:
    database_url: str = field(default_factory=lambda: os.environ.get(
        "ATENA_DATABASE_URL", "postgresql://postgres@localhost:5432/atena"))
    # Slušný zber
    per_host_delay_s: float = field(default_factory=lambda: float(os.environ.get("ATENA_HOST_DELAY", "2.0")))
    request_timeout_s: float = field(default_factory=lambda: float(os.environ.get("ATENA_TIMEOUT", "30")))
    max_bytes: int = field(default_factory=lambda: int(os.environ.get("ATENA_MAX_BYTES", str(25 * 1024 * 1024))))
    max_redirects: int = 5
    # Bezpečnosť: v produkcii nikdy nepovoliť súkromné adresy (ochrana pred SSRF)
    allow_private_networks: bool = field(default_factory=lambda: _bool("ATENA_ALLOW_PRIVATE", False))
    # Zber
    max_items_per_feed: int = field(default_factory=lambda: int(os.environ.get("ATENA_MAX_FEED_ITEMS", "30")))
    # Opätovná kontrola už známych dokumentov (tichá výmena PDF, zmena textu na rovnakej URL)
    doc_refresh_s: int = field(default_factory=lambda: int(os.environ.get("ATENA_DOC_REFRESH_S", str(7 * 86400))))
    max_doc_refresh: int = field(default_factory=lambda: int(os.environ.get("ATENA_MAX_DOC_REFRESH", "100")))
    max_child_links: int = field(default_factory=lambda: int(os.environ.get("ATENA_MAX_CHILD_LINKS", "20")))
    # Preklad – voliteľný lokálny server s rozhraním kompatibilným s OpenAI (llama.cpp, vLLM…)
    translate_base_url: str | None = field(default_factory=lambda: os.environ.get("ATENA_TRANSLATE_URL"))
    translate_model: str = field(default_factory=lambda: os.environ.get("ATENA_TRANSLATE_MODEL", "eurollm-9b-instruct"))
    target_language: str = field(default_factory=lambda: os.environ.get("ATENA_TARGET_LANG", "sk"))
    # Jazyky, ktoré sa cieľovému používateľovi neprekladajú (zadanie: čeština ostáva v origináli)
    no_translate: tuple[str, ...] = field(default_factory=lambda: tuple(
        os.environ.get("ATENA_NO_TRANSLATE", "sk,cs").split(",")))


settings = Settings()
