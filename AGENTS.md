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
- Commits: `git commit -s`, plus an `Assisted-by: <tool> (<model>, <vendor>)` trailer, e.g.
  `Assisted-by: pi (Claude, Anthropic)`.

## Layout

- `crates/` – the Rust workspace: the portable core (no wasm-bindgen or JS types) and the thin
  wasm binding.
- `harness/` – the comparison harness (runs the reference tools and diffs typed values).
- `spec/` – spec notes, one per behaviour, each backed by a probe journal.

## Rust style

We follow a few rules from the
[Apollo Rust best-practices handbook](https://github.com/apollographql/rust-best-practices);
the mechanical ones are enforced by `[workspace.lints]` in `Cargo.toml`.

- **No `unwrap`/`expect` outside tests.** The core runs inside browsers and apps, where a panic
  is a crash. Make impossible states unrepresentable (a field that is always set is not an
  `Option`), or turn the problem into a finding.
- **Borrow, don't clone.** Accessors return slices (`&[T]`); clone only when ownership is truly
  needed, as late as possible.
- **No silent truncation.** Convert with `try_from`, not `as`.
- **Fix lints, don't silence them.** If a lint must be overridden, use `#[expect(...)]` with a
  one-line reason, never a bare `#[allow(...)]`. Integration test files may allow
  `unwrap_used`/`expect_used` at the top (a panic there is a failed test).
- **Comments say why**, not what. `TODO`s name an issue: `// TODO(#12): ...`.
- **Tests**: through the public interface only, named after the behaviour they check. One test
  per journal comparing the whole result is fine.
- **Before 0.1.0** every public item gets a doc comment (`missing_docs` will be turned on).

## Checks before you finish

- `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo deny check` (licences: permissive only, no GPL/LGPL/AGPL; advisories; sources),
  `cargo machete` (unused dependencies) and `typos` (spelling). Install: `cargo install
  --locked cargo-deny cargo-machete typos-cli`. A new dependency's licence must be in
  `deny.toml`'s allow-list; never widen the list to admit a copyleft licence.
- `cargo test --workspace`
- the harness (`harness/README.md`)
- builds for wasm32, `aarch64-apple-ios` and `aarch64-linux-android`, and the size budget
