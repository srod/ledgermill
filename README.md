# ledgermill

Ledger engines for plain-text accounting: one for hledger journals, one for Beancount ledgers.
They are written in Rust, compile to WebAssembly for browsers, and are built to be embedded in
native apps later. They compute with exact decimals and answer through a typed interface.

**Status: early work.** Nothing is released yet. The first goal is to give the same answers as
hledger 1.52.4 and Beancount 3.2.3 for everything its first user, a personal finance app, reads from a
journal.

## What it does

```text
load(files) → ledger
ledger.transactions()   booked transactions with their file and line (Beancount: padding included)
ledger.postings()       one row per posting and commodity, with its transaction's header fields
ledger.prices()         hledger: the `P` directives, as `hledger prices` lists them (Beancount: empty for now)
ledger.check()          problems found, as structured findings
ledger.atCost()         hledger: the transactions as `print --cost` gives them
ledger.value(date, commodity)   hledger: the transactions as `print --value=DATE,COMMODITY` gives them
ledger.holdings(query)  hledger: per account and commodity, the units, cost, value and cost value (real postings only)
ledger.free()
```

`postings()` gives, for hledger, what `print --location -O csv` lists: one row per posting and commodity (an elided
posting gets a row for each commodity it was given), by date, with `flag` the transaction's status (`""`, `"*"` or
`"!"`), `account` without the `()` or `[]` of a virtual posting and its `kind`, `number` the exact quantity (never rounded, with a
point as decimal mark) and `line` the line the posting itself is written on, which hledger does not give. `prices()` gives
each `P` directive with its date, commodity, rate (the commodity and the exact quantity, with the places written), file and line,
sorted by date. A `P` line whose rate is followed by a price, a lot or a label is refused with an `unsupported-syntax` finding
(`price-annotation`).

In Rust these are `Ledger::at_cost()` and `Ledger::value(date, commodity)`; both return `Result<Vec<Transaction>, Finding>`.
`at_cost` replaces every cost by an amount in the cost's commodity and drops the conversion postings; `value` converts
every amount at the prices (`P` lines) known on `date` (`YYYY-MM-DD`), by the shortest path of prices, with the
arithmetic hledger uses. A question ledgermill cannot answer as hledger does is an `Err` with an `unsupported-syntax`
finding, not a guess: an `at_cost` on a transaction that has pairs of conversion postings together with several priced
postings or a balance assignment, a `value` in no commodity, and both on a Beancount ledger (its valuation is not built). A date
that is not `YYYY-MM-DD` is an `invalid-argument` finding. In the wasm binding the `Err` is thrown as an `Error` whose message is
the finding as JSON.

`holdings(query)` takes `{ commodity, end, today }` (JSON in the wasm binding, `HoldingsQuery` in Rust): the commodity V to value
in, `end` an optional exclusive `YYYY-MM-DD` like `-e`, and `today` the date today, because the core has no clock. It gives one
`Holding` per account and commodity held, ordered by account name then commodity: `units` and the lists `cost`, `value` and
`costValue` (`cost_value` in Rust), exact, every sum rounded to ten places once. Only real postings count; no account-prefix
filter and no symbol aliases (callers do those). It refuses like `at_cost` and `value` (`unsupported-syntax`) and answers an
`invalid-argument` finding for an `end` or `today` that is not a date; see `spec/hledger/holdings.md`.

For hledger, balance assertions (`=`, `==`, `=*`, `==*`) and assignments are read: `check()` reports a failed
assertion (`assertion-failed`, with the account, commodity and the exact expected and actual amounts) and the
transactions come with the amounts the assignments filled in. Its first error is the one hledger prints first
(file and line), not the first by line. The automated (`=`) and periodic (`~`) rules are read, with the
errors hledger gives for them, and left out: hledger only applies them with `--auto` and `--forecast`, so
each one gets an `ignored-syntax` warning and changes nothing. A few things are refused with an
`unsupported-syntax` error instead (`include`, timeclock, timedot and CSV files, dates with no year and no
`Y`, and a few more); [spec/hledger/syntax.md](spec/hledger/syntax.md) lists every construct and what
ledgermill does with it.

For Beancount, `check()` gives the verdict `bean-check` gives, with the same first error (file and line):
unknown or closed accounts, balance assertions and their tolerance, unused pads, currency
constraints, duplicates, unbalanced transactions and more (`spec/README.md` lists the codes).
Beancount `pad` is read and its padding transactions are built; `include`, `plugin`, unusual options
and AVERAGE booking are refused with a finding.

More questions follow: Beancount prices and valuation, and a
formatter that only realigns whitespace. See the issue tracker for the plan.

ledgermill is not a command-line clone of hledger or Beancount. It does not print their
reports and does not copy their messages; it answers questions about a ledger.

## How we know it is right

The real `hledger` and Beancount tools are our **reference tools**. A comparison harness runs
them on journals we write ourselves, decodes their output, and compares it with ledgermill's
answers value by value. Every rule ledgermill follows is written down as a spec note with the
journal, command and output that show it (`spec/`).

## Clean room

ledgermill is an independent implementation. Nobody working on it reads the source code of
hledger, Beancount, rustledger or Ledger. Behaviour is learned by running those programs on
journals we wrote and recording what they do. Their manuals may be read to learn rules and
names, but no text, table or example is copied from them. The rules are in
[CONTRIBUTING.md](CONTRIBUTING.md); what each contributor has been exposed to is recorded in
[EXPOSURE.md](EXPOSURE.md); the remaining legal risk and our answer to a complaint are in
[RISK.md](RISK.md).

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
