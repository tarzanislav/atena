"""Prístup k PostgreSQL (psycopg 3)."""
from __future__ import annotations

from contextlib import contextmanager
from importlib import resources

import psycopg
from psycopg.rows import dict_row

from ..config import settings


def connect(url: str | None = None) -> psycopg.Connection:
    return psycopg.connect(url or settings.database_url, row_factory=dict_row)


@contextmanager
def session(url: str | None = None):
    conn = connect(url)
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def init_schema(conn: psycopg.Connection) -> None:
    sql = resources.files("atena").joinpath("db/schema.sql").read_text(encoding="utf-8")
    conn.execute(sql)
    conn.commit()
