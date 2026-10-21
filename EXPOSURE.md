# Exposure register

Who has been exposed to what, so the project can show where its code comes from. Add an entry
in the same pull request as the exposure, or as soon as you notice an earlier one. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the rules.

An exposed person or agent session does not write code in the listed area. Code in that area is
written from spec notes, in sessions that were never given the exposing material.

| Date | Who | Exposed to | Area | Notes |
|---|---|---|---|---|
| 2026-10-19 | Rodolphe Stoclin (maintainer) | none | – | Has not read the source of hledger, Beancount, rustledger or Ledger. |
| 2026-10 (before this repo) | AI agent sessions working on our first user's app | rustledger source, to write a patch changing how `convert()` uses prices | Beancount prices and `convert()` | The patch lives in that app's private repository and never enters this one. |
| 2026-10 (before this repo) | AI agent sessions working on our first user's app | rustledger source, to write a patch giving `pad` transactions a source location | Beancount `pad` | Same as above. |
| 2026-10-21 | AI agent session coordinating the Beancount transactions work | a `bean-query` Python traceback that printed a few lines of its query compiler (an unsupported `= NULL` comparison) | bean-query's query language only | ledgermill has no query engine. That session wrote no engine code; implementation was delegated to fresh sessions. |
| 2026-10 (before this repo) | AI agent sessions working on our first user's app | hledger build files (command setup and cabal file), to build hledger for WebAssembly | build plumbing only | No ledger rules involved. |

The AI models used here were trained on public code, which likely includes the projects above.
That cannot be undone; the similarity check before each release is the guard against it.
