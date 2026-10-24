package main

import (
	"encoding/json"
	"fmt"
	"math"
	"os"
	"strings"
	"time"
	"unicode"
)

type Doc struct{ Lang, Title, Body string }
type tok struct {
	s      string
	prefix bool
}

// Go nemá v štandardnej knižnici NFKD – mapovanie diakritiky pre jazyky korpusu
var fmap = map[rune]string{'á': "a", 'ä': "a", 'à': "a", 'â': "a", 'ą': "a", 'č': "c", 'ć': "c", 'ç': "c", 'ď': "d", 'é': "e", 'ě': "e", 'è': "e", 'ê': "e", 'ë': "e", 'ę': "e", 'í': "i", 'î': "i", 'ï': "i", 'ĺ': "l", 'ľ': "l", 'ň': "n", 'ń': "n", 'ñ': "n", 'ó': "o", 'ô': "o", 'ö': "o", 'ő': "o", 'ò': "o", 'ŕ': "r", 'ř': "r", 'š': "s", 'ś': "s", 'ť': "t", 'ú': "u", 'ů': "u", 'ü': "u", 'ű': "u", 'ù': "u", 'û': "u", 'ý': "y", 'ž': "z", 'ź': "z", 'ż': "z", 'ß': "ss"}

func fold(s string) string {
	var b strings.Builder
	for _, r := range strings.ToLower(s) {
		if m, ok := fmap[r]; ok {
			b.WriteString(m)
		} else {
			b.WriteRune(r)
		}
	}
	return b.String()
}

func alnum(r rune) bool { return unicode.IsLetter(r) || unicode.IsDigit(r) }

func tokens(s string) []string {
	cs := []rune(fold(s))
	out := []string{}
	for i := 0; i < len(cs); {
		if alnum(cs[i]) {
			st := i
			for i < len(cs) && alnum(cs[i]) {
				i++
			}
			for i+1 < len(cs) && (cs[i] == '-' || cs[i] == '\'') && alnum(cs[i+1]) {
				i++
				for i < len(cs) && alnum(cs[i]) {
					i++
				}
			}
			out = append(out, string(cs[st:i]))
		} else {
			i++
		}
	}
	return out
}

func match(toks []string, ph []tok) int {
	n, m := len(toks), len(ph)
	if m == 0 || n < m {
		return 0
	}
	c := 0
	for i := 0; i <= n-m; i++ {
		ok := true
		for j, k := range ph {
			t := toks[i+j]
			if k.prefix {
				if !strings.HasPrefix(t, k.s) {
					ok = false
					break
				}
			} else if t != k.s {
				ok = false
				break
			}
		}
		if ok {
			c++
		}
	}
	return c
}

func main() {
	var docs []Doc
	b, _ := os.ReadFile(os.Args[1])
	json.Unmarshal(b, &docs)
	var kw struct{ Kw map[string][][]json.RawMessage }
	b, _ = os.ReadFile(os.Args[2])
	json.Unmarshal(b, &kw)
	prep := map[string][][][]tok{}
	for l, cs := range kw.Kw {
		for _, c := range cs {
			var ph [][]string
			json.Unmarshal(c[2], &ph)
			var cp [][]tok
			for _, p := range ph {
				var tp []tok
				for _, t := range p {
					r := []rune(t)
					if len(r) >= 4 {
						if len(r) >= 7 {
							r = r[:len(r)-2]
						}
						tp = append(tp, tok{string(r), true})
					} else {
						tp = append(tp, tok{t, false})
					}
				}
				cp = append(cp, tp)
			}
			prep[l] = append(prep[l], cp)
		}
	}
	t0 := time.Now()
	n, chk := 0, 0.0
	for _, d := range docs {
		r := []rune(d.Body)
		h := r
		if len(h) > 1500 {
			h = r[:1500]
		}
		var rest []rune
		if len(r) > 1500 {
			e := len(r)
			if e > 20000 {
				e = 20000
			}
			rest = r[1500:e]
		}
		tt, ht, rt := tokens(d.Title), tokens(string(h)), tokens(string(rest))
		for _, phrases := range prep[d.Lang] {
			w := 0
			for _, p := range phrases {
				hits := 3*match(tt, p) + 2*match(ht, p) + match(rt, p)
				if hits > 6 {
					hits = 6
				}
				w += hits
			}
			if w > 0 {
				s := 1 - math.Exp(-float64(w)/3)
				if s >= 0.45 {
					n++
					chk += math.Round(s*1000) / 1000
				}
			}
		}
	}
	dt := time.Since(t0).Seconds()
	fmt.Printf("go: %d docs in %.2f s = %.3f ms/doc; suggestions=%d checksum=%.3f\n", len(docs), dt, dt/float64(len(docs))*1000, n, chk)
}
