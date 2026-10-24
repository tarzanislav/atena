import fs from 'fs';
const docs = JSON.parse(fs.readFileSync(process.argv[2])), kw = JSON.parse(fs.readFileSync(process.argv[3])).kw;
const fold = s => s.toLowerCase().replaceAll('ß', 'ss').normalize('NFKD').replace(/\p{M}/gu, '');
const RX = /[\p{L}\p{N}]+(?:[-'][\p{L}\p{N}]+)*/gu;
const tokens = s => fold(s).match(RX) || [];
const prep = {};
for (const [l, cs] of Object.entries(kw)) prep[l] = cs.map(([, , ph]) => ph.map(p => p.map(t => { const c = [...t]; return c.length >= 4 ? [c.length >= 7 ? c.slice(0, -2).join('') : t, true] : [t, false]; })));
const match = (toks, ph) => { const n = toks.length, m = ph.length; if (!m || n < m) return 0; let c = 0;
  for (let i = 0; i <= n - m; i++) { let ok = true; for (let j = 0; j < m; j++) { const [k, pre] = ph[j], t = toks[i + j]; if (pre ? !t.startsWith(k) : t !== k) { ok = false; break; } } if (ok) c++; } return c; };
const t0 = performance.now(); let n = 0, chk = 0;
for (const d of docs) { const ch = [...d.body]; const tt = tokens(d.title), ht = tokens(ch.slice(0, 1500).join('')), rt = tokens(ch.slice(1500, 20000).join(''));
  for (const phrases of prep[d.lang] || []) { let w = 0; for (const p of phrases) w += Math.min(3 * match(tt, p) + 2 * match(ht, p) + match(rt, p), 6);
    if (w) { const s = 1 - Math.exp(-w / 3); if (s >= 0.45) { n++; chk += Math.round(s * 1000) / 1000; } } } }
const dt = (performance.now() - t0) / 1000;
console.log(`node (JS/TypeScript): ${docs.length} docs in ${dt.toFixed(2)} s = ${(dt / docs.length * 1000).toFixed(3)} ms/doc; suggestions=${n} checksum=${chk.toFixed(3)}`);
