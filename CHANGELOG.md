# Changelog

All notable changes to ledgermill are written here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## 0.1.0 – not released yet

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

Filled in at release time, from the release-gate workflow and the owner's own runs. Nothing below
has been run yet.

- Random hledger journals (5,000): TBD at release (0 mismatches expected)
- Random Beancount journals (5,000): TBD at release (0 mismatches expected)
- Random price networks (5,000): TBD at release (0 mismatches expected)
- Corpus comparison: TBD at release
- Every interface method compared: TBD at release
- Desktop bench: TBD at release (first release: this is the baseline, nothing to compare with)
- Stress test, 100,000 transactions: TBD at release
- Similarity check score: TBD at release
- Real journals: pass – TBD at release (the owner fills this in; no numbers)
- Device targets: met – TBD at release (the owner fills this in; no numbers, no device names)
