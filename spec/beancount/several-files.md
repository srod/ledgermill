# Several files in one `load`: one ledger per Beancount file

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), hledger 1.52.4, 2026-10-27.

Status: **built** (PLA-393). This note records a design decision of the project owner, and what the reference
tools do that makes it the sensible one. The merge order and the sums are ours: no reference tool loads a
Beancount file and a journal together, so there is nothing to compare them with, and the harness compares each
file alone.

## Rule

`load` takes a list of files (the manifest). The ledgers in it are:

- **each Beancount file** (`.beancount`, `.bean`) is **one ledger on its own**. Nothing is shared between two
  Beancount files: no account, no option, no price, no balance assertion, no padding. `include` is still refused
  (refused.md), so a file never pulls another in;
- **all the hledger files together** are **one ledger**, exactly as before (hledger/ordering.md,
  hledger/assertion-failures.md). A manifest with hledger files in it has one hledger ledger, wherever
  its files sit in the list.

A manifest with one ledger (one Beancount file, or hledger files only, or no file) behaves as it always did,
down to the bytes. The rest of this note is about a manifest with more than one ledger, which used to be refused
(`several-beancount-files` and `mixed-formats` are gone).

### Why one ledger per Beancount file

Plainledger stores one Beancount file per user ledger and asks `bean-query` about each file separately. And
`bean-check` itself gives no way to read two files side by side without the `include` that ledgermill refuses:
on a hand-made root that includes them, the files stop being independent.

- Each file is valid on its own and both are in one directory, but a root that includes both is not: both open
  `Assets:Bank` and `Equity:Opening`, which is fine in each file and a *Duplicate open directive* under the
  include (`open-a`, `open-b`, `include-open`).
- An option belongs to the file it is written in. `rename-a.beancount` renames the assets root (`name_assets`)
  and loads with `Actifs`; included from a root that does not say so, the option is not applied (`Assets` again).
  Joining the files by include would therefore change what a file means, while loading them one by one cannot
  (`rename-a`, `include-rename`).

So the only reading that keeps every file as its reference tool reads it is one ledger per file, and the results
are combined afterwards.

## Merge order

For a manifest with more than one ledger:

- `transactions()`, `postings()` and `prices()` are a **stable sort by date**. On the same date the item with
  the file earlier in the manifest comes first (the **manifest index** of its source file: `Transaction.source.file`,
  `FlatPosting.file`, `PriceDirective.file`; the first match if a name is given twice). What is left is each
  ledger's own order, which is already the order the reference prints for it. For one ledger these keys agree
  with the engine's own order, so a manifest of one ledger is unchanged.
- `check()`: the findings of each ledger, in the order that ledger gives them, **stable-sorted by the manifest
  index of `finding.file`**. A finding whose file is not in the manifest takes the index of its ledger's first
  file. In one file the order is the ledger's own. The ledgers are taken in the order of their first file.
- A Beancount file with a construct ledgermill refuses (an `unsupported-syntax` error) **contributes nothing**, as
  when it is loaded alone, and its finding is in `check()`. The other ledgers load as usual.

### Valuation

- `at_cost()` and `value(date, commodity)` ask each ledger on its own (each with its own prices) and put the
  answers back in the order of `transactions()`: result `i` is for `transactions()[i]`. The error is the first
  ledger's, ledgers being ordered by their first file's manifest index: for example the refusal of a Beancount
  file, or `invalid-argument`. A refused file therefore makes `at_cost`, `value` and `holdings` fail for the
  whole manifest, as one refused file always did.
- `holdings(query)` asks each ledger on its own. The rows of **one account and one commodity** that are in several
  ledgers are **added**:
  - `units` is the exact sum of the units;
  - `cost`, `value` and `cost_value` are added per commodity;
  - a row in one ledger only is passed through with its strings exactly as the ledger printed them;
  - the rows are then ordered as `holdings()` orders them: by the account's `:`-separated names, then by
    commodity, both in code point order.

  **How a sum is printed.** Each term was already rounded by the engine that produced it (ten places for hledger,
  28 digits for Beancount), so the sum of the printed strings is not rounded again. It is exact and keeps the
  places of its most precise term, which is what both engines do when they add (`1.50 + 2.5` is `4.00`). A
  commodity whose sum is exactly zero is left out of a list, and a units sum of exactly zero is `"0"`, as both
  engines print them. A summed row with units zero and no commodity left in any list is not a holding and is
  not listed, as in either engine. Values are those of each ledger at its own prices: the hledger ledger values at
  its own latest date, a Beancount file at its own latest prices. Prices of one ledger are never used for another.

## Probe

`probes/several-files/open-a.beancount`, `probes/several-files/open-b.beancount`,
`probes/several-files/include-open.beancount`, `probes/several-files/rename-a.beancount`,
`probes/several-files/include-rename.beancount`

The merge, check, valuation and holdings rules are tested in `crates/ledgermill/tests/multi_ledger.rs` on
`probes/several-files/a.beancount`, `b.beancount`, `c.journal` (three ledgers with an account, a commodity and a
day in common) and the files `unbalanced.*`, `refused.beancount`. Their expected values are what
`spec/beancount/show.sh`, `show-prices.sh`, `show-holdings.sh` and the hledger `show-*.sh` scripts print for each
of those files alone, plus the rules above.

Command:

```sh
spec/beancount/show-several-files.sh
```

`show-several-files.sh` runs `bean-check` on each probe, and loads it with `beancount.loader` to print the
`name_assets` option it ends with, the number of entries and the number of errors. File names are cut to their last
part.

## Output

```text
== open-a.beancount
bean-check: (no output)
name_assets: Assets | entries: 3 | errors: 0
== open-b.beancount
bean-check: (no output)
name_assets: Assets | entries: 3 | errors: 0
== include-open.beancount
bean-check: open-b.beancount:1: Duplicate open directive for Assets:Bank
   2026-01-01 open Assets:Bank
open-b.beancount:2: Duplicate open directive for Equity:Opening
   2026-01-01 open Equity:Opening
name_assets: Assets | entries: 6 | errors: 2
== rename-a.beancount
bean-check: (no output)
name_assets: Actifs | entries: 3 | errors: 0
== include-rename.beancount
bean-check: rename-a.beancount:4: Duplicate open directive for Equity:Opening
   2026-01-01 open Equity:Opening
name_assets: Assets | entries: 6 | errors: 1
```

## Not settled

- A manifest that names the same file twice is read twice. Two Beancount files with the same name are two
  ledgers with the same manifest index: their items on one date come in ledger order.
- The sums are checked by the tests against the rule above, not against a reference tool: none adds the holdings
  of two files.
