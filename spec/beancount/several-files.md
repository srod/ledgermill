# Several files in one `load`: the Beancount files are one ledger

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), hledger 1.52.4, 2026-11-14.

Status: **built**. This note replaces the earlier rule of one ledger per Beancount file. The merge order of the two
ledgers and the sums are ours: no reference tool loads a Beancount file and a journal together, so there is nothing to
compare them with. Inside the Beancount ledger the reference is the one that `include` builds, as below.

## Rule

`load` takes a list of files (the manifest). The ledgers in it are:

- **all the Beancount files** (`.beancount`, `.bean`) together are **one ledger**, read in the order given. Accounts,
  `open` and `close`, pads, balance assertions, commodity declarations, lots and prices apply across the files as if the
  text were one file. `include` is still refused (refused.md), so a file never pulls another in; the manifest does it;
- **all the hledger files together** are **one ledger**, exactly as before (hledger/ordering.md,
  hledger/assertion-failures.md), wherever its files sit in the list.

A manifest with one ledger (Beancount files only, or hledger files only, or no file) is that ledger, down to the bytes.
With both formats the two ledgers merge as below. `format()` stays one file at a time.

### What the reference does with several files

The reference reads several files only through `include`. A set of Beancount files is therefore what a root that
includes them reads. The first file of the manifest stands for the **root**, and the others for the included files in
the order given. The harness builds that root: the text of the first file, then an `include` line for each other file
(`harness/README.md`, "Several ledgers"). What `bean-check` and `bean-query` print for it (`show-several-files.sh`) is
what we match.

**The walk.** The reference sorts the entries of all the files together by date, then by directive kind (`open` first,
then `balance`, then the rest, and `close` and `document` last), then by **line number alone**; the file is not in the
key, so entries of two files on one date and one line stay in the order the files were loaded. Every check goes in that
order, whichever file an entry is in:

- A `balance` sees the postings of both files that are dated before it, and none of its own day
  (`bal-a`, `bal-b`); `bal-wrong-b` expects 100, the first file's sum alone, and fails on the second file's line.
- A duplicate `open` is reported on the entry that comes *second in the walk*. With different dates that is the
  later one. On one date the line decides, whichever file it is in: `order-open-a` opens on line 5 and `order-open-b`
  on line 1, and the error is on `order-open-a:5` for either order of the files. With the same line too, it is the
  file later in the list (`open-a`, `open-b`: the error names `open-b`, lines 1 and 2). An account is therefore
  opened and used across files without regard for which comes first (`a`, `b`).
- A `pad` in one file is filled by a `balance` in another (`pad-a`, `pad-b`). The padding transaction stands on the
  pad's file and line; its **postings** stand on the balance's file and line (`pad-b.beancount:1`). In the flat posting
  list the padding rows therefore carry the balance's file.
- Booking sees the transactions of all the files in the same order, so a lot opened in one file is reduced in another.

**Options and plugins.** Only the root's apply. `booking_method` in the root is the default of every file, and one
in an included file does nothing (`fifo-a`, `sale-b`: the sale is booked first-in-first-out when `fifo-a` is the root,
and is an ambiguous match when `sale-b` is). A `plugin` line likewise: `auto_accounts` of the root opens the accounts that
the other files use, in an included file it opens nothing (`auto-a`, `auto-b`), and `implicit_prices` of the root adds
a price for every transaction of every file, on the transaction's own file and line, where one in an included file
adds none (`implicit-a`, `implicit-b`). So ledgermill takes the options and plugins of the first Beancount file of the
manifest for the whole ledger and ignores those of the others. (A file read as an included file is parsed with its own
options, the account names of `name_assets` among them: `include-rename`. ledgermill refuses every option of that
kind, in any file, so that cannot show.)

### Our order

`transactions()` and `postings()` are a stable sort by date, then by the **manifest index** of the item's source file
(`Transaction.source.file`, `FlatPosting.file`; the first match if a name is given twice), and the rest is the order of
the file (by line), as the lists of the merge below are. The reference lists its entries by line first, and `postings`
by file name, so the harness puts the reference's rows in our order before it compares. A padding transaction sorts
with the file of its pad, its rows with the file of its balance.

`prices()` of the Beancount ledger is the **table's own order**: by date, then line, a line shared by two files in file
order. That order is not a detail: with two prices of a pair on one day, a valuation takes the later one of the table
(`implicit-a`, `implicit-b`: a transaction priced at 10 on line 6 of the first file follows one priced at 11 on line 4 of
the second, and `convert` on that day gives 10). For one file every key agrees with the reference's own order.

### Refusals

A construct ledgermill refuses in any Beancount file (`unsupported-syntax`: a plugin that is not `auto_accounts` or
`implicit_prices`, an option it does not read, an expression it does not build, a booking it does not do) **refuses the
Beancount ledger**: it lists no transaction, row or price and runs no check, and `check()` holds the refusals, as for a
single refused file. The files next to it are not read on their own: what they would give without the refused file's
accounts and lots would not be the answer. A parse error in a file does the same, with its own findings (parse-error
and the like), for every file that has one. The hledger ledger of a mixed set is not affected.

## Merge order

With both formats in the manifest, the two ledgers merge:

- `transactions()`, `postings()` and `prices()` are a **stable sort by date**. On the same date the item with
  the file earlier in the manifest comes first (the manifest index above). What is left is each ledger's
  own order. A manifest of one ledger is unchanged. (Each ledger still values at its own price order.)
- `check()`: the findings of each ledger, in the order that ledger gives them, **stable-sorted by the manifest
  index of `finding.file`**: the hledger ledger's file by file. The Beancount ledger's checks run over all its files
  at once, so its findings stay **one block in its own order**, placed by the file of its first finding. A finding
  whose file is not in the manifest takes the index of its ledger's first file. The ledgers are taken in the order
  of their first file.

### Valuation

The Beancount files are one ledger and so one part: one price table, valued at the prices of all of them.

- `at_cost()` and `value(date, commodity)` ask each ledger on its own and put the answers back in the order of
  `transactions()`: result `i` is for `transactions()[i]`. The error is the first ledger's, ledgers being ordered by
  their first file's manifest index: for example the refusal of a Beancount file, or `invalid-argument`. A refused
  Beancount file therefore makes `at_cost`, `value` and `holdings` fail for the whole manifest, as one refused file always did.
- `holdings(query)` asks each ledger on its own. The rows of **one account and one commodity** that are in both
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
  its own latest date, the Beancount one at its own latest prices. Prices of one ledger are never used for the other.

## Probe

`probes/several-files/`:

- `a.beancount`, `b.beancount`: valid together; `b` opens only `Expenses:Food` and uses `Assets:Bank` and `Assets:Stock`
  of `a`. Both have a transaction and a price on 2026-01-05.
- `bal-a.beancount`, `bal-b.beancount`, `bal-wrong-b.beancount`: a balance in the second file over postings of both.
- `open-a.beancount`, `open-b.beancount`: both open `Assets:Bank` and `Equity:Opening`.
- `order-open-a.beancount`, `order-open-b.beancount`: an open on line 5 and on line 1, on one date.
- `pad-a.beancount`, `pad-b.beancount`: a pad in one file, its balance in the other.
- `fifo-a.beancount`, `sale-b.beancount`: `booking_method` in a file and a sale that needs it in another.
- `auto-a.beancount`, `auto-b.beancount`: `auto_accounts` in a file and a use of accounts in another.
- `implicit-a.beancount`, `implicit-b.beancount`: `implicit_prices` in a file; the other file has the transactions.
- `include-open.beancount`, `include-rename.beancount`, `rename-a.beancount`: hand-made roots that only include.

The merge, check, valuation and holdings rules are tested in `crates/ledgermill/tests/multi_ledger.rs` on `a.beancount`,
`b.beancount`, `c.journal` (an account, a commodity and a day in common) and the files `unbalanced.*`, `refused.beancount`.
`d.journal` is a second hledger file, for a manifest that has a Beancount file between two hledger files (the harness
runs it: `harness/README.md`, "Several ledgers"). Their expected values are what the reference tools print for the
Beancount files read as one ledger, and for the hledger ones, plus the merge rules above.

Command:

```sh
spec/beancount/show-several-files.sh
```

`show-several-files.sh` builds the root of each set in a temporary directory (the first file, then an `include` line
for each other file), and runs `bean-check` and, for some, `bean-query` on it. The last part runs the hand-made roots and
loads them with `beancount.loader` to print the `name_assets` option, the number of entries and the number of errors.
File names are cut to their last part.

## Output

```text
##### accounts, balances, pads and the order of the walk
== a.beancount b.beancount
bean-check: (no output)
== bal-a.beancount bal-b.beancount
bean-check: (no output)
== bal-a.beancount bal-wrong-b.beancount
bean-check: bal-wrong-b.beancount:6: Balance failed for 'Assets:Bank': expected 100 EUR != accumulated 150 EUR (50 too much)
   2026-01-04 balance Assets:Bank                                     100 EUR
== open-a.beancount open-b.beancount
bean-check: open-b.beancount:1: Duplicate open directive for Assets:Bank
   2026-01-01 open Assets:Bank
open-b.beancount:2: Duplicate open directive for Equity:Opening
   2026-01-01 open Equity:Opening
== order-open-a.beancount order-open-b.beancount
bean-check: order-open-a.beancount:5: Duplicate open directive for Assets:Bank
   2026-01-01 open Assets:Bank
== order-open-b.beancount order-open-a.beancount
bean-check: order-open-a.beancount:5: Duplicate open directive for Assets:Bank
   2026-01-01 open Assets:Bank
== pad-a.beancount pad-b.beancount
bean-check: (no output)
== pad-a.beancount pad-b.beancount (postings)
date,filename,lineno,account,number,narration
2026-01-02,pad-b.beancount,1,Assets:Bank,100,(Padding inserted for Balance of 100 EUR for difference 100 EUR)
2026-01-02,pad-b.beancount,1,Equity:Opening,-100,(Padding inserted for Balance of 100 EUR for difference 100 EUR)
##### options and plugins: the root's apply, an included file's do not
== fifo-a.beancount sale-b.beancount
bean-check: (no output)
== sale-b.beancount fifo-a.beancount
bean-check: sale-b.beancount:2: Ambiguous matches for "-1 STOCK {}": 1 STOCK {10 EUR, 2026-01-02}, 1 STOCK {20 EUR, 2026-01-03}
   2026-01-04 * "sell one"
     Assets:Stock  -1 STOCK {}
     Assets:Bank
== auto-a.beancount auto-b.beancount
bean-check: (no output)
== auto-b.beancount auto-a.beancount
bean-check: auto-a.beancount:3: Invalid reference to unknown account 'Income:Gift'
   2026-01-02 * "uses accounts nobody opened"
     Assets:Cash   5 EUR
     Income:Gift  -5 EUR
auto-a.beancount:3: Invalid reference to unknown account 'Assets:Cash'
   2026-01-02 * "uses accounts nobody opened"
     Assets:Cash   5 EUR
     Income:Gift  -5 EUR
auto-b.beancount:1: Invalid reference to unknown account 'Income:Gift'
   2026-01-03 * "uses them too"
     Assets:Cash   2 EUR
     Income:Gift  -2 EUR
auto-b.beancount:1: Invalid reference to unknown account 'Assets:Cash'
   2026-01-03 * "uses them too"
     Assets:Cash   2 EUR
     Income:Gift  -2 EUR
== implicit-a.beancount implicit-b.beancount (prices)
date,currency,number(amount),currency(amount)
2026-01-05,STOCK,11,EUR
2026-01-05,STOCK,10,EUR
2026-01-07,STOCK,11.5,EUR
2026-01-09,STOCK,12,EUR
date,lineno,filename
2026-01-05,4,implicit-b.beancount
2026-01-05,6,implicit-a.beancount
2026-01-07,8,implicit-b.beancount
2026-01-09,10,implicit-a.beancount
== implicit-b.beancount implicit-a.beancount (prices)
date,currency,number(amount),currency(amount)
2026-01-07,STOCK,11.5,EUR
date,lineno,filename
2026-01-07,8,implicit-b.beancount
##### roots that only include
== include-open.beancount
bean-check: open-b.beancount:1: Duplicate open directive for Assets:Bank
   2026-01-01 open Assets:Bank
open-b.beancount:2: Duplicate open directive for Equity:Opening
   2026-01-01 open Equity:Opening
name_assets: Assets | entries: 6 | errors: 2
== include-rename.beancount
bean-check: rename-a.beancount:4: Duplicate open directive for Equity:Opening
   2026-01-01 open Equity:Opening
name_assets: Assets | entries: 6 | errors: 1
```

## Not settled

- A manifest that names the same file twice is read twice, and the second read duplicates every `open`. Two Beancount
  files with the same name share one manifest index, so their items on one date come in file order.
- A `plugin` that cannot be imported is reported by the reference when it is in the root and not when it is in an
  included file, where no plugin is run. ledgermill refuses a plugin it does not build, and an option it does not read,
  in any file.
- The sums are checked by the tests against the rule above, not against a reference tool: none adds the holdings
  of a Beancount file and a journal.
