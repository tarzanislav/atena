use serde::Deserialize;
use std::collections::HashMap;
use std::time::Instant;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

#[derive(Deserialize)]
struct Doc { lang: String, title: String, body: String }
#[derive(Deserialize)]
struct Kw { kw: HashMap<String, Vec<(String, String, Vec<Vec<String>>)>> }

fn fold(s: &str) -> String {
    let lower: String = s.chars().flat_map(|c| c.to_lowercase()).collect::<String>().replace('ß', "ss");
    lower.nfkd().filter(|c| !is_combining_mark(*c)).collect()
}

// rovnaké pravidlo ako Python: [^\W_]+(?:[-'][^\W_]+)*
fn tokens(s: &str) -> Vec<String> {
    let f = fold(s);
    let cs: Vec<char> = f.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_alphanumeric() {
            let start = i;
            while i < cs.len() && cs[i].is_alphanumeric() { i += 1; }
            while i + 1 < cs.len() && (cs[i] == '-' || cs[i] == '\'') && cs[i + 1].is_alphanumeric() {
                i += 1;
                while i < cs.len() && cs[i].is_alphanumeric() { i += 1; }
            }
            out.push(cs[start..i].iter().collect());
        } else { i += 1; }
    }
    out
}

fn match_phrase(toks: &[String], phrase: &[(String, bool)]) -> u32 {
    let (n, m) = (toks.len(), phrase.len());
    if m == 0 || n < m { return 0; }
    let mut count = 0;
    for i in 0..=(n - m) {
        let mut ok = true;
        for (j, (kw, prefix)) in phrase.iter().enumerate() {
            let tk = &toks[i + j];
            if *prefix { if !tk.starts_with(kw.as_str()) { ok = false; break; } }
            else if tk != kw { ok = false; break; }
        }
        if ok { count += 1; }
    }
    count
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let docs: Vec<Doc> = serde_json::from_str(&std::fs::read_to_string(&a[1]).unwrap()).unwrap();
    let kw: Kw = serde_json::from_str(&std::fs::read_to_string(&a[2]).unwrap()).unwrap();
    // predpríprava: stem (kw bez posledných 2 znakov od 7 znakov), prefix od 4 znakov
    let prep: HashMap<String, Vec<Vec<Vec<(String, bool)>>>> = kw.kw.iter().map(|(l, cs)| {
        (l.clone(), cs.iter().map(|(_, _, ph)| ph.iter().map(|p| p.iter().map(|t| {
            let n = t.chars().count();
            if n >= 4 { let stem: String = if n >= 7 { t.chars().take(n - 2).collect() } else { t.clone() }; (stem, true) }
            else { (t.clone(), false) }
        }).collect()).collect()).collect())
    }).collect();
    let t0 = Instant::now();
    let (mut n, mut chk) = (0u32, 0f64);
    for d in &docs {
        let chars: Vec<char> = d.body.chars().collect();
        let head: String = chars.iter().take(1500).collect();
        let rest: String = chars.iter().skip(1500).take(18500).collect();
        let (tt, ht, rt) = (tokens(&d.title), tokens(&head), tokens(&rest));
        if let Some(concepts) = prep.get(&d.lang) {
            for phrases in concepts {
                let mut w = 0u32;
                for p in phrases {
                    let hits = 3 * match_phrase(&tt, p) + 2 * match_phrase(&ht, p) + match_phrase(&rt, p);
                    w += hits.min(6);
                }
                if w > 0 {
                    let score = 1.0 - (-(w as f64) / 3.0).exp();
                    if score >= 0.45 { n += 1; chk += (score * 1000.0).round() / 1000.0; }
                }
            }
        }
    }
    let dt = t0.elapsed().as_secs_f64();
    println!("rust: {} docs in {:.2} s = {:.3} ms/doc; suggestions={} checksum={:.3}", docs.len(), dt, dt / docs.len() as f64 * 1000.0, n, chk);
}
