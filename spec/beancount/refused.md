# Refused constructs

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

Status: **built**. `pad` and `balance` are no longer refused; the rest of this note is as it was.

## Rule

`pad` and `balance` are not refused: they are read (pad.md, balance-directive.md).

ledgermill refuses what could change which transactions or postings come out, or their figures,
and that it does not reproduce yet. It reports `unsupported-syntax` (`details.construct` names it,
see spec/README.md), returns **no** transaction for the file (a partial answer would look
complete), and the harness shows SKIP. What bean-check says about each probe:

| Probe | bean-check | ledgermill `construct` |
|---|---|---|
| `average-booking` | rejects: "AVERAGE method is not supported", at the sale | `average-booking` (`details.account`) |
| `cost-merge` | rejects: "Cost merging is not supported yet" | `cost-merge` |
| `cost-partial-per-unit`, `cost-partial-total` | accept: the missing part of `{2 # EUR}` / `{# 1 EUR}` is worked out | `cost-partial` |
| `inferred-price-with-cost` | rejects: "Too many missing numbers" | `inferred-price` |
| `include` | rejects here, the file is missing: "File glob ... does not match any files" | `include` |
| `number-expression` | accepts: `(1 + 2) * 3 EUR` is `9`; the same in a `price` directive (price-table.md) | `number-expression` |
| `option-tolerance` | rejects: "does not balance", the tolerance options do not rescue it | `option` (`details.name`) |
| `option-unknown` | accepts, the account root is renamed | `option` (`details.name`) |
| `plugin` | accepts, the plugin runs | `plugin` |

Why each is refused: `include` and `plugin` bring in files or rewrite the entries; a
number expression needs the expression grammar; an option other than the ones in options-and-directives.md may
change tolerances, account names or booking; `cost-merge` (`{..., *}`) and AVERAGE booking merge
lots. ledgermill refuses a rejected journal as unsupported rather than as an error when the
construct is one it would refuse anyway, so the harness reports SKIP, not "both fail".

Also refused, without a probe because nothing needs a reference to confirm it:

- `number-too-large`: more than 22 characters in a number. `number-too-precise`: a figure that
  needs more digits than ledgermill handles.
- `inferred-price`: a price with its number or currency left out, next to a cost on the same
  posting, or with zero units. `inferred-cost`: a cost to work out in a transaction that also has
  a reduction with an open cost, or with zero units and nothing to pay. Everything else about
  both is built; see inferred-cost-and-price.md, which lists what stays refused.
- `include`: only a single root file is read, so an `include` cannot be followed. Several files in one
  `load` are not refused any more: each Beancount file is its own ledger (several-files.md).

## Probe

`probes/refused/average-booking.beancount`, `probes/refused/cost-merge.beancount`, `probes/refused/include.beancount`, `probes/refused/number-expression.beancount`, `probes/refused/option-tolerance.beancount`, `probes/refused/option-unknown.beancount`, `probes/refused/plugin.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/refused/average-booking.beancount
spec/beancount/show.sh spec/beancount/probes/refused/cost-merge.beancount
spec/beancount/show.sh spec/beancount/probes/refused/include.beancount
spec/beancount/show.sh spec/beancount/probes/refused/number-expression.beancount
spec/beancount/show.sh spec/beancount/probes/refused/option-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/refused/option-unknown.beancount
spec/beancount/show.sh spec/beancount/probes/refused/plugin.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

average-booking:

```text
bean-check: average-booking.beancount:8: AVERAGE method is not supported
20c573,transaction,5,2026-01-02,*,,buy
ef883d,transaction,8,2026-01-10,*,,sell
20c573,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
20c573,Assets:Bank,,-10,EUR,,,,,,,7
```

cost-merge:

```text
bean-check: cost-merge.beancount:5: Cost merging is not supported yet
77b54e,transaction,4,2026-01-02,*,,a merge marker
77b54e,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,5
77b54e,Assets:Bank,,-10,EUR,,,,,,,6
```

include:

```text
bean-check: <load>:0: File glob "other.beancount" does not match any files
```

number-expression:

```text
bean-check: (no output, exit 0)
3cb4ff,transaction,4,2026-01-02,*,,arithmetic in an amount
3cb4ff,Assets:A,,9,EUR,,,,,,,5
3cb4ff,Assets:B,,-9,EUR,,,,,,,6
```

option-tolerance:

```text
bean-check: option-tolerance.beancount:7: Transaction does not balance: (-0.4 EUR)
6e2c76,transaction,7,2026-01-02,*,,a looser tolerance for EUR
6e2c76,Assets:A,,10,EUR,,,,,,,8
6e2c76,Assets:B,,-10.4,EUR,,,,,,,9
```

option-unknown:

```text
bean-check: (no output, exit 0)
ebffa0,transaction,6,2026-01-02,*,,renamed account types
ebffa0,Expenses:Food,,1,EUR,,,,,,,7
ebffa0,Actifs:Bank,,-1,EUR,,,,,,,8
```

plugin:

```text
bean-check: (no output, exit 0)
bd4aba,transaction,3,2026-01-02,*,,no open directives needed with the plugin
bd4aba,Assets:A,,1,EUR,,,,,,,4
bd4aba,Assets:B,,-1,EUR,,,,,,,5
```

