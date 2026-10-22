# Refused constructs

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

ledgermill refuses what could change which transactions or postings come out, or their figures,
and that it does not reproduce yet. It reports `unsupported-syntax` (`details.construct` names it,
see spec/README.md), returns **no** transaction for the file (a partial answer would look
complete), and the harness shows SKIP. What bean-check says about each probe:

| Probe | bean-check | ledgermill `construct` |
|---|---|---|
| `average-booking` | rejects: "AVERAGE method is not supported", at the sale | `average-booking` (`details.account`) |
| `balance` | accepts | `balance` |
| `cost-merge` | rejects: "Cost merging is not supported yet" | `cost-merge` |
| `include` | rejects here, the file is missing: "File glob ... does not match any files" | `include` |
| `number-expression` | accepts: `(1 + 2) * 3 EUR` is `9` | `number-expression` |
| `option-tolerance` | rejects: "does not balance", the tolerance options do not rescue it | `option` (`details.name`) |
| `option-unknown` | accepts, the account root is renamed | `option` (`details.name`) |
| `pad` | accepts, and adds a transaction (flag `P`) | `pad` |
| `plugin` | accepts, the plugin runs | `plugin` |

Why each is refused: `include` and `plugin` bring in files or rewrite the entries; `pad` adds a
transaction no line of the file shows; `balance` asserts what the running inventory holds; a
number expression needs the expression grammar; an option other than the ones in options-and-directives.md may
change tolerances, account names or booking; `cost-merge` (`{..., *}`) and AVERAGE booking merge
lots. ledgermill refuses a rejected journal as unsupported rather than as an error when the
construct is one it would refuse anyway, so the harness reports SKIP, not "both fail".

Also refused, without a probe because nothing needs a reference to confirm it:

- `number-too-large`: more than 22 characters in a number. `number-too-precise`: a figure that
  needs more digits than ledgermill handles.
- `inferred-price`: a price with its number or currency left out. `inferred-cost`: a new lot
  whose cost is left to be worked out. What the reference does with both is in
  inferred-cost-and-price.md; they stay refused until that is built.
- `several-beancount-files` and `mixed-formats`: more than one Beancount file in one `load`, or
  Beancount and hledger files together (reported at line 1: of the first Beancount file for a mix,
  of the second one for several). Only a single root file is read, so an `include` cannot be followed either.

## Probe

`probes/refused/average-booking.beancount`, `probes/refused/balance.beancount`, `probes/refused/cost-merge.beancount`, `probes/refused/include.beancount`, `probes/refused/number-expression.beancount`, `probes/refused/option-tolerance.beancount`, `probes/refused/option-unknown.beancount`, `probes/refused/pad.beancount`, `probes/refused/plugin.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/refused/average-booking.beancount
spec/beancount/show.sh spec/beancount/probes/refused/balance.beancount
spec/beancount/show.sh spec/beancount/probes/refused/cost-merge.beancount
spec/beancount/show.sh spec/beancount/probes/refused/include.beancount
spec/beancount/show.sh spec/beancount/probes/refused/number-expression.beancount
spec/beancount/show.sh spec/beancount/probes/refused/option-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/refused/option-unknown.beancount
spec/beancount/show.sh spec/beancount/probes/refused/pad.beancount
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

balance:

```text
bean-check: (no output, exit 0)
53c9f8,transaction,4,2026-01-02,*,,pay
53c9f8,Assets:Bank,,500.00,EUR,,,,,,,5
53c9f8,Income:Pay,,-500.00,EUR,,,,,,,6
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

pad:

```text
bean-check: (no output, exit 0)
800fe5,transaction,4,2026-01-02,P,,(Padding inserted for Balance of 500.00 EUR for difference 500.00 EUR)
800fe5,Assets:Bank,,500.00,EUR,,,,,,,5
800fe5,Equity:Opening,,-500.00,EUR,,,,,,,5
```

plugin:

```text
bean-check: (no output, exit 0)
bd4aba,transaction,3,2026-01-02,*,,no open directives needed with the plugin
bd4aba,Assets:A,,1,EUR,,,,,,,4
bd4aba,Assets:B,,-1,EUR,,,,,,,5
```

