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
                                  # (--save DIR writes the journals that fail to DIR, with their holdings questions)
bun harness/fuzz.ts --graph [N] [SEED]   # N random networks of prices, valued at random dates; CI runs 300
bun harness/real.ts FILE...       # your own journals (several files: each alone, then all together)
bun test harness/lib.test.ts      # the harness's own checks
```

## Read the results

Each case ends in one verdict:

- **PASS**: both give the same transactions, or both fail with the same first error (see below).
- **SKIP**: we report an `unsupported-syntax` finding (the line after `SKIP` says which construct):
  not built. No SKIP is expected on the hledger corpus any more, apart from the `--cost` refusal below: the
  hledger journals ledgermill refuses are kept apart (see Corpus). For Beancount, one journal is skipped, the one whose only error is a
  `document` file that does not exist: ledgermill does not look at files and warns instead.
- **FAIL**: anything else. The line gives the first difference as a path, for example
  `[2].postings[0].amounts[0].quantity: hledger="3.51" ours="3.50"`, or says that hledger fails and
  we report no error, or that we report `unbalanced-transaction` while hledger accepts. A FAIL is a
  bug in the engine or in the harness; the exit code is 1.

The last line is `N pass, N fail, N skip`, over everything compared (raw, postings, prices, `--cost`, `--value`, holdings and
Plainledger's reads; the lines before it count each of them: see "Postings and prices", "Cost and value" and "Holdings").

When hledger rejects a journal (exits non-zero), the harness takes the file and line of the **first error**
from the first line of its stderr and compares them with our **first error-severity finding**
(`check()` for hledger is in hledger's own order, so this is its first error):

- `hledger: Error: FILE:LINE:COL:` (a parse error, a failed assertion) and `FILE:FIRST-LAST:` (a
  transaction that does not balance; the line is the first one, the transaction's header). Same file
  (by basename) and same line: PASS, `both fail, same first error`. Another line or file: FAIL, with
  both. No error finding of ours: FAIL.
- Our first error is `unsupported-syntax`: SKIP (hledger fails, but we cannot say where).
- hledger names no position (the only one in the corpus is a posting date on an assignment, `Balance
  assignments and custom posting dates may not be combined.`, which shows the transaction and no file or
  line): `both fail`, the old rule, with no line compared.

A journal with `=` or `~` rules gets an `ignored-syntax` warning from us; hledger has nothing to say about
them, so the warning is not compared and the transactions are, as for any journal.

Every rejected journal of the corpus is compared by line today, except those three. The errors of hledger's
`check` command (undeclared accounts and the like) are not reported by `check()` yet and no journal of the
corpus depends on them; when they are built, their journals are compared by line too.

One corpus journal is compared by value only, with the places of the quantities ignored
(`placesMayDiffer` in `compare.ts`; the PASS note says so): `assign-inclusive-order-names-x123` is one of a
pair where hledger adds the sub-accounts of `a =* 4` in an order that follows no rule we know, which changes
the places of a sum that passes through zero (`spec/hledger/assignments.md`). We add them in name order.

How a case is compared: hledger's `print -O json` is projected onto our `Transaction` shape (date,
status, description, comment trimmed, source file and line, postings with account, kind, comment
trimmed and amounts, each with its price and lot). Quantities are exact decimal strings built
with `BigInt` from hledger's `decimalMantissa` and `decimalPlaces`, so places and big numbers
survive (very large mantissas come in floating notation and are read back exactly).
`source.file` is compared by basename.

### Postings and prices

Every hledger case that hledger accepts and that passes the comparison above is also compared on `postings()` and
`prices()` (`spec/hledger/postings-flat.md` and `prices-list.md`):

- **postings**: `hledger print --location -O csv` against `ledger.postings()`, row by row. The CSV is decoded with
  `decodeCsv` (fields with newlines are quoted); the file and the header lines are those of the **last line** of the
  `comment` field (`location: FILE:FIRST-LAST`), and the file is matched to the name we were given (the same string when
  absolute, else the one the resolved path ends with). Compared exactly: date, status ↔ `flag`, description ↔ `narration`,
  account (one pair of brackets stripped, the kind read from them), `amount` ↔ `number` (a comma of a decimal-comma style is
  read as the point) and its places, commodity ↔ `currency`, file, and no payee. hledger gives no line for a posting: it must
  lie inside `FIRST+1..=LAST` of its transaction and must not go back inside one (rows share a `txnidx`);
- **prices**: `hledger prices` run twice, the second time with `-c "1.000000000000000000 COMM"` for every commodity of the
  rate column of the first (as `spec/hledger/show-prices.sh` does), decoded as `P DATE FROM RATE TO` (quoted names, a rate with
  no commodity), against `ledger.prices()` in order: date, commodity, rate commodity and the rate text, **places included**
  (the second run keeps the places written, and pads a bare rate to `D`'s). The file must be one we were given and the line
  must hold a `P`.

A case is not asked these when hledger rejects it, when it fails the first comparison or when we refuse it (a `P` line
with a price, a lot or a label is refused, so the journals that have one live in `spec/hledger/probes/refused/`). A
difference is a FAIL: `postings: [3].number: hledger="1.50" ours="1.5"`, `prices: [0].rate.quantity: ...`. The summary
has a line each (`hledger postings: ...`, `hledger prices: ...`), and the fuzzer asks both of every valid journal.

### Cost and value

Every hledger case that hledger accepts and that passes the comparison above is compared twice more,
with the same projection and the same diff (`ignorePlaces` too):

- **`--cost`**: `hledger print -O json --cost` against `ledger.atCost()` (`Ledger::at_cost()`).
- **`--value=DATE,COMM`**: `hledger print -O json --value=DATE,COMM` against `ledger.value(DATE, COMM)`, for a
  journal with `P` lines (any file of the case), at: one day before the first `P` date, each distinct `P` date
  (six at most, spread evenly over the sorted dates when there are more) and 30 days after the last; in `EUR`,
  `USD`, the commodity of the first price and the first commodity priced. Hence up to 32 questions per
  journal (`valueQueries` in `lib.ts`, read from the `P` lines of the files; `priceLines` is the reader).

A journal hledger rejects is not asked these: it is compared as above (first error), and so is a case that
fails or is skipped there. Verdicts:

- the two agree: PASS; they differ: FAIL, the note names the query (`cost: ...` or `value=DATE,COMM: ...`);
- we return an `unsupported-syntax` error: **SKIP**. The wasm throws an `Error` whose message is the finding as
  JSON; the harness reads it. The shapes `at_cost()` refuses are those of `spec/hledger/cost.md` (Gaps): a
  transaction with pairs of conversion postings and either several priced postings or a balance assignment.
  One corpus journal is skipped for that (`conversion-several.journal`);
- any other error of ours, or hledger failing a question after accepting the journal, is a FAIL.

The summary has a line per kind before the total: `hledger --cost: ...`, `hledger --value: ...` and `hledger holdings: ...`
(one count per question asked; see "Holdings"). A case's line shows the worst of its comparisons.

### Holdings

Every hledger case that passes the first comparison is also asked `ledger.holdings({ commodity: V, end, today })`
(`spec/hledger/holdings.md`), compared with `hledger balance --layout=tidy -O json --real` (the exact values, not the CSV).
The questions (`holdingQuestions`): `end` none and up to three end dates taken from the `--value` dates around the `P` lines
(the first, the middle one, the last), each valued in `EUR`, `USD` and the first commodity priced; a journal with no `P`
line: no `end` and the day after its middle transaction, in `EUR`. `today` is this machine's date (hledger's clock). For each
question:

- the units: one run with no query (never a `--` with no account pattern: that makes the run empty, see the note);
- for each commodity C that we or the units run hold: `--cost`, `--value=end,V` and `--cost --value=end,V`, each with
  `cur:^C$` (regex characters escaped, `currencyQuery`). The `--cost` runs do not depend on V and are kept per journal.

The JSON is decoded with `decodeBalance`: each account's amounts added up per commodity whatever their costs, by value,
zero sums left out. The rows are compared per (account, C): the units, and the cost, the value and the cost value as lists
of `COMMODITY QUANTITY` (as sets of exact quantities, places ignored). A holding hledger has no row for counts as zero, so
`holdings()` may list a zero one. The order of `holdings()` is checked too (account names by segment, commodity by code point).

`cur:` is a case-insensitive regex, so `cur:^X$` also finds `x`; such a C cannot be asked alone. Its rows are left out on both
sides, the others are compared, and the verdict is a **SKIP** if they agree (`cur: cannot tell x, X from another commodity`;
seven corpus journals). A refusal of ours (`unsupported-syntax`) is a SKIP, any other error a FAIL, as for `--cost`.

**Plainledger's reads.** The journals of `harness/journals/app` are Plainledger's own test journals. Each of them (and its
pairs) is also checked against the commands Plainledger runs today (`packages/ledger-domain/src/holdings/hledgerHoldings.ts`):
prefixes `assets` and `liabilities`, V = EUR, no end; the units run (`--real`, no query); for each held C that is not EUR or `€`,
the three runs `balance --layout=tidy -O csv [--real] --cost|--value=end,EUR|both -- assets liabilities cur:C`. From the CSV,
the cost basis and market value in EUR of each (account, C) are compared with ours from `holdings()` filtered the same way
(case-insensitive prefix, C not an alias of V, the missing-cost rule: a C left in `--cost` makes the cost basis null), within the CSV's
rounding. First with `--real` on the three runs (the owner's choice, "real postings everywhere"): any difference is a FAIL
(`plainledger: ...`). Then without `--real` there, as Plainledger runs them: the journals where the figures differ are
listed under the summary (`Plainledger's figures that change with real postings only`). That is a virtual posting on a tracked
account, and none of the app journals has one, so the list is empty. Accounts with a space in their name are left out of both
sides: Plainledger looks them up by their first word. The summary line is `Plainledger's reads: ...`.

The fuzzer asks `holdings()` too (two questions per journal: no end date, and one end date; a random valuation commodity).
Every hledger run of the holdings comparison has `-E`: without it `balance` hides an account whose amounts all look zero at
their display precision (an inferred `-0.01506 Y` when `Y` is written with one place), with or without `cur:`, and the
engine keeps the exact amount (`spec/hledger/holdings.md`, "balance hides an account whose amounts all look zero").

### Beancount

For a `.beancount` or `.bean` file (a single file; a Beancount pair is not run), the harness:

1. runs `bean-check FILE`. `bean-query` exits 0 even for an invalid journal, so `bean-check`
   decides whether the reference accepts the file (a non-zero exit means it does not);
2. if it accepts, runs `bean-query -f csv FILE` three times:
   - the transactions: `SELECT id, type, filename, lineno, date, flag, payee, narration FROM
     entries WHERE type = 'transaction'`;
   - the postings: `SELECT id, account, posting_flag, number, currency, cost_number,
     cost_currency, cost_date, cost_label, number(price), currency(price), filename, lineno FROM
     postings`;
   - the flat postings: `SELECT date, flag, payee, narration, account, number, currency,
     filename, lineno ORDER BY date, filename, lineno`;
3. decodes the first two CSVs (`decodeCsv`: CRLF rows, quoted fields with commas, doubled quotes and
   newlines) and joins them on `id`. A transaction without postings stays; several postings may
   share an `id` and a line (a sale over several lots). Decoding starts at the header, because
   `bean-query` prints errors in front of the CSV. The padding `bean-query` puts around numbers is
   trimmed. The result is projected onto our `Transaction` shape (flag, payee, narration, each
   posting's flag and line, its price, and its lot from the cost columns). A transaction's
   comment is always empty;
4. diffs that with what the wasm returns, like for hledger;
5. decodes the flat postings the same way (starting at the header, numbers trimmed), projects each
   row onto our `FlatPosting` JSON shape (a missing payee is left out) and diffs the list with
   `postings()`, `file` by basename. The row order is part of the comparison. A case passes only if
   the transactions and the postings both match; a postings difference is reported as
   `postings: [3].line: ...`.

Verdicts are as above, with these Beancount cases:

- `bean-check` rejects the journal: the harness takes the file and line of the **first error** it
  printed (a line `FILE:LINE: MESSAGE`; the indented copy of the directive under it is skipped) and
  compares them with our **first error-severity finding** (`check()` for Beancount is in
  `bean-check`'s order, so this is the reference's first). Same file (by basename) and same line:
  PASS, `both fail, same first error`. Another line or file: FAIL, with both. No error finding of
  ours: FAIL. Warnings do not count. The exception is a missing `document` file (see SKIP).
  When the first error matches and the file was booked, the harness also runs the three
  `bean-query` queries (which still list the entries of a rejected journal, after its errors) and
  diffs transactions and flat postings as for an accepted journal, padding included; a
  difference is a FAIL (`both fail, same first error, same lists` is the PASS). Not diffed, because
  the lists cannot match by design: a file with a parse-level error (`parse-error`,
  `invalid-account-name`, `unbalanced-push`, `duplicate-metadata`: the parser gives up on the whole file,
  bean-query lists part of it) and a file with a `booking-error` other than two elided postings
  (bean-query keeps such a transaction, often without its postings, and ledgermill leaves it out);
- `bean-check` accepts the journal and we report a warning (`ignored-syntax`): fine, the
  transactions and postings are compared as usual;
- we report `unsupported-syntax`: SKIP, whether or not the reference accepts.

The summary prints a line per format when both are run. A FAIL is also: we report an error where
`bean-check` accepts, or the transactions or the postings differ.

The fuzzer (`fuzz.ts`) writes random, mostly valid journals covering number forms and marks,
directives, prices, lots, elided amounts, inferred prices, conversion postings, virtual postings
and big numbers, and for hledger `=` and `~` rules, about a third of them invalid on purpose (they must
fail in both), and `P` lines (about 45%: dates, times of day, zero and negative prices). Each hledger
journal is compared raw, for its postings and prices, under `--cost`, and under one or two random `--value=DATE,COMM` (dates around the
`P` lines, the fuzzer's commodities). Same seed, same journals; a FAIL prints the journal.

`bun harness/fuzz.ts --graph N SEED` writes dense networks of prices instead: 9 to 11 commodities out of
twelve (names that sort in different orders, `Z` before `a`), 1 to 16 `P` lines with random dates, targets and
rates (zero and negative ones too), a transaction with an amount in each commodity (some with a cost or a lot)
and now and then a posting with several amounts; each journal is compared raw, under `--cost` and under two
random `--value` runs. These exercise the choice of the path and the arithmetic of `spec/hledger/value-prices.md`
and `value-arithmetic.md`.

## Corpus

All journals are written by us and safe to publish.

- `journals/prototype/`: probe journals from our earlier prototype.
- `journals/app/`: test journals of our first user, a personal finance app, published with its
  author's approval.
- `../spec/hledger/probes/`: the probe journals behind each spec note.
- `../spec/hledger/probes/refused/`: the hledger journals ledgermill refuses (`unsupported-syntax`).
  `compare.ts` does not run them, since hledger accepts them; the Rust test
  `crates/ledgermill/tests/hledger_refused_probes.rs` reads the directory and checks that each gives an
  `unsupported-syntax` error.
- `journals/app/*.beancount`: the Beancount test journals of our first user (12).
- `../spec/beancount/probes/`: the Beancount probe journals behind each spec note
  (`probes/refused/` holds those ledgermill refuses; they are not run by `compare.ts`).
- Multi-file cases: the `pairs`, `fixturePairs` and `specPairs` lists in `compare.ts` (files are loaded
  in the order listed; `specPairs` are probes of `../spec/hledger/probes/`: assertions and assignments, and the
  `rule-file-*` pairs, where a file with a rule is read to its end before the next starts).

Never commit private journals: `real.ts` exists so you can check them locally.
