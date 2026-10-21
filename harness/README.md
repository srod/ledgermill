# Comparison harness

Runs `hledger` (1.52.4) or, for `.beancount` and `.bean` files, `bean-check` and `bean-query`
(Beancount 3.2.3, beanquery 0.2.0), and our wasm on the same journals, and diffs **typed values**:
the transactions each one produces, not printed text. These are reference tools here: we run them
and never read their source.

## Set up

You need [Bun](https://bun.sh), `hledger` 1.52.4 on `PATH`, Beancount (below), Rust (the
toolchain in `rust-toolchain.toml`) and `wasm-bindgen-cli` 0.2.129:

```sh
cargo install --locked wasm-bindgen-cli@0.2.129
cargo build --release --target wasm32-unknown-unknown -p ledgermill-wasm
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/ledgermill_wasm.wasm
```

Rebuild both steps after changing the Rust code: the harness loads `pkg/`.

Beancount: the harness runs `bean-check` and `bean-query` from `PATH`, and the versions matter
(the spec notes record what these ones print):

```sh
pipx install beancount==3.2.3
pipx inject beancount beanquery==0.2.0     # or: pip install beancount==3.2.3 beanquery==0.2.0
bean-query --version                       # beanquery 0.2.0, beancount 3.2.3
```

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

### Beancount

For a `.beancount` or `.bean` file (a single file; a Beancount pair is not run), the harness:

1. runs `bean-check FILE`. `bean-query` exits 0 even for an invalid journal, so `bean-check`
   decides whether the reference accepts the file (a non-zero exit means it does not);
2. if it accepts, runs `bean-query -f csv FILE` twice:
   - the transactions: `SELECT id, type, filename, lineno, date, flag, payee, narration FROM
     entries WHERE type = 'transaction'`;
   - the postings: `SELECT id, account, posting_flag, number, currency, cost_number,
     cost_currency, cost_date, cost_label, number(price), currency(price), filename, lineno FROM
     postings`;
3. decodes both CSVs (`decodeCsv`: CRLF rows, quoted fields with commas, doubled quotes and
   newlines) and joins them on `id`. A transaction without postings stays; several postings may
   share an `id` and a line (a sale over several lots). Decoding starts at the header, because
   `bean-query` prints errors in front of the CSV. The padding `bean-query` puts around numbers is
   trimmed. The result is projected onto our `Transaction` shape (flag, payee, narration, each
   posting's flag and line, its price, and its lot from the cost columns). A transaction's
   comment is always empty;
4. diffs that with what the wasm returns, like for hledger.

Verdicts are as above, with two Beancount cases:

- both reject: `bean-check` fails and we report an error finding (other than
  `unsupported-syntax`): PASS, `both fail`;
- **reference rejects (check() comes later)**: `bean-check` fails but we report no error. Our
  `check()` does not yet raise everything `bean-check` does, so this is a SKIP, counted apart
  (`N of them: reference rejects, check() comes later`) and not a FAIL;
- we report `unsupported-syntax`: SKIP, whether or not the reference accepts.

The summary prints a line per format when both are run. A FAIL is also: we report an error where
`bean-check` accepts, or the transactions differ.

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
- `journals/app/*.beancount`: the Beancount test journals of our first user (12).
- `../spec/beancount/probes/`: the Beancount probe journals behind each spec note
  (`probes/refused/` holds those ledgermill refuses; they are not run by `compare.ts`).
- Multi-file cases: the `pairs` and `fixturePairs` lists in `compare.ts` (files are loaded in the
  order listed).

Never commit private journals: `real.ts` exists so you can check them locally.
