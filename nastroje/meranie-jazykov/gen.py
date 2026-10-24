import json, random, sys
sys.path.insert(0, '/home/claude/atena')
from atena.taxonomy import default_taxonomy
from atena.textnorm import fold
tax = default_taxonomy()
# kľúčové slová: jazyk -> [[scheme, id, [frázy ako zoznam tokenov]]]
kw = {}
for (s, cid), c in tax.concepts.items():
    if s == 'typ-obsahu' or not c.keywords: continue
    for lang, phrases in c.keywords.items():
        if lang == 'zh': continue
        kw.setdefault(lang, []).append([s, cid, phrases])
random.seed(42)
langs = ['sk', 'cs', 'en', 'de']
pool = {l: [] for l in langs}
for l in langs:
    for _, _, ph in kw[l]:
        for p in ph: pool[l].append(' '.join(p))
filler = {
 'sk': 'žiaci učiteľ škola hodina vyučovanie práca text úloha cieľ výsledok rodičia trieda materiál obsah ročník predmet program skupina otázka odpoveď príklad metóda čas deň rok'.split(),
 'cs': 'žáci učitel škola hodina výuka práce text úkol cíl výsledek rodiče třída materiál obsah ročník předmět program skupina otázka odpověď příklad čas den rok'.split(),
 'en': 'students teacher school lesson learning work text task goal result parents class material content grade subject program group question answer example time day year'.split(),
 'de': 'Schüler Lehrer Schule Stunde Unterricht Arbeit Text Aufgabe Ziel Ergebnis Eltern Klasse Material Inhalt Jahrgang Fach Programm Gruppe Frage Antwort Beispiel Zeit Tag Jahr'.split(),
}
def sent(l):
    words = [random.choice(filler[l]) for _ in range(random.randint(8, 18))]
    if random.random() < 0.35: words.insert(random.randint(0, len(words)), random.choice(pool[l]))
    return ' '.join(words).capitalize() + '.'
docs = []
for i in range(4000):
    l = langs[i % 4]
    title = ' '.join(random.choice(filler[l]) for _ in range(5)) + (' ' + random.choice(pool[l]) if random.random() < .5 else '')
    body = ' '.join(sent(l) for _ in range(random.randint(30, 90)))
    docs.append({'lang': l, 'title': title, 'body': body})
json.dump({'kw': kw}, open('kw.json', 'w'), ensure_ascii=False)
json.dump(docs, open('docs.json', 'w'), ensure_ascii=False)
print('docs', len(docs), 'avg chars', sum(len(d['body']) for d in docs)//len(docs))
