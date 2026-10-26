# Releasing

The procedure for a release, in order. Nothing is published until step 4. Steps 4 and 5 need the
owner's explicit yes for that release; an agent may run them once it has it.

## Access needed

- A **crates.io** API token that may publish `ledgermill`, `ledgermill-wasm`,
  `ledgermill-beancount-parser` and `ledgermill-beancount-booking` (`cargo login`).
- **npm** membership of the `@ledgermill` organisation with publish rights (`npm login`; the
  package is public, `publishConfig.access` is set).
- Push rights on `srod/ledgermill` for the tag and the GitHub release.

## Procedure

1. **Release gate on GitHub.** Run the `release-gate.yml` workflow (Actions → Release gate → Run
   workflow, on the commit to release) and wait for it to be green.
2. **Owner's local runs.** These use private journals and a phone, so they never run in CI:
   - `bun harness/real.ts JOURNAL...` on the owner's own journals (see `harness/README.md`);
   - the device bench, `harness/device-bench`, on the owner's phone.
3. **Fill in the gate.** In [CHANGELOG.md](CHANGELOG.md), replace every `TBD at release` of the
   "Release gate" section with the results of steps 1 and 2, set the heading to
   `0.1.0 – YYYY-MM-DD`, and commit. Never write a result that was not run. Check that
   `[workspace.package] version` in `Cargo.toml` and `version` in
   `crates/ledgermill-wasm/package.json` are the release version (`build.sh` refuses a
   mismatch).
4. **Publish, in this order.** Each crate must be visible on crates.io before the next one
   publishes.
   1. `ledgermill-beancount-parser`: `cargo publish` in a checkout of `srod/beancount-parser-lima`
      at the commit `crates/ledgermill/Cargo.toml` pins;
   2. `ledgermill-beancount-booking`: the same in `srod/limabean-booking`.

      Both forks are on crates.io since 2026-10-26 (parser 0.16.3, booking 0.10.5), so these two
      are only needed again when a fork changes. A published version can't be replaced: bump the
      fork's version, publish it, then pin the new commit and its exact version (`=X.Y.Z`) in
      `crates/ledgermill/Cargo.toml`;
   3. `cargo publish -p ledgermill`;
   4. `cargo publish -p ledgermill-wasm`;
   5. `crates/ledgermill-wasm/build.sh && npm publish ./pkg`.

   Try each `cargo publish` with `--dry-run` first. A dry run resolves the dependencies from
   crates.io, so `-p ledgermill` only works once both forks are published, and `-p ledgermill-wasm`
   once `ledgermill` is.
5. **Tag and release.** `git tag v0.1.0` on the released commit, push the tag, and create the
   GitHub release for `v0.1.0` with the CHANGELOG entry as its text.
