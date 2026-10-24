# ATENA design

Goal: the most beautiful and inviting design – based on the Windows 11 / Fluent look of the author's ŠkVP app,
and going further. Every visible text comes from locale files; the Slovak locale is the reference.

## Principles
1. **Calm pastel colours**, nothing saturated that tires the eyes; text contrast WCAG 2.2 AA.
2. **Larger type**: body 15–16 px, headings 30–34 px.
3. **Light and dark themes** plus "follow the system"; switching is instant.
4. **Each section has its own colour** – a bar in the menu, a stripe on its card, a tag on each item.
5. **Cards**: 10 px radius, subtle border, a gentle lift on hover; no gradient fills on surfaces.
6. **Left navigation** with icons and an accent bar on the selected item; nested items with a colour bar; collapses to an icon rail.
7. **Mobile**: navigation at the bottom, touch targets ≥ 44 px.
8. **Honest UI**: source authority (A1–D) on every item, "translation – suggestion", "ATENA suggestion", the original always one click away.
9. **Locale-proof layout**: no fixed-width labels; German and Hungarian strings are often 30–40 % longer than Slovak.

## Type
`'Segoe UI Variable Text', 'Segoe UI', 'Open Sans', system-ui` – Segoe UI on Windows as in ŠkVP, Open Sans
(OFL, by the same designer) elsewhere; headings `Segoe UI Variable Display`. Fonts must cover Latin Extended,
Cyrillic and CJK (Noto Sans SC for Chinese).

## Icons
Fluent UI System Icons (Microsoft, MIT) – the Windows 11 / ŠkVP style.

## Colour tokens (light / dark)
| Token | Light | Dark |
|---|---|---|
| background | #f3f3f3 | #202020 |
| navigation | #f9f9f9 | #1b1b1b |
| card | #ffffff | #2b2b2b |
| border | #e3e3e3 | #3b3b3b |
| text | #1b1b1b | #f3f3f3 |
| muted text | #5c5c5c | #b9b9b9 |
| accent | #0067c0 | #62b0f2 |
| lavender card (day) | #ece9f7 / #cdc6ea | #33304a / #5a5182 |
| green (deadline, A1–A2) | #dcefe2 / #aed6ba | #24382b / #3f6a4c |
| peach (notice) | #f9e9e2 / #ecc8b8 | #3d302b / #6b5047 |

## Section colours
| Section | Colour |
|---|---|
| Changes & legislation | #7b8fd6 |
| Curriculum & reforms | #5fb3a7 |
| Teaching in practice | #6dbb7f |
| Literacies & thinking | #e3a96b |
| Inclusion & support | #e48aa6 |
| Digital learning & AI | #a48ad6 |
| Pedagogical approaches | #e39a7f |
| Non-formal & lifelong learning | #d4b94e |
| School leadership | #8ea4ba |
| Research, data & trends | #6aa8d8 |
| World & inspiration | #9cc06a |

## Screens (plan)
Today · Search with facets · Section · Concept/method page (steps, lesson timeline, evidence, sources, related) ·
Document card (original, versions, change comparison) · Source register · Glossary · Calendar · My library ·
Profile (country packs, school types, languages) · Settings.
