import json, sys, time
sys.path.insert(0, '/home/claude/atena')
from atena.taxonomy import default_taxonomy, suggest
tax = default_taxonomy()
schemes = [s for s in tax.schemes if s != 'typ-obsahu']
docs = json.load(open(sys.argv[1]))
t0 = time.perf_counter()
n = 0; chk = 0.0
for d in docs:
    for s in suggest(d['title'], d['body'], d['lang'], tax, schemes=schemes):
        n += 1; chk += s.score
dt = time.perf_counter() - t0
print(f"python: {len(docs)} docs in {dt:.2f} s = {dt/len(docs)*1000:.2f} ms/doc; suggestions={n} checksum={chk:.3f}")
