# Changelog

All notable changes to ledgermill are written here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- `load` reads several Beancount files, and hledger files together with Beancount files, in one call
  (PLA-393). Each Beancount file is its own ledger (`include` is still refused) and all the hledger
  files together are one. `transactions()`, `postings()` and `prices()` are merged by date, then by
  the position of the item's file in the list; `check()` lists each ledger's findings by the position
  of their file; `at_cost()` and `value()` are answered per ledger and line up with `transactions()`;
  `holdings()` adds the rows of the same account and commodity. The `several-beancount-files` and
  `mixed-formats` refusals are gone. A set of one ledger answers as before. See
  `spec/beancount/several-files.md`.
- Harness: sets of several ledgers are compared with the reference (PLA-393). The reference runs per
  ledger (`bean-check` and `bean-query` on each Beancount file, `hledger` on all the hledger files
  together) and the harness merges the results by the rule of the note with an implementation of its
  own; the wasm loaded with the whole set must give the same transactions, `postings()`, `prices()`,
  `at_cost()`, `value()` and `holdings()` (rows of one account and commodity added by value) and the
  same first error. `compare.ts` has 40 such sets (the `several-files` probes in every order, the
  rejected and the refused ones, Plainledger's own journals); `fuzz.ts --multi` writes random sets
  (60 per push, 5,000 in the release gate); the summary has a `multi-ledger` line. See
  `harness/README.md`, "Several ledgers".

### Fixed

- A Beancount currency of one capital letter glued to the next character (`{2 X}`, `1 X,`, `X;`, `X` at
  the end of a CRLF line or of the file) was accepted; bean-check says `Invalid token: 'X}'` (PLA-411).
  It is now a `parse-error` on the line of the letter, the line bean-check gives. A letter followed by a
  space, a tab or the end of the line, or by more currency characters (`X1`, `X.B`), is read as before.
  `fuzz.ts --beancount` now adds `X` to three journals in ten (seed 411, 1,000 journals: 0 fail; of 300,
  79 use `X`, 49 accepted by bean-check, 25 refused on the `X`).
- A Beancount date or time written with digits other than 0-9 (`٢٠٢٦-01-02`, `time: ١:٢٣`) made the
  parser panic, a crash in a browser or an app (PLA-406). It is now a `parse-error` on its line, the
  line bean-check gives, and `format()` returns the file unchanged. The fix is in our parser fork
  (`ledgermill-beancount-parser` 0.16.4). `fuzz.ts --beancount` now puts such a digit in one journal
  in five.

## 0.1.2 – 2026-10-27

### Fixed

- hledger: a price on an amount whose quantity and cost are both zero (`0 X @ 3 EUR`) no longer
  stops a price from being inferred; that amount takes the inferred price, as hledger does. Such a
  transaction was refused as unbalanced since 0.1.0 (`c 0 X @ 3 EUR`, `a 1 X`, `b -4 EUR`;
  hledger gives `0 X @ 4 EUR` and `1 X @ 4 EUR`). Found by a fuzz soak (seed 90006).

### Release gate

From the release-gate workflow on commit `f772a9e` (run 40537425419, 2026-10-27), CI on the same
commit (run 40537426399), and the owner's own runs.

- Random hledger journals (5,000): 0 mismatches (2,236 that hledger accepts; the rest rejected by
  both, 34 skipped, all `conversion-postings`)
- Random Beancount journals (5,000): 0 mismatches
- Random price networks (5,000): 0 mismatches
- Random Beancount journals with pairs priced both ways (5,000, new in this gate): 0 mismatches
- Corpus comparison: 11,822 pass, 0 fail, 30 skip
- Desktop bench against 0.1.1 from npm (CI runner, 10,000 transactions): every operation within
  0.73–1.11× of 0.1.1 (the 0.73 is `check`, at 0.02 ms; the 1.11 is hledger `holdings`, 9.6 to
  10.6 ms); load+transactions 92.5 ms (hledger), 132.6 ms (Beancount); start 19.8 ms
- Stress test, 100,000 transactions: ok, no crash (peak wasm memory 930 MB hledger, 578 MB Beancount)
- Similarity check score: max 0.102 (JPlag against rustledger 0.24.0; fail at 0.15), 0 fail,
  8 warn; copydetect max 0.275 (information only)
- Real journals: pass (21 pass, 0 fail)
- Device targets: not run again (the fix changes no timing or memory path)

## 0.1.1 – 2026-10-27

### Fixed

- hledger: a commodity that `==*` or `=*` zeroed on an account, when one posting with a lot on a
  sub-account made it, keeps that lot; zeroing it again gives the lot back, as hledger does.
  0.1.0 dropped it (`a:b 4 X {10 EUR}`, `a 1 X`, `a ==* 3`, then `a == 2 EUR` gave `4 X`, not
  `4 X {10 EUR}`). Found by the release gate run on the `v0.1.0` tag (random journal, seed 303).
- hledger `--cost` (`atCost()`): in a transaction with two conversion pairs, one that fits the
  priced posting and one that does not, and a first plain posting that another repeats, the pair
  that fits is dropped and the other kept, as hledger does. 0.1.0 kept both (so 7 postings where
  hledger gives 5, and holdings followed). Found by CI's random journals (seed 21).

### Release gate

From the release-gate workflow on commit `4643b81` (run 40479812101, 2026-10-27), CI on the same
commit (run 40479812678), and the owner's own runs.

- Random hledger journals (5,000): 0 mismatches (2,232 that hledger accepts; the rest rejected by
  both, 42 skipped)
- Random Beancount journals (5,000): 0 mismatches
- Random price networks (5,000): 0 mismatches
- Corpus comparison: 11,811 pass, 0 fail, 30 skip
- Desktop bench against 0.1.0 from npm (CI runner, 10,000 transactions): every operation within
  0.74–1.06× of 0.1.0 (the 0.74 is `check`, at 0.02 ms); load+transactions 94.5 ms (hledger),
  134.1 ms (Beancount); start 19.3 ms
- Stress test, 100,000 transactions: ok, no crash (peak wasm memory 930 MB hledger, 578 MB Beancount)
- Similarity check score: max 0.102 (JPlag against rustledger 0.24.0; fail at 0.15), 0 fail,
  8 warn; copydetect max 0.275 (information only)
- Real journals: pass
- Device targets: not run again (the fix changes no timing or memory path)

## 0.1.0 – 2026-10-26

The first release. Two ledger engines, one for hledger journals and one for Beancount ledgers,
in a Rust core that compiles to WebAssembly. They compute with exact decimals, never read a file
or a clock, and answer through typed values.

Packages: `ledgermill` and `ledgermill-wasm` on crates.io, `@ledgermill/wasm` on npm (the wasm
and its glue).

### Matches

- hledger **1.52.4**
- Beancount **3.2.3**, and beanquery **0.2.0** for the queries (`bean-query`)

These are the reference tools the comparison harness runs on journals we wrote. A different
version may behave differently; ledgermill makes no promise about it.

### The interface

`load(files)` reads `{ name, content }` files in order, and never fails: every problem becomes a
finding. The format comes from the file name (`.beancount` and `.bean` are Beancount, anything else
is hledger). The loaded ledger answers:

- `transactions()` – the booked transactions, with file and line (Beancount: padding included);
- `postings()` – one row per posting and commodity, with the fields of its transaction;
- `prices()` – the `P` directives (hledger) or `price` directives (Beancount);
- `check()` – the problems found, as findings;
- `atCost()` – the transactions with every cost applied (`hledger print --cost`; Beancount `cost(position)`);
- `value(date, commodity)` – every amount valued at the prices known on a date
  (`hledger print --value=DATE,COMMODITY`; Beancount `convert(position, V, date)`);
- `holdings(query)` – per account and commodity, the units, cost, value and cost value;
- `free()` – releases the ledger held in wasm memory.

`format(name, content)` realigns the postings of a file, changing only whitespace: hledger
journals like `hledger print` lays them out, Beancount ledgers like `bean-format`. A file that
ledgermill refuses, or whose formatted text would not read as the same ledger, comes back
unchanged.

In Rust the same interface is the `ledgermill` crate (`load`, `Ledger`, `format`); the wasm
binding returns each answer as a JSON string in the core's own shape.

### Both formats

- hledger: transactions, postings, virtual postings, costs and lots, balance assertions and
  assignments, `P` prices, aliases, `apply account`, `D`, `decimal-mark`, `Y`, and the errors
  hledger gives for them, with the same first error (file and line).
- Beancount: the transactions and every directive `bean-check` checks (accounts, `open` and
  `close`, `balance`, `pad`, currency constraints, duplicates, booking), booking of costs, `price`
  directives and the same first error as `bean-check`.

### Findings

`check()` returns findings: `{ code, severity, file, line, column?, details, message }`. The
`code` and the keys of `details` are stable; `message` is a short English sentence for people and
may change. Errors are `parse-error`, `unsupported-syntax`, `unbalanced-transaction`,
`assertion-failed`, `booking-error`, `account-not-open` and others; warnings are
`ignored-syntax`. [spec/README.md](spec/README.md) lists every code and its `details`.

### What is refused or ignored

A construct ledgermill cannot reproduce is never guessed: it is an `unsupported-syntax` error on
the line (and the transaction is not returned), or an `ignored-syntax` warning when hledger
itself does nothing with it.

- hledger: `include`, timeclock, timedot and CSV files, dates with no year when no `Y` is in force,
  and a few more are refused. Automated (`=`) and periodic (`~`) rules are read and then left
  out, with a warning each, because hledger applies them only with `--auto` and `--forecast`.
  [spec/hledger/syntax.md](spec/hledger/syntax.md) lists every construct and its class.
- Beancount: `include`, `plugin`, options outside a harmless list, AVERAGE booking, arithmetic in
  numbers, partial costs and a few more are refused, and several Beancount files, or Beancount
  and hledger files, cannot be loaded together yet. A Beancount file with a refused construct
  lists no transaction. `document` directives are read and skipped with a warning.
  [spec/README.md](spec/README.md) and [spec/beancount/syntax.md](spec/beancount/syntax.md)
  have the details.
- `format` and the valuation reads answer only for what ledgermill reads; see their spec notes.

### Release gate

From the release-gate workflow on commit `fe79e24` (run 40360104607, 2026-10-26) and the owner's
own runs.

- Random hledger journals (5,000): 0 mismatches (2,287 that hledger accepts, compared on
  transactions, postings, prices and holdings; the rest rejected by both, 45 skipped)
- Random Beancount journals (5,000): 0 mismatches (transactions, prices, cost, value and holdings)
- Random price networks (5,000): 0 mismatches
- Corpus comparison: 11,797 pass, 0 fail, 30 skip
- Every interface method compared: yes (`harness/interface.test.ts`)
- Desktop bench: first release, so this is the baseline (CI runner, 10,000 transactions):
  load+transactions 80.1 ms (hledger), 118.3 ms (Beancount); start 15.4 ms
- Stress test, 100,000 transactions: ok, no crash (peak wasm memory 930 MB hledger, 578 MB Beancount)
- Similarity check score: max 0.102 (JPlag against rustledger 0.24.0; fail at 0.15), 0 fail,
  8 warn; copydetect max 0.275 (information only)
- Real journals: pass
- Device targets: met
