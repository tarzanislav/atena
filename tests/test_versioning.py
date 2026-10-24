from atena.versioning import compare, unified_diff

OLD = "Termín podania prihlášok je 20. 3. 2026.\n\nŠkola zverejní kritériá prijatia.\n\nKontakt: riaditeľ."


def test_whitespace_only_is_no_change():
    assert compare(OLD, OLD.replace(" ", "  ").replace("\n\n", "\n\n\n")).kind == "none"


def test_date_change_is_substantive():
    c = compare(OLD, OLD.replace("20. 3. 2026", "27. 3. 2026"))
    assert c.kind == "substantive" and c.numbers_changed


def test_small_typo_is_minor():
    assert compare(OLD, OLD.replace("riaditeľ", "riaditeľ.")).kind == "minor"


def test_new_paragraph_is_substantive():
    c = compare(OLD, OLD + "\n\nNovinka: od septembra platí nová povinnosť oznámiť zmenu zákonnému zástupcovi.")
    assert c.kind == "substantive" and c.added


def test_initial_and_diff_lines():
    assert compare(None, OLD).kind == "initial"
    lines = unified_diff(OLD, OLD.replace("20. 3.", "27. 3."))
    assert ("-", "Termín podania prihlášok je 20. 3. 2026.") in lines
