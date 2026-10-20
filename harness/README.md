# Comparison harness

Runs `hledger` (1.52.4) and our wasm on the same journals and diffs **typed values**: the
transactions each one produces, not printed text. hledger is a reference tool here: we run it and
never read its source.

## Set up

You need [Bun](https://bun.sh), `hledger` 1.52.4 on `PATH`, Rust (the toolchain in
`rust-toolchain.toml`) and `wasm-bindgen-cli` 0.2.129:

```sh
cargo install --locked wasm-bindgen-cli@0.2.129
cargo build --release --target wasm32-unknown-unknown -p ledgermill-wasm
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/ledgermill_wasm.wasm
```

Rebuild both steps after changing the Rust code: the harness loads `pkg/`.

## Run

```sh
bun harness/compare.ts            # every corpus journal and pair, one line each
bun harness/compare.ts --quiet    # only the lines that are not PASS
bun harness/fuzz.ts [N] [SEED]    # N random journals (default 500); CI runs 1,000 with a new seed each run
bun harness/real.ts FILE...       # your own journals (several files: each alone, then all together)
bun test harness/lib.test.ts      # the harness's own checks
```

## Read the results

Each case ends in one verdict:

- **PASS**: both give the same transactions, or both fail (hledger exits non-zero and we report an
  error finding).
- **SKIP**: we report an `unsupported-syntax` finding (the line after `SKIP` says which construct):
  not built yet.
- **FAIL**: anything else. The line gives the first difference as a path, for example
  `[2].postings[0].amounts[0].quantity: hledger="3.51" ours="3.50"`, or says that hledger fails and
  we report no error, or that we report `unbalanced-transaction` while hledger accepts. A FAIL is a
  bug in the engine or in the harness; the exit code is 1.

The last line is `N pass, N fail, N skip`.

How a case is compared: hledger's `print -O json` is projected onto our `Transaction` shape (date,
status, description, comment trimmed, source file and line, postings with account, kind, comment
trimmed and amounts, each with its price and lot). Quantities are exact decimal strings built
with `BigInt` from hledger's `decimalMantissa` and `decimalPlaces`, so places and big numbers
survive (very large mantissas come in floating notation and are read back exactly).
`source.file` is compared by basename.

The fuzzer (`fuzz.ts`) writes random, mostly valid journals covering number forms and marks,
directives, prices, lots, elided amounts, inferred prices, conversion postings, virtual postings
and big numbers, about a third of them invalid on purpose (they must fail in both). Same seed,
same journals; a FAIL prints the journal.

## Corpus

All journals are written by us and safe to publish.

- `journals/prototype/`: probe journals from our earlier prototype.
- `journals/app/`: test journals of our first user, a personal finance app, published with its
  author's approval.
- `../spec/hledger/probes/`: the probe journals behind each spec note.
- Multi-file cases: the `pairs` and `fixturePairs` lists in `compare.ts` (files are loaded in the
  order listed).

Never commit private journals: `real.ts` exists so you can check them locally.
