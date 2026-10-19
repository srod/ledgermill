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
ledger.transactions()   booked transactions with their file and line
ledger.check()          problems found, as structured findings
ledger.free()
```

More questions follow: values at a date in a currency, holdings, postings, prices, and a
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
