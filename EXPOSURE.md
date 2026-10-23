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
| 2026-10-22 | AI agent session working on inferred lot costs and prices | two `bean-check`/`bean-query` tracebacks that printed lines of Beancount's booking code | Beancount cost and price booking | The session reverted the engine code it had begun in this area, never committed it, and wrote only probes and the spec note `spec/beancount/inferred-cost-and-price.md`, from tool output. A fresh session that never saw the tracebacks wrote the engine code from that note. Don't run the journals that crash the tools: a cost with a price that has no number, or zero units with a price left to work out. |
| 2026-10-22 | AI agent session writing the pad/balance spec notes | the first line of a `bean-check` error that embeds a Python traceback (loading a `plugin` module that does not exist) | Beancount plugin loading only | The rest of the output was filtered out unread and the journal was not run again. `plugin` stays refused. The session wrote only probes and spec notes, no engine code. |
| 2026-10-22 | AI agent session fixing zero-amount postings (pad/balance branch) | a `bean-query` Python traceback with about twenty lines of Beancount's loader and booking code (a division by zero for zero units with a total cost `0 BTC {{10 EUR}}`) | Beancount booking, zero-unit costs | The traceback was not read for rules; the journal was deleted and is not to be run again. The session wrote the probes and the rule in `spec/beancount/check-errors.md` from `bean-check` output on other shapes. Its engine code for this rule was dropped before merge and rewritten from that note by a fresh session that never saw the traceback. |
| 2026-10-23 | AI agent session writing the Beancount prices, `convert()` and holdings spec notes | the first three lines of a `bean-query` Python traceback: the path of the `bean-query` launcher script and its `sys.exit(main())` line (a `SELECT ... currency ... FROM entries WHERE type = 'price'` query, a column that table does not have, made it print one) | none: the launcher stub of the tool, no Beancount or beanquery source and no rule | The rest of the output was not read; the column that made it fail (`currency`) is not queried again, and the scratch journal was not committed. The harness still asks that table for `date, lineno, filename` only (`PRICE_LINES_QUERY`), the session's own query: the three lines it saw name no table, column or rule. From then on the session ran every reference command through a wrapper that hides the output when it holds a traceback. |
| 2026-10 (before this repo) | AI agent sessions working on our first user's app | hledger build files (command setup and cabal file), to build hledger for WebAssembly | build plumbing only | No ledger rules involved. |

The AI models used here were trained on public code, which likely includes the projects above.
That cannot be undone; the similarity check before each release is the guard against it.
