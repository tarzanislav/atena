from atena.taxonomy import default_taxonomy, load_taxonomy, suggest


def ids(sugs, scheme):
    return {s.concept_id for s in sugs if s.scheme == scheme}


def test_loads_and_is_consistent():
    t = default_taxonomy()
    assert len(t.concepts) > 250
    # všetkých 6 skupín škôl zo zadania je prepínateľných
    top = {c.id for c in t.scheme_concepts("druh-skoly") if c.broader is None}
    assert {"predprimarne", "zakladne", "stredne", "svvp", "spojena", "celozivotne"} <= top
    # všetky body zadania 1a–1x majú pojem
    for cid in ["trendy", "zmeny", "aktivizacia", "ukazky-hodin", "metody-prax", "strategie-prax", "neformalne",
                "digitalne", "inkluzia", "kurikularna-reforma", "vzdelavacie-oblasti", "citatelska", "ai",
                "urovne-porozumenia", "kriticke-myslenie", "fazy-procesu", "legislativa", "terminologia", "hry"]:
        assert t.get("tema", cid), cid
    assert len(t.scheme_concepts("gramotnost")) >= 10   # bod 1r


def test_duplicate_detection():
    import pytest
    with pytest.raises(ValueError):
        load_taxonomy("schemes: {a: {concepts: [{id: x, sk: X}, {id: x, sk: Y}]}}")


def test_slovak_reading_kindergarten():
    s = suggest("Rozvoj čitateľskej gramotnosti v materskej škole",
                "Metodika ponúka aktivity na čítanie s porozumením pre deti v materskej škole. "
                "Deti pracujú v skupinách a učiteľ využíva aktivizačné metódy.", "sk")
    assert "citatelska" in ids(s, "tema")
    assert "ms" in ids(s, "druh-skoly")
    assert "metodika" in ids(s, "typ-obsahu")


def test_other_languages():
    assert "ai" in ids(suggest("Künstliche Intelligenz im Unterricht", "Wie Lehrkräfte künstliche Intelligenz nutzen.", "de"), "tema")
    assert "citatelska" in ids(suggest("Читательская грамотность", "Задания на читательскую грамотность для 7 класса.", "ru"), "tema")
    assert "kriticke-myslenie" in ids(suggest("Kritické myšlení ve výuce", "Rozvoj kritického myšlení žáků.", "cs"), "tema")
    assert "ai" in ids(suggest("人工智能教育", "人工智能进课堂。人工智能教育指南。", "zh"), "tema")


def test_language_isolation():
    # slovenské „sen“ (snívanie) nesmie spustiť anglickú skratku SEN (špeciálne potreby)
    s = suggest("Môj sen", "Mal som krásny sen o lete a sen o mori.", "sk")
    assert "inkluzia" not in ids(s, "tema")


def test_legislation_type():
    s = suggest("Vyhláška č. 123/2026 Z. z.", "Ministerstvo ustanovuje podľa zákona č. 245/2008 Z. z.", "sk")
    assert "predpis" in ids(s, "typ-obsahu")
