"""Vyhľadávanie: fulltext bez diakritiky s predponovým porovnaním (náhrada stemmingu pre
skloňované jazyky), trigramy pre preklepy v názve a fazetové filtre.

Vektorové (významové) vyhľadávanie je pripravená ďalšia vrstva (pgvector + BGE-M3/E5);
výsledky sa spoja metódou Reciprocal Rank Fusion bez zmeny tohto rozhrania.
"""
from __future__ import annotations

from dataclasses import dataclass, field

from .textnorm import fold, has_cjk, tokens

STOP = {"a", "i", "v", "vo", "na", "do", "z", "zo", "s", "so", "k", "ku", "o", "pre", "pri", "za", "the", "of",
        "and", "in", "to", "for", "je", "sa", "ako", "aj", "alebo", "der", "die", "das", "und", "w", "na", "dla"}


@dataclass
class Filters:
    country: list[str] = field(default_factory=list)
    authority: list[str] = field(default_factory=list)
    language: list[str] = field(default_factory=list)
    source: list[str] = field(default_factory=list)
    # pojmy: {"tema": ["citatelska"], "druh-skoly": ["zs1"]} – v rámci schémy ALEBO, medzi schémami A ZÁROVEŇ
    concepts: dict[str, list[str]] = field(default_factory=dict)
    # vypnuté druhy škôl z profilu: skryje položky zaradené IBA do vypnutých druhov
    hidden_school_types: list[str] = field(default_factory=list)
    # sekcia: stačí zhoda s ľubovoľnou dvojicou (schéma, pojmy|None = celá schéma)
    any_concepts: list[tuple[str, list[str] | None]] = field(default_factory=list)
    exclude_country: list[str] = field(default_factory=list)
    changed_only: bool = False


def build_tsquery(q: str) -> str | None:
    toks = [t for t in tokens(q) if t not in STOP]
    if not toks:
        return None
    parts = []
    for t in toks[:12]:
        safe = "".join(ch for ch in t if ch.isalnum())
        if not safe:
            continue
        # predpona od 4 znakov zachytí skloňovanie (čitateľská/čitateľskej -> citatel:*)
        stem = safe[: max(4, len(safe) - 2)] if len(safe) > 5 else safe
        parts.append(f"{stem}:*")
    return " & ".join(parts) if parts else None


def search(conn, q: str = "", filters: Filters | None = None, limit: int = 20, offset: int = 0) -> dict:
    f = filters or Filters()
    where = ["d.status <> 'withdrawn'", "s.enabled"]
    args: dict = {}
    rank = "0"
    tsq = build_tsquery(q) if q and not has_cjk(q) else None
    if tsq:
        where.append("(v.tsv @@ to_tsquery('simple', %(tsq)s) OR atena_fold(v.title) %% %(qf)s)")
        args.update(tsq=tsq, qf=fold(q))
        rank = "ts_rank_cd(v.tsv, to_tsquery('simple', %(tsq)s), 32) + similarity(atena_fold(v.title), %(qf)s)"
    elif q:
        where.append("(v.title ILIKE %(like)s OR v.body ILIKE %(like)s)")
        args["like"] = f"%{q}%"
        rank = "1"
    for col, vals, key in (("s.country", f.country, "country"), ("s.authority", f.authority, "auth"),
                           ("d.language", f.language, "lang"), ("s.id", f.source, "src")):
        if vals:
            where.append(f"{col} = ANY(%({key})s)")
            args[key] = vals
    for i, (scheme, ids) in enumerate(sorted(f.concepts.items())):
        if ids:
            where.append(f"""EXISTS (SELECT 1 FROM label l WHERE l.document_id=d.id AND l.status<>'rejected'
                              AND l.scheme=%(sc{i})s AND l.concept_id = ANY(%(ci{i})s))""")
            args[f"sc{i}"], args[f"ci{i}"] = scheme, ids
    if f.any_concepts:
        ors = []
        for i, (scheme, ids) in enumerate(f.any_concepts):
            args[f"as{i}"] = scheme
            cond = f"l.scheme=%(as{i})s"
            if ids:
                args[f"ai{i}"] = ids
                cond += f" AND l.concept_id = ANY(%(ai{i})s)"
            ors.append(f"({cond})")
        where.append(f"""EXISTS (SELECT 1 FROM label l WHERE l.document_id=d.id AND l.status<>'rejected'
                          AND ({' OR '.join(ors)}))""")
    if f.exclude_country:
        where.append("NOT (s.country = ANY(%(exc)s))")
        args["exc"] = f.exclude_country
    if f.hidden_school_types:
        where.append("""NOT (EXISTS (SELECT 1 FROM label l WHERE l.document_id=d.id AND l.scheme='druh-skoly'
                                     AND l.status<>'rejected')
                         AND NOT EXISTS (SELECT 1 FROM label l WHERE l.document_id=d.id AND l.scheme='druh-skoly'
                                     AND l.status<>'rejected' AND NOT (l.concept_id = ANY(%(hidden)s))))""")
        args["hidden"] = f.hidden_school_types
    if f.changed_only:
        where.append("v.change_kind = 'substantive'")
    sql_where = " AND ".join(where)
    base = f"""FROM document d JOIN document_version v ON v.id=d.current_version_id
               JOIN source s ON s.id=d.source_id WHERE {sql_where}"""
    total = conn.execute(f"SELECT count(*) AS n {base}", args).fetchone()["n"]
    rows = conn.execute(
        f"""SELECT d.id, d.url, d.title, d.language, d.published_at, d.last_changed, d.status, d.rights_mode,
                   v.summary, v.change_kind, v.version_no, s.id AS source_id, s.name AS source_name,
                   s.country, s.authority, ({rank}) AS rank
            {base} ORDER BY rank DESC, d.last_changed DESC LIMIT %(limit)s OFFSET %(offset)s""",
        {**args, "limit": limit, "offset": offset}).fetchall()
    ids = [r["id"] for r in rows]
    labels = {}
    if ids:
        for l in conn.execute("""SELECT document_id, scheme, concept_id, status, score FROM label
                                 WHERE document_id = ANY(%s) AND status<>'rejected' ORDER BY score DESC""", (ids,)):
            labels.setdefault(l["document_id"], []).append(l)
    for r in rows:
        r["labels"] = labels.get(r["id"], [])
    facets = {}
    if total:
        facets["autorita"] = conn.execute(f"SELECT s.authority AS k, count(*) AS n {base} GROUP BY 1 ORDER BY 2 DESC", args).fetchall()
        facets["krajina"] = conn.execute(f"SELECT s.country AS k, count(*) AS n {base} GROUP BY 1 ORDER BY 2 DESC", args).fetchall()
        facets["tema"] = conn.execute(
            f"""SELECT l.concept_id AS k, count(*) AS n {base.replace('WHERE', 'JOIN label l ON l.document_id=d.id AND l.scheme=%(fsc)s AND l.status<>%(rej)s WHERE', 1)}
                GROUP BY 1 ORDER BY 2 DESC LIMIT 15""", {**args, "fsc": "tema", "rej": "rejected"}).fetchall()
    return {"total": total, "items": rows, "facets": facets, "query": q, "tsquery": tsq}
