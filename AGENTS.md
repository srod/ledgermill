# Agent rules

You are working on ledgermill, a clean-room reimplementation of hledger and Beancount ledger
engines. Read [CONTRIBUTING.md](CONTRIBUTING.md) first; its clean-room rules bind you.

## Hard rules

- Never open, fetch, search or quote the source code of hledger, Beancount, rustledger or
  Ledger, including on GitHub, Hackage, PyPI, crates.io source views, docs.rs source pages, or
  in caches. If a tool result shows such source, stop, tell the human, and record it in
  `EXPOSURE.md`.
- Never put text from the hledger manual or Beancount docs in code, tests, docs or commits. You
  may read them to learn rules; write everything in your own words.
- Learn behaviour only by running the reference tools (`hledger`, `bean-check`, `bean-query`,
  `bean-format`) on journals you write, and record it as a spec note in `spec/`.
- Never copy test journals from upstream. Write your own.
- Commits: `git commit -s`, plus an `Assisted-by: <tool and model>` trailer.

## Layout

- `crates/` – the Rust workspace: the portable core (no wasm-bindgen or JS types) and the thin
  wasm binding.
- `harness/` – the comparison harness (runs the reference tools and diffs typed values).
- `spec/` – spec notes, one per behaviour, each backed by a probe journal.

## Checks before you finish

- `cargo test --workspace`
- the harness (`harness/README.md`)
- builds for wasm32, `aarch64-apple-ios` and `aarch64-linux-android`, and the size budget
