# Rights review of the sources

*Slovenčina: [sk/PRAVA-ZDROJOV.md](sk/PRAVA-ZDROJOV.md)*

Every source in `data/sources.yaml` has a `licence`, a `terms` value and a `rights` record: the day of
the review, the page that was read (terms of use, legal notice, licence page – for official texts the
law that frees them) and what it says. The register refuses a licence or terms other than the defaults
without such a record. The first review covered all 186 sources on **24 October 2026**. It is not legal
advice; repeat it at least once a year, and whenever a publisher changes its terms.

## How a finding becomes `licence` and `terms`
| What the publisher states | `licence` | `terms` | ATENA |
|---|---|---|---|
| An open licence covering the site's own content (CC BY, CC BY-SA, CC0, OGL, Licence Ouverte) | the licence | `allowed` | text kept, shown in full with attribution |
| Laws, official documents, US federal works | `official` | `allowed` | text kept, shown in full |
| No terms at all, or only a copyright notice ("all rights reserved") | `copyrighted` | `allowed` | text kept for change detection and classification where both laws allow mining; never shown in full |
| Reuse permitted, also only for non-commercial purposes | `copyrighted` | `allowed` | as above |
| Terms forbid robots, scrapers, crawling or indexing, or allow access only for personal use | `copyrighted` | `forbidden` | not crawled at all |
| Terms reserve text and data mining or AI use | `copyrighted` | `reserved` | metadata only |
| Terms could not be read (robots.txt refused our reading tool) or contradict each other | `copyrighted` | `unknown` | metadata only |

Rules behind the table:
* **Silence and plain copyright notices are not a reservation.** Art. 4(3) of Directive (EU)
  2019/790 lets a rights holder exclude mining only by reserving it expressly; a general "all rights
  reserved" does not do that. The project author decided on 24 Oct 2026 to treat both as `allowed`.
  Machine-readable reservations still win: TDMRep, and robots.txt groups refusing AI crawlers.
* **Non-commercial permissions do not make content open.** ATENA must stay usable commercially, so
  only licences in `OPEN_LICENCES` count as open; CC BY-NC and similar remain `copyrighted`.
* **Repositories and mixed sites stay `copyrighted`** even when their own pages are open: they host
  third-party works under other licences (Zenodo, Scientix, the European School Education Platform,
  Better Internet for Kids, Oak National Academy).
* **Terms that limit access itself are a ban.** "Only for your personal use", "may not include the
  content in any database or retrieval system" and "non-commercial use of the web services only" are
  treated as `forbidden`: what is uncertain is not done.

## Result (24 Oct 2026)
| | Sources |
|---|---|
| `allowed` | 141 |
| `forbidden` – not crawled | 28 |
| `reserved` – metadata only | 1 (NRICH) |
| `unknown` – metadata only | 16 |
| shown in full (official or open) | 22 |

**Not crawled** (terms forbid automated access): UNESCO (GL01–GL04, GL13), YouTube (N39), British
Council TeachingEnglish (UK05), Reading Rockets (US06, US07), Edutopia (US08), PBS LearningMedia
(US09), IRIS Center (US10), CAST UDL Guidelines (US11), CASEL (US13), Common Sense Education (US14),
ISTE+ASCD (US19), GeoGebra (TO02), Scratch Foundation (TO03), iCivics (TO06), OER Commons (TO07),
American Montessori Society (AL02), HighScope (AL10), The Philosophy Foundation (AL17), Cult of
Pedagogy (AL24), klicksafe (N11), Evidence for ESSA (N22), CommonLit (N24), CORE (N33).

**Still to verify** (our reading tool was refused by robots.txt, or the terms contradict each other):
Deutscher Bildungsserver and Elixier (DE01, DE02), Fachportal Pädagogik (DE04), Oktatási Hivatal and
Nemzeti Köznevelési Portál (HU01–HU03), edu.gov.ru (RU01, RU02), waldorf.sk (AL04), Reggio Children
(AL09), CKE (N07), Eduthek (N13), NCCA (N20), HundrED (N21), e-Koolikott (N30), DOAJ (N32). Their
pages may still be fetched where robots.txt lets ATENA in; only metadata is kept.

Video links are taken from institutions' own pages; ATENA never visits YouTube (decision of the
project author, 24 Oct 2026).
