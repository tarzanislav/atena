"""One-off migration of the prototype data (Slovak IDs) to the language-neutral v2 format.

Reads prototyp-v0.1/atena/data/{taxonomia,zdroje}.yaml and writes:
  data/taxonomy.yaml   – English stable IDs, labels/notes/keywords per language
  data/sources.yaml    – English field values, interval in seconds, notes per language
  data/id-map.csv      – old scheme:id -> new scheme:id (for the prototype and old links)

Already applied; data/ is now edited directly. Do not re-run it: it would overwrite later work
such as the Czech, Russian and Chinese labels.
"""
import csv
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
OLD = ROOT / "prototyp-v0.1" / "atena" / "data"
STOP = {"and", "the", "of", "a", "an", "for", "in", "on", "to", "with", "by"}

SCHEMES = {
    "sekcia": "section", "tema": "topic", "gramotnost": "literacy", "metoda": "method",
    "strategia": "strategy", "forma": "form", "ramec-faz": "lesson-phase-framework",
    "ramec-porozumenia": "comprehension-framework", "smer": "approach", "druh-skoly": "school-type",
    "typ-obsahu": "resource-type", "pre-koho": "audience", "autorita": "authority",
    "dokaz": "evidence", "krajina": "country",
}

OVERRIDES = {
    "section": {
        "prehlad": "today", "zmeny": "changes", "kurikulum": "curriculum", "prax": "practice",
        "gramotnosti": "literacies", "inkluzia": "inclusion", "digital": "digital", "smery": "approaches",
        "neformalne": "non-formal", "riadenie": "leadership", "vyskum": "research", "svet": "world",
        "pojmy": "glossary", "kalendar": "calendar", "moja": "my-atena",
        "zmeny.zahranicie": "foreign-regulations", "vyskum.merania": "assessments",
        "kurikulum.svp": "national-curriculum", "kurikulum.zahranicie": "foreign-curricula",
    },
    "topic": {
        "trendy": "trends", "zmeny": "changes", "metody-prax": "teaching-methods",
        "strategie-prax": "teaching-strategies", "neformalne": "non-formal", "digitalne": "digital-learning",
        "inkluzia": "inclusion", "nadanie": "gifted", "vzdelavacie-oblasti": "learning-areas",
        "citatelska": "reading-literacy", "urovne-porozumenia": "comprehension-levels", "ai": "ai",
        "fazy-procesu": "lesson-phases", "legislativa": "legislation", "terminologia": "terminology",
        "wellbeing": "wellbeing", "cudzinci": "second-language-learners", "riadenie": "school-leadership",
        "profesijny-rozvoj": "professional-development", "vyskum": "research-evidence",
        "statistika": "statistics-assessments", "celozivotne": "lifelong-learning",
        "odborne": "vocational-education", "rodicia": "family-partnership",
    },
    "literacy": {"jazykova": "language"},
    "method": {
        "vyklad": "lecture", "rozhovor": "questioning", "demonstracia": "demonstration",
        "praca-s-textom": "working-with-text", "problemova": "problem-solving", "inscenacna": "role-play",
        "brainstorming": "brainstorming", "riesene-priklady": "worked-examples", "metakognicia": "metacognition",
        "exit-ticket": "exit-ticket", "hejneho": "hejny-method",
    },
    "strategy": {"gamifikacia": "gamification", "personalizovane": "differentiation", "steam": "stem-steam"},
    "form": {"centra-aktivit": "learning-centres", "distancna": "distance"},
    "lesson-phase-framework": {
        "klasicky": "classical", "klasicky.motivacna": "classical.motivation",
        "klasicky.expozicna": "classical.exposition", "klasicky.fixacna": "classical.consolidation",
        "klasicky.diagnosticka": "classical.assessment", "klasicky.aplikacna": "classical.application",
        "eur": "err", "eur.evokacia": "err.evocation", "eur.uvedomenie": "err.realisation",
        "eur.reflexia": "err.reflection", "5e": "5e", "5e.engage": "5e.engage", "5e.explore": "5e.explore",
        "5e.explain": "5e.explain", "5e.elaborate": "5e.elaborate", "5e.evaluate": "5e.evaluate",
        "gagne": "gagne", "rosenshine": "rosenshine", "herbart": "herbart",
    },
    "comprehension-framework": {
        "barrett": "barrett", "barrett.doslovne": "barrett.literal", "barrett.reorganizacia": "barrett.reorganisation",
        "barrett.inferencne": "barrett.inferential", "barrett.hodnotiace": "barrett.evaluation",
        "barrett.ocenenie": "barrett.appreciation", "pirls": "pirls", "pirls.vyhladanie": "pirls.retrieve",
        "pirls.vyvodenie": "pirls.infer", "pirls.integracia": "pirls.interpret-integrate",
        "pirls.hodnotenie": "pirls.evaluate-critique", "pisa2018": "pisa2018",
        "pisa2018.lokalizacia": "pisa2018.locate", "pisa2018.porozumenie": "pisa2018.understand",
        "pisa2018.hodnotenie": "pisa2018.evaluate-reflect", "bloom": "bloom", "bloom.zapamatat": "bloom.remember",
        "bloom.porozumiet": "bloom.understand", "bloom.aplikovat": "bloom.apply", "bloom.analyzovat": "bloom.analyse",
        "bloom.hodnotit": "bloom.evaluate", "bloom.tvorit": "bloom.create",
    },
    "approach": {"waldorf": "waldorf", "step-by-step": "step-by-step", "p4c": "p4c"},
    "school-type": {
        "jasle": "nursery", "gymnazium": "gymnasium", "sos": "vocational-secondary", "svvp": "special-education",
        "svvp.ms": "special-kindergarten", "svvp.zs": "special-primary", "svvp.ss": "special-secondary",
        "vs": "higher-education",
    },
    "resource-type": {
        "strategia": "policy", "sprava": "report", "dataset": "dataset", "vyzva": "call", "novinka": "news",
        "skusenost": "practice-account", "interaktivita": "interactive", "nastroj": "tool",
    },
    "audience": {"vychovavatel": "educator", "lektor": "youth-worker"},
}
KEEP_IDS = {"authority", "evidence", "country"}

# Resource types are recognised from the title and the first lines (v0.1 used regular expressions in code;
# here they are data, per language, like every other keyword).
TYPE_KEYWORDS = {
    "legal-act": {"sk": ["zákon č", "vyhláška", "nariadenie vlády", "z. z"], "cs": ["zákon č", "vyhláška", "sb"],
                  "pl": ["ustawa", "rozporządzenie", "dz.u"], "de": ["gesetz", "verordnung"], "hu": ["törvény", "rendelet"],
                  "ru": ["федеральный закон"], "en": ["act", "statutory instrument"]},
    "guidance": {"sk": ["usmernenie", "metodický pokyn", "pedagogicko-organizačné pokyny"], "cs": ["metodický pokyn"],
                 "en": ["guidance"], "de": ["leitfaden"], "pl": ["wytyczne"]},
    "webinar": {"sk": ["webinár"], "cs": ["webinář"], "en": ["webinar"], "de": ["webinar"], "pl": ["webinar"], "ru": ["вебинар"]},
    "course": {"sk": ["kurz"], "cs": ["kurz"], "en": ["course"], "de": ["fortbildung"], "pl": ["szkolenie", "kurs"], "ru": ["курс"]},
    "worksheet": {"sk": ["pracovný list", "pracovné listy"], "cs": ["pracovní list"], "en": ["worksheet"],
                  "de": ["arbeitsblatt"], "pl": ["karta pracy"]},
    "lesson-plan": {"sk": ["scenár", "plán hodiny", "príprava na hodinu"], "cs": ["příprava na hodinu", "scénář"],
                    "en": ["lesson plan"], "de": ["unterrichtsentwurf"], "pl": ["scenariusz"], "ru": ["конспект урока"]},
    "teacher-guide": {"sk": ["metodika", "metodická príručka", "príručka"], "cs": ["metodika", "příručka"],
                      "en": ["handbook", "teacher guide"], "de": ["handreichung"], "pl": ["poradnik"]},
    "call": {"sk": ["výzva", "grant", "súťaž"], "cs": ["výzva", "grant", "soutěž"], "en": ["call for", "grant"],
             "de": ["ausschreibung"], "pl": ["konkurs"]},
    "event": {"sk": ["konferencia", "seminár", "workshop", "podujatie"], "cs": ["konference", "seminář"],
              "en": ["conference", "workshop"], "de": ["tagung"], "pl": ["konferencja"]},
    "report": {"sk": ["správa o", "analýza"], "cs": ["zpráva o", "analýza"], "en": ["report"], "de": ["bericht"], "pl": ["raport"]},
    "systematic-review": {"sk": ["metaanalýz", "systematický prehľad"], "en": ["systematic review", "meta-analysis"],
                          "de": ["metaanalyse"]},
}

STATUS = {"Ponechať": "keep", "Upraviť": "adjusted", "Zlúčiť ako vstup": "merged-endpoint",
          "Archív (jednorazovo)": "archive", "Nový zdroj": "new"}


def slug(label: str, maxw: int = 4) -> str:
    words = [w for w in re.findall(r"[a-z0-9]+", label.lower().replace("&", " ")) if w not in STOP]
    return "-".join(words[:maxw])


def interval(freq: str | None) -> int:
    f = (freq or "").lower()
    for rx, sec in [("hodin", 3600), ("2× denne", 43200), ("týžden", 604800), ("denne", 86400),
                    ("mesač", 2592000), ("štvrťroč", 7776000), ("ročne|pri vydaní|pri zmene|jednorazovo", 31536000)]:
        if re.search(rx, f):
            return sec
    return 86400


def main() -> None:
    old = yaml.safe_load((OLD / "taxonomia.yaml").read_text(encoding="utf-8"))
    idmap: list[tuple[str, str, str, str]] = []
    out_schemes = {}
    for old_s, sdef in old["schemes"].items():
        new_s = SCHEMES[old_s]
        ids: dict[str, str] = {}
        for c in sdef["concepts"]:
            cid = str(c["id"])
            new_id = cid if new_s in KEEP_IDS else OVERRIDES.get(new_s, {}).get(cid) or slug(c["en"])
            if new_id in ids.values():
                sys.exit(f"duplicate id {new_s}:{new_id}")
            ids[cid] = new_id
        concepts = []
        for c in sdef["concepts"]:
            cid = str(c["id"])
            labels = {"sk": c["sk"], "en": c["en"]}
            labels.update(c.get("labels") or {})
            item = {"id": ids[cid]}
            if c.get("broader"):
                item["broader"] = ids[c["broader"]]
            item["labels"] = labels
            if c.get("note"):
                item["note"] = {"sk": c["note"]}
            for k in ("isced", "lang"):
                if k in c:
                    item[k] = c[k]
            if c.get("kw"):
                item["keywords"] = c["kw"]
            if new_s == "resource-type" and ids[cid] in TYPE_KEYWORDS:
                item["keywords"] = TYPE_KEYWORDS[ids[cid]]
            concepts.append(item)
            idmap.append((old_s, cid, new_s, ids[cid]))
        sch = {"labels": sdef["title"]}
        if sdef.get("note"):
            sch["note"] = {"sk": sdef["note"].strip()}
        sch["concepts"] = concepts
        out_schemes[new_s] = sch
    tax = {"version": 2, "reference_language": "sk", "core_languages": ["sk", "en", "cs", "ru", "zh"],
           "languages": ["sk", "en", "cs", "ru", "zh", "de", "pl", "hu"], "schemes": out_schemes}
    head = ("# ATENA taxonomy (SKOS-like). Licence: CC BY 4.0.\n"
            "# Stable English IDs; every language is only a label. Keywords drive automatic SUGGESTIONS,\n"
            "# matched only against documents in the same language. Generated by tools/migrate-data from v0.1.\n")
    (ROOT / "data" / "taxonomy.yaml").write_text(
        head + yaml.safe_dump(tax, allow_unicode=True, sort_keys=False, width=140), encoding="utf-8")

    src = yaml.safe_load((OLD / "zdroje.yaml").read_text(encoding="utf-8"))
    out = []
    for s in src["sources"]:
        e = {"id": s["id"], "name": s["name"], "url": s["url"], "country": s["country"],
             "languages": s["languages"], "authority": s["authority"], "priority": s["priority"],
             "connector": s["connector"], "interval_s": interval(s.get("frequency")),
             "review": STATUS[s["status"]]}
        if s.get("feed"):
            e["feed"] = s["feed"]
        if s.get("note"):
            e["note"] = {"sk": s["note"]}
        out.append(e)
    (ROOT / "data" / "sources.yaml").write_text(
        "# ATENA source register. Licence: CC0 1.0. Reviewed 24 Oct 2026 (see docs/sk and the review workbook).\n"
        + yaml.safe_dump({"version": 2, "sources": out}, allow_unicode=True, sort_keys=False, width=140),
        encoding="utf-8")

    with open(ROOT / "data" / "id-map.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["old_scheme", "old_id", "scheme", "id"])
        w.writerows(idmap)
    print(f"concepts {len(idmap)}, sources {len(out)}")


if __name__ == "__main__":
    main()
