# Contributing to ATENA

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
