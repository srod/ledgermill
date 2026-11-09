# Refused constructs

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

Status: **built**. `pad` and `balance` are no longer refused, and neither is arithmetic in a number
(number-expressions.md); only a few shapes of it still are, listed in that note's "What ledgermill does
with it". The rest of this note is as it was.

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
| `number-expression-stacked-sign` | accepts: `--4 EUR` is `4` (a sign in front of a sign, no parentheses) | `number-expression` |
| `number-expression-tolerance-sum` | accepts: `0 ~ 1 + 2 EUR`, a tolerance that is a sum, no parentheses | `number-expression` |
| `number-expression-operands-too-large` | accepts: operands of 28 and 29 digits, `(1234567890123456789012345678 + 0.5) EUR` is `1234567890123456789012345678` (a tie to the even digit) | `number-too-large` |
| `number-expression-result-too-large` | accepts: `(12345678901234567890 * 12345678901234567890) EUR` is `1.524157875323883675019051999E+38` | `number-too-precise` |
| `option-tolerance` | rejects: "does not balance", the tolerance options do not rescue it | `option` (`details.name`) |
| `option-unknown` | accepts, the account root is renamed | `option` (`details.name`) |
| `plugin-check-commodity` | accepts, the plugin runs (it has nothing to say here) | `plugin` (`details.name`) |
| `plugin-auto-accounts-config` | rejects: the plugin takes no configuration string, `<load>:0: Error applying plugin` | `plugin` (`details.name`) |
| `plugin-unknown-module` | rejects: `<load>:0: Error importing` the module | `plugin` (`details.name`) |
| `plugin-escaped-name` | accepts, the plugin runs: `\o` in the module name is `o` (auto-accounts.md) | `plugin` (`details.name`, as written) |
| `plugin-escaped-control` | rejects: `\n` in the module name is a line feed, `<load>:0: Error importing` | `plugin` (`details.name`, as written) |

Why each is refused: `include` brings in files, and a `plugin` rewrites or checks the entries (only `beancount.plugins.auto_accounts` is built, auto-accounts.md; any other module, with or without a configuration string, is refused, and `details.name` is its module name); an option other than the ones in options-and-directives.md may
change tolerances, account names or booking; `cost-merge` (`{..., *}`) and AVERAGE booking merge
lots. A sign in front of a sign and a tolerance that is a sum are read by the reference but not by the
parser ledgermill is built on, which takes one sign and a plain number there; the same two inside
parentheses are read (number-expressions.md). A result that needs more digits than the booking code holds
is `number-too-precise`, as any other number is. ledgermill refuses a rejected journal as unsupported rather than as an error when the
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

`probes/refused/average-booking.beancount`, `probes/refused/cost-merge.beancount`, `probes/refused/include.beancount`, `probes/refused/number-expression-operands-too-large.beancount`, `probes/refused/number-expression-result-too-large.beancount`, `probes/refused/number-expression-stacked-sign.beancount`, `probes/refused/number-expression-tolerance-sum.beancount`, `probes/refused/option-tolerance.beancount`, `probes/refused/option-unknown.beancount`, `probes/refused/plugin-auto-accounts-config.beancount`, `probes/refused/plugin-check-commodity.beancount`, `probes/refused/plugin-escaped-control.beancount`, `probes/refused/plugin-escaped-name.beancount`, `probes/refused/plugin-unknown-module.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/refused/average-booking.beancount
spec/beancount/show.sh spec/beancount/probes/refused/cost-merge.beancount
spec/beancount/show.sh spec/beancount/probes/refused/include.beancount
spec/beancount/show.sh spec/beancount/probes/refused/number-expression-operands-too-large.beancount
spec/beancount/show.sh spec/beancount/probes/refused/number-expression-result-too-large.beancount
spec/beancount/show.sh spec/beancount/probes/refused/number-expression-stacked-sign.beancount
spec/beancount/show.sh spec/beancount/probes/refused/number-expression-tolerance-sum.beancount
spec/beancount/show.sh spec/beancount/probes/refused/option-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/refused/option-unknown.beancount
spec/beancount/show.sh spec/beancount/probes/refused/plugin-check-commodity.beancount
spec/beancount/show-opens.sh spec/beancount/probes/refused/plugin-auto-accounts-config.beancount spec/beancount/probes/refused/plugin-unknown-module.beancount
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

number-expression-operands-too-large:

```text
bean-check: (no output, exit 0)
4b3972,transaction,4,2026-02-17,*,,a tie rounds to the even digit: down
c76229,transaction,8,2026-02-18,*,,a tie rounds to the even digit: up
a7ffa2,transaction,12,2026-02-19,*,,not a tie
cb83f5,transaction,16,2026-02-20,*,,29 digits in an operand
4b3972,Assets:A,,1234567890123456789012345678,EUR,,,,,,,5
c76229,Assets:A,,1234567890123456789012345680,EUR,,,,,,,9
a7ffa2,Assets:A,,1234567890123456789012345679,EUR,,,,,,,13
cb83f5,Assets:A,,1.234567890123456789012345679E+28,EUR,,,,,,,17
```

number-expression-result-too-large:

```text
bean-check: (no output, exit 0)
463924,transaction,3,2026-01-02,*,,a product of 39 digits
463924,Assets:A,,1.524157875323883675019051999E+38,EUR,,,,,,,4
463924,Assets:B,,-1.524157875323883675019051999E+38,EUR,,,,,,,5
```

number-expression-stacked-sign:

```text
bean-check: (no output, exit 0)
076a89,transaction,4,2026-01-02,*,,two signs in a row without parentheses
076a89,Assets:A,,4,EUR,,,,,,,5
076a89,Assets:B,,-4,EUR,,,,,,,6
```

number-expression-tolerance-sum:

```text
bean-check: (no output, exit 0)
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

plugin-check-commodity:

```text
bean-check: (no output, exit 0)
97bd44,transaction,7,2026-01-02,*,,the plugin checks every commodity has a directive; this one does
97bd44,Assets:A,,1,EUR,,,,,,,8
97bd44,Assets:B,,-1,EUR,,,,,,,9
```

plugin-auto-accounts-config and plugin-unknown-module (`show-opens.sh`, a Python traceback replaced by a marker):

```text
bean-check: <load>:0: Error applying plugin "beancount.plugins.auto_accounts": [traceback hidden]
bean-check: plugin-auto-accounts-config.beancount:3: Invalid reference to unknown account 'Assets:B'
bean-check: plugin-auto-accounts-config.beancount:3: Invalid reference to unknown account 'Assets:A'
bean-check: <load>:0: Error importing "no.such.module": [traceback hidden]
```

