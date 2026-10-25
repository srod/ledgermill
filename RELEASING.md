# Releasing

The procedure for a release, in order. Nothing is published until step 4, and nothing is pushed
or published by an agent: the owner runs steps 4 and 5.

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
   1. `ledgermill-beancount-parser`, from `srod/beancount-parser-lima` at its pinned commit;
   2. `ledgermill-beancount-booking`, from `srod/limabean-booking` at its pinned commit;
   3. `cargo publish -p ledgermill`;
   4. `cargo publish -p ledgermill-wasm`;
   5. `crates/ledgermill-wasm/build.sh && npm publish ./pkg`.

   Try each `cargo publish` with `--dry-run` first. (`cargo publish -p ledgermill-wasm --dry-run`
   only works once `ledgermill` is on crates.io.)
5. **Tag and release.** `git tag v0.1.0` on the released commit, push the tag, and create the
   GitHub release for `v0.1.0` with the CHANGELOG entry as its text.
