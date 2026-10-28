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
crates/ledgermill-wasm/build.sh
```

The script builds the wasm, runs `wasm-bindgen` into `pkg/` and adds the npm `package.json`
(`@ledgermill/wasm`). Run it again after changing the Rust code: the harness loads `pkg/`. An app
can use the package before the first release with `cd pkg && bun link`.

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
bun harness/fuzz.ts --beancount [N] [SEED]   # N random Beancount journals with prices, lots and costs; CI runs 150
bun harness/fuzz.ts --multi [N] [SEED]   # N random sets of 2 to 4 files, Beancount and hledger, several ledgers; CI runs 60
bun harness/real.ts FILE...       # your own journals (several files: each alone, then all together)
bun test harness/                 # the harness's own checks
bun harness/stress.ts [N]         # N (default 100,000) generated transactions per format through every call of the wasm
bun harness/bench.ts [--runs N] [--against DIR | --against-npm VERSION]   # desktop timing bench, see below
bun harness/device-bench/server.ts   # the phone bench, for the release gate: harness/device-bench/README.md
```

`harness/interface.test.ts` fails when the wasm exports a function or a `Ledger` method that the comparison never calls,
so every method of the interface is compared with the reference tools.

The release gate (`.github/workflows/release-gate.yml`, run by hand before a release, see `RELEASING.md`) runs 5,000 of
each kind of random journal (hledger, `--graph`, `--beancount`, `--both-ways`, `--multi`) in shards of 500 with seeds of their own, the corpus, the
harness's own checks, the stress test, the desktop bench against the last published package, the similarity check
(`scripts/similarity/`) and what the packages would ship.

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

The last line is `N pass, N fail, N skip`, over everything compared (raw, postings, prices, format, `--cost`, `--value`, holdings,
Plainledger's reads, the Beancount reads and the sets of several ledgers; the lines before it count each of them: see "Postings and
prices", "Cost and value", "Holdings", "Beancount prices, cost, value and holdings" and "Several ledgers").

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

### Format

Every case (any journal, also one the reference rejects or we refuse) is run through `format(name, content)` of the wasm
(`spec/hledger/format.md`, `spec/beancount/format.md`), file by file, and must satisfy (`compareFormat` in `lib.ts`):

- formatting again changes nothing (`format(format(x)) === format(x)`);
- the formatted text equals the original once all whitespace is removed (`sameWithoutWhitespace`);
- the reference reads the same: the original and the formatted files are written to a temp dir under the same basenames
  (one subdirectory per file, so the set and the order of a pair are kept) and read by the reference, compared with the
  same projection and `firstDiff` as above, source lines included (lines never move). For hledger it is
  `hledger print -O json`. For Beancount it is the transaction and posting rows of the Beancount comparison
  (`bean-query`); when the reference rejects the original it must reject the formatted files with the same first error
  (file and line; hledger with no position: the same message). A Beancount file the formatter returned as it was is not
  read again.

A difference is a FAIL (`format: ...`). The summary line `hledger format: N pass, N fail (N changed, N returned unchanged)`
counts the cases whose text the formatter changed and those it gave back as they were (already aligned, nothing to align, or
its own safety check refused): a formatter that silently does nothing shows as `0 changed`. The fuzzer asks the same of every
journal, `--graph` and `--beancount` included.

**Beancount against `bean-format`.** A Beancount file must also come out as `bean-format` prints it
wherever alignment is the only difference. The harness runs `bean-format FILE` on the original (its stderr is never
shown) and turns both texts into the shape `bean-format` prints (CRLF and CR to LF, a final newline added: ours keeps the
endings, `bean-format` does not). Then, in order:

1. **no reference**: the file starts with a byte order mark or is not UTF-8 (`bean-format` crashes on some), `bean-format`
   exits non-zero (a blank other than a space or tab next to a number), or its text differs from the original by more than
   whitespace. Nothing is compared;
2. **equal**: the two texts are the same;
3. **differ by design**: they differ, and `designDifference` gives a reason from the spec: ledgermill refuses the file or
   cannot parse it and returns it as it was (`unsupported-syntax`, or a parse-level error); a string runs over a line break
   (`bean-format` rewrites the lines inside it, which changes the narration; we do not); or the account lines have more than
   one indent (`bean-format` works the column out from the indents as written, we from the usual one, so the texts agree
   only when there is one);
4. otherwise a FAIL (`differs from bean-format (alignment only): line N: ...`).

The summary line `beancount format: N pass, N fail (N changed, N returned unchanged; bean-format: N equal, N differ by design,
N no reference)` counts them. `compare.ts` also formats the Beancount files of `spec/beancount/probes/refused/` (format only: they
are not compared otherwise, and are not in the pass/fail/skip totals of the raw comparison).

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

For a `.beancount` or `.bean` file on its own (several files, or a Beancount file next to a journal, are a set: see "Several ledgers"),
the harness:

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

#### Beancount prices, cost, value and holdings

Every Beancount journal that `bean-check` accepts and whose transactions and postings pass the comparison above is asked
`prices()`, `at_cost()`, `value()` and `holdings()` too. Each is compared with `bean-query -f csv` run with the query texts
Plainledger sends (`apps/server/lib/beancountQueries.ts`, `packages/ledger-domain/src/holdings/beancountHoldingsQueries.ts`),
word for word, except where said. Numbers are compared as the exact strings the reference prints, places and exponents
included (`1E-7`, `-0.00`), never by value. A tool that exits non-zero or prints a Python traceback is a FAIL whose message
does not repeat its output: tracebacks are not read (`EXPOSURE.md`).

- **prices**: `SELECT date, currency, number(amount), currency(amount) FROM prices` against `ledger.prices()`, in order,
  row by row (date, commodity, rate commodity and number), and `SELECT date, lineno, filename FROM entries WHERE type =
  'price'` (not Plainledger's) for the line and file of each (`spec/beancount/price-table.md`).
- **cost**: the postings query with `number(cost(position)) AS amount, currency(cost(position)) AS commodity` against
  `ledger.atCost()`, posting by posting; the account, units and commodity columns must agree with the posting too, so that
  a row cannot be matched to another posting (`cost-of-position.md`).
- **value**: the same query with `convert(position, "V", date(Y, M, D))` against `ledger.value(date, V)`
  (`convert-rates.md`, `convert-positions.md`). The questions (`beanValueQueries`) are asked of the reference's own price
  table: one day before the first price date, each distinct price date (six at most, spread evenly), 30 days after the
  last; in `EUR`, `USD` and the two commodities of the first price. A journal with no price: the middle transaction's date,
  in `EUR` and `USD`.
- **holdings**: for no date and the first, middle and last date of the value questions, in the same commodities, the units
  query and the holdings query (`WHERE date <= D` when a date is given) against `ledger.holdings({ commodity, end, today })`,
  with `end` the day after `D` (`end` is exclusive, `date <= D` is not; `holdings.md`). Every cell exactly, in a cell the
  amounts as a set (their order is not settled), a `()` row as absent. The two queries must agree on the units too.

The summary has a line each (`beancount prices`, `beancount cost`, `beancount value`, `beancount holdings`). We refuse a
journal with an `unsupported-syntax` finding: SKIP, and none of this is asked. Plainledger's own Beancount test journals
(`journals/app/*.beancount`) go through the same comparisons, so its holdings, units, prices and cost queries are covered on
them. `fuzz.ts --beancount` writes random Beancount journals (prices among seven commodities, out of order, twice a day,
zero and negative, one in 2099 now and then; lots, price annotations and plain units of either sign) and asks all of the above.
One journal in five has one ASCII digit replaced by another kind of digit (Arabic-Indic, fullwidth, superscript...), which
both must refuse on the same line (the parser used to panic on such a date, PLA-406). Three journals in ten also have the
one-letter currency `X`, which bean-check reads only before a space or a line end: fine as units or a price, refused in a cost
(`{3 X}`), and both must agree (PLA-411). `--multi` keeps `XX` and `YY`, so that its sets do not mostly carry a refused ledger.

The fuzzer (`fuzz.ts`) writes random, mostly valid hledger journals covering number forms and marks,
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

### Several ledgers

A set with more than one ledger (`spec/beancount/several-files.md`) is compared as a set: several Beancount files, or Beancount files
and hledger files in one `load`. **Each Beancount file is a ledger of its own, and all the hledger files together are one**, wherever
they sit in the list. Nothing here applies to a set with one ledger (one Beancount file, or hledger files only): it is the case of the
sections above, unchanged. `compareAll` decides by `isMultiLedger`.

**The reference** is run per ledger, with the pipelines above: `bean-check` and the `bean-query` queries on each Beancount file alone,
and `hledger` on all the hledger files of the set together, in the order of the list. The harness then merges those results by the rule
of the note, with an implementation of its own (`mergeOrder`, `splitLists`, `sumHoldingRows`, `expectedFirstError` in `lib.ts`, each with
a self-test of hand-written values): it never calls the engine. What it compares them with is the wasm loaded with the whole set at once.

- **first error.** When some ledgers are rejected by the reference, the expected first error is that of the rejecting ledger whose error
  file comes first in the list (a file that is not in the list counts as its ledger's first file; a tie goes to the earlier ledger); it is
  compared with our first error-severity finding as for one ledger (`compareFirstError`, `compareRejection`), and nothing else is
  compared. When hledger rejects with no position and another ledger is rejected too, the rank cannot be told: only "both fail" is asked.
- **transactions.** Each ledger's transactions as the single-file cases project them, stable-sorted together by date and then by the
  position in the list of the transaction's file (the ledger's own order for the rest), against `transactions()`: the same projection and
  diff.
- **`postings()` and `prices()`.** The reference's rows are put in the merged order by the same key (the date, the position of the file).
  Our list is cut in that order into one list per ledger, and each part is compared as the format does alone: `diffPostings` against
  `print --location -O csv` for the hledger ledger, the `bean-query` rows for a Beancount file; `diffPrices` against `hledger prices`
  (two runs), `compareBeanPrices` for a Beancount file. Our order is therefore checked: a part cut in the wrong place differs. `hledger
  prices` does not name the file of a row; it is told from the `P` lines of the files (`attributePrices`: the rows of a date go to the
  files that have a `P` of that date, in file order).
- **`atCost()` and `value()`.** Ours is cut in the order of `transactions()` and each part compared with the answer of its own ledger:
  `hledger print --cost` or `--value=DATE,COMM` for the hledger ledger, the cost and `convert` queries for a Beancount file (ours as
  booked line up with the rows). Prices are those of each ledger. The questions are the union of each ledger's own (the hledger ones from
  its `P` lines, the Beancount ones from its price table, see "Cost and value" and "Beancount prices, cost, value and holdings"), cut down
  by `thinQueries` to at most four dates (the first, the last and the others spread evenly) and four commodities (EUR and USD first):
  at most 16 per set.
- **`holdings()`.** Per ledger, the reference rows that the single-file cases decode: `hledger balance --layout=tidy` runs for the
  hledger ledger (quantities by value), the units and holdings queries for a Beancount file (exact strings). The rows with the same account and
  commodity in more than one ledger are **added exactly** (`BigInt` decimals): units, and cost, value and cost value per commodity;
  a zero sum is left out of a list, a units sum of zero is `0` and a row with nothing left is dropped; the others pass unchanged. Our
  rows are compared **by value, places ignored**, where a row was summed, and for any hledger row (it gives values only). A row of one
  Beancount file that passed unchanged is compared **exactly** (places included), unless the text of another ledger mentions its account:
  that ledger may hold a zero row of it, which hledger does not list and which would change the places of the sum. For summed rows the
  shape the note promises is checked too (`summedShape`): no zero sum in a list, `0` and not `0.00` for a zero units sum, no row with
  nothing in it. The questions are the first, middle and last date of the value questions and none, in at most three commodities
  (`setHoldingQuestions`). A commodity hledger cannot ask for alone (`cur:`, see "Holdings") is left out of every ledger and of ours; the
  verdict is a SKIP if the rest agrees.
- **an `unsupported-syntax` finding anywhere in the set** (a Beancount file with a plugin, say): the whole set is a SKIP, as one such
  file always was. `plugin` is in `EXPECTED_SKIPS` for the probe that does this on purpose.

Not asked of a set: `format()` (every file is formatted in its single-file case) and Plainledger's reads (single files and pairs only).

The summary has a line for the sets: `multi-ledger: N pass, N fail, N skip` (the transactions and the first errors), and one each for
`postings`, `prices`, `--cost`, `--value` and `holdings`; they are in the total. `compare.ts` runs the sets of `severalSets` (the probes of
`spec/beancount/probes/several-files/` in several orders, with the hledger file first, in the middle and last; the rejected ones; the
refused one next to good ones) and `appSets` (Plainledger's own Beancount journals with each other and with its hledger ones, which
share accounts such as `Assets:Bank`, so that rows are added; all twelve Beancount ones together; all with three journals among them).
`real.ts` with several files runs each alone and then all together, as a set, whatever the formats.

`bun harness/fuzz.ts --multi [N] [SEED]` writes sets of 2 to 4 files (one Beancount at least, then Beancount or hledger; a hledger
journal may bring its second file), in a shuffled list order, with the generators above: the hledger accounts take the Beancount
generator's names and the commodities EUR, USD, XX and YY are in both, the dates overlap, so days tie and rows are added. About half of the
sets have a rejected ledger (the hledger generator writes invalid journals on purpose), which asks for the first error. Per set: one or
two `--value` questions and two holdings questions. Same seed, same sets. A FAIL prints the files in list order, and `--save DIR` writes
them as `multi-SEED-N-I.beancount|journal` (`I` is the position in the list) with `multi-SEED-N.txt` that holds the note and the
questions; `real.ts` loads them in that order.

## Stress test

`bun harness/stress.ts [N]` generates N transactions (default 100,000) per format with `bigjournal.ts` and runs
them through the wasm in `pkg/`: `load`, `transactions()`, `postings()`, `prices()`, `check()`, `atCost()`, `value()`,
`holdings()`, `format()` and `free()`. After each step it prints the time and the size of the wasm memory. It does not
use the reference tools and checks no values.

It fails (exit 1) only on a crash: a wasm trap (`RuntimeError`), an error whose message is not a finding as JSON,
running out of memory, or the process dying. A refusal (a finding thrown by a call) or a finding from `check()` is printed
and is fine. CI runs it after the fuzz steps.

`bigjournal.ts` writes the ledgers: `hledgerJournal(N, seed)` and `beancountLedger(N, seed)` give the same text for the
same seed. It is a personal-finance story (about thirty accounts, EUR, USD, GBP, VWCE and AAPL, prices, purchases at
`@`, `@@` and lots, sales, balance assertions, comments) written in either syntax. Both must be valid for the
reference tool; after changing it, check a journal by hand:

```sh
bun -e 'import { hledgerJournal, beancountLedger } from "./harness/bigjournal.ts"; await Bun.write("/tmp/big.journal", hledgerJournal(10000, 1)); await Bun.write("/tmp/big.beancount", beancountLedger(10000, 1))'
hledger -f /tmp/big.journal check && bean-check /tmp/big.beancount
```

## Desktop bench

`bun harness/bench.ts [--runs N] [--against DIR | --against-npm VERSION]` times the wasm in `pkg/` on 10,000
generated transactions per format (fixed seed): `start` (compile and instantiate from bytes), `load+transactions`,
`check`, `value`, `holdings`, `postings`, `prices` and `format`. Each metric is the median of N runs (default 7)
after one warm-up. It prints a Markdown table, and appends it to `$GITHUB_STEP_SUMMARY` when that is set.

With `--against DIR` (a `@ledgermill/wasm` package directory, such as another `pkg/`) or `--against-npm VERSION`
(`npm pack @ledgermill/wasm@VERSION`, for example `latest`), the previous build runs in the same process, as its own
module instance, alternating with the current one, so the speed of the machine affects both alike. A metric the previous
build lacks shows `n/a` and is not compared. With `--against-npm` and no published version (npm answers 404) there is no
previous release: the current timings are printed and the exit code is 0.

It fails (exit 1) when a metric of the current build is more than 20% slower than the previous one, or when a call of the
current build throws. A metric also has to be 0.1 ms slower to fail, since a timer cannot tell 0.01 ms from 0.02 ms.
Comparing a build with itself (`--against pkg`) passes, with ratios near 1.0. CI does not run it on every push.

## Corpus

All journals are written by us and safe to publish.

- `journals/prototype/`: probe journals written for our retired TypeScript prototype; they are test inputs
  like the others, and the prototype is no reference.
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
- Sets of several ledgers: the `severalSets` and `appSets` lists in `compare.ts` (see "Several ledgers").
- Multi-file cases: the `pairs`, `fixturePairs` and `specPairs` lists in `compare.ts` (files are loaded
  in the order listed; `specPairs` are probes of `../spec/hledger/probes/`: assertions and assignments, and the
  `rule-file-*` pairs, where a file with a rule is read to its end before the next starts).

Never commit private journals: `real.ts` exists so you can check them locally.
