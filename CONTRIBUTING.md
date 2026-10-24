# Contributing to ATENA

Every change must serve the four pillars of ATENA ([README.md](README.md)): seriousness, openness and
moving human knowledge forward; anonymity and respect for privacy; security and protection against
dangerous people, robots, code and processes; respect for the law.

* **Original code.** Code is written for ATENA. Ideas from other projects (including the author's own) are
  re-thought and re-written, never copied: clean, readable, simple, efficient.
* **Languages.** Code, identifiers, comments, commit messages and technical documentation in English.
  User-facing texts only in locale files (Project Fluent); Slovak is the reference locale, English the pivot,
  Czech, Russian and Chinese (Simplified) are core locales. User guide in Slovak (`docs/sk/`).
* **Licences.** Only dependencies whose licences allow private and commercial use without fees
  (`deny.toml`, `docs/LICENSES.md`). A new dependency needs a reason in the pull request.
* **Quality gates.** `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo deny check` – all must pass.
  Every behaviour has a test; the Python prototype's tests are the specification for the Rust core.
* **Honest data.** Unknown values stay unknown; automatic classification and translation are suggestions until reviewed.

## Lawful by design
The legality principle ([docs/LEGALITY.md](docs/LEGALITY.md), `atena-core/src/legality.rs`) stands above
every other rule: only open sources, nothing that infringes the law, misuses data or breaches data
protection in **any** country; the operator's law and each source country's law apply and the stricter
wins; what is uncertain is not done. Every decision on fetching, keeping and showing content goes through
`legality::permission`. A legal profile may be made stricter by anyone; making it more permissive needs
a legal review recorded in the pull request. Changes must also keep these true:
* never sign in, bypass access controls, ignore robots.txt, TDM reservations or terms of use;
* a new source starts as `licence: copyrighted`, `terms: unknown` until its licence and terms are verified;
  any other value needs a `rights` record – the date, the page that was read and what it says (the
  register refuses the entry otherwise; see [docs/SOURCE-RIGHTS.md](docs/SOURCE-RIGHTS.md));
* a source proposed by a user is never crawled before a person confirms it; automatic assessment
  may only make a proposal stricter (see [docs/ASSESSMENT.md](docs/ASSESSMENT.md));
* never store more personal data than the purpose needs – no contact details, no originals of web pages,
  no pages about people – and keep erasure (`Store::forget`) complete when adding tables.
