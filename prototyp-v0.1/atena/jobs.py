"""Jednoduchá a spoľahlivá fronta úloh priamo v PostgreSQL (bez RabbitMQ/Redis).

* doručenie „aspoň raz“, deduplikácia kľúčom (dedup_key),
* FOR UPDATE SKIP LOCKED – viac pracovníkov naraz bez kolízií,
* exponenciálne opakovanie, po max_attempts zostane úloha v zozname chýb.
"""
from __future__ import annotations

import json
import os
import socket


WORKER_ID = f"{socket.gethostname()}:{os.getpid()}"


def enqueue(conn, kind: str, payload: dict | None = None, dedup_key: str | None = None, delay_s: int = 0) -> int | None:
    row = conn.execute(
        """INSERT INTO job(kind,payload,dedup_key,run_at) VALUES (%s,%s,%s, now() + make_interval(secs => %s))
           ON CONFLICT (dedup_key) WHERE done_at IS NULL AND dedup_key IS NOT NULL DO NOTHING RETURNING id""",
        (kind, json.dumps(payload or {}), dedup_key, delay_s)).fetchone()
    return row["id"] if row else None


def claim(conn, kinds: list[str] | None = None) -> dict | None:
    q = """UPDATE job SET locked_at=now(), locked_by=%s, attempts=attempts+1
           WHERE id = (SELECT id FROM job WHERE done_at IS NULL AND run_at <= now()
                       AND (locked_at IS NULL OR locked_at < now() - interval '15 minutes')
                       AND attempts < max_attempts {kinds}
                       ORDER BY run_at FOR UPDATE SKIP LOCKED LIMIT 1)
           RETURNING *"""
    args: list = [WORKER_ID]
    if kinds:
        q = q.format(kinds="AND kind = ANY(%s)")
        args.append(kinds)
    else:
        q = q.format(kinds="")
    row = conn.execute(q, args).fetchone()
    conn.commit()
    return row


def complete(conn, job_id: int) -> None:
    conn.execute("UPDATE job SET done_at=now(), locked_at=NULL WHERE id=%s", (job_id,))
    conn.commit()


def fail(conn, job: dict, error: str) -> None:
    backoff = min(60 * (2 ** job["attempts"]), 6 * 3600)
    conn.execute("""UPDATE job SET locked_at=NULL, last_error=%s, run_at=now() + make_interval(secs => %s)
                    WHERE id=%s""", (error[:2000], backoff, job["id"]))
    conn.commit()


def failed(conn, limit: int = 50) -> list[dict]:
    return conn.execute("""SELECT * FROM job WHERE done_at IS NULL AND attempts >= max_attempts
                           ORDER BY created_at DESC LIMIT %s""", (limit,)).fetchall()
