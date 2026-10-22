# bean-check's verdict on every journal of the corpus

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Status: built.

## Rule

`bean-check` accepts a journal (exit 0, nothing printed) or rejects it (exit 1, one or more errors
printed to stderr, `FILE:LINE: MESSAGE`). `check()` has to give the same verdict, and as its first
error the first one `bean-check` prints, on the same file and line (check-order.md says which that is,
and check-errors.md, balance-directive.md, pad.md, balancing.md, lots-and-booking.md and
inferred-cost-and-price.md say on which line each error is reported). The harness compares the verdict and the file and
line of the first error (README of the harness); this note records both, for every journal that is in the corpus:
`harness/journals/app/*.beancount`, `probes/*.beancount` and `probes/refused/*.beancount` (in `spec/beancount/`).

**Accepted**: 91 of the 154 journals. That is all 12 of `harness/journals/app/*.beancount` and every
probe not listed below. The list is what `verdicts.sh` prints with `accepts`.

**Rejected**: the 63 below, with the first line `bean-check` printed (the directory is cut from the
file name). The last column is what the harness says now: `PASS, same first line` (ledgermill's first error
finding is on the same file and line), `SKIP` for the one journal whose only error is a `document` file that
is not there (ledgermill does not look at files and warns instead), or not run (`refused/` is outside the
harness).

## Probe

All the journals under `harness/journals/app/` and `probes/` and `probes/refused/`.

Command:

```sh
spec/beancount/verdicts.sh
```

run from the repository root; it prints one line per journal: `FILE | accepts |` or
`FILE | rejects | FIRST-ERROR-LINE`. (`bean-check FILE` is the only tool it runs.)

## Output

Rejected journals (`probes/` unless a directory is given):

| Journal | First line bean-check printed | Harness now |
|---|---|---|
| `balance-directive-account-errors.beancount` | balance-directive-account-errors.beancount:6: Invalid currency 'USD' for Balance directive:  | PASS, same first line |
| `balance-directive-duplicates.beancount` | balance-directive-duplicates.beancount:11: Balance failed for 'Assets:Bank': expected 1 USD != accumulated 0 USD (1 too little) | PASS, same first line |
| `balance-directive-fails.beancount` | balance-directive-fails.beancount:29: Balance failed for 'Assets:G': expected 5 USD != accumulated 0 USD (5 too little) | PASS, same first line |
| `balance-directive-rejected-transactions.beancount` | balance-directive-rejected-transactions.beancount:13: You may not have more than one auto-posting per currency | PASS, same first line |
| `balance-directive-syntax.beancount` | balance-directive-syntax.beancount:7: syntax error, unexpected LCURL, expecting end of file or EOL | PASS, same first line |
| `balance-fails-integers.beancount` | balance-fails-integers.beancount:4: Transaction does not balance: (-0.4 EUR) | PASS, same first line |
| `balance-fails-price.beancount` | balance-fails-price.beancount:4: Transaction does not balance: (-0.005 EUR) | PASS, same first line |
| `balance-fails-tolerance.beancount` | balance-fails-tolerance.beancount:4: Transaction does not balance: (-0.011 EUR) | PASS, same first line |
| `balance-fails-two-elided.beancount` | balance-fails-two-elided.beancount:6: You may not have more than one auto-posting per currency | PASS, same first line |
| `balance-total-price-rounding.beancount` | balance-total-price-rounding.beancount:4: Transaction does not balance: (-0.000000000000000000000000001 EUR) | PASS, same first line |
| `booking-default-is-strict.beancount` | booking-default-is-strict.beancount:9: Ambiguous matches for "-1 BTC {}": 1 BTC {10 EUR, 2026-01-02}, 1 BTC {20 EUR, 2026-01-02} | PASS, same first line |
| `booking-line-elided-after-amounts.beancount` | booking-line-elided-after-amounts.beancount:8: You may not have more than one auto-posting per currency | PASS, same first line |
| `booking-line-missing-number-first.beancount` | booking-line-missing-number-first.beancount:6: Too many missing numbers for currency group 'EUR' | PASS, same first line |
| `booking-line-negative-price.beancount` | booking-line-negative-price.beancount:7: Negative prices are not allowed: -12.5 EUR (see http://furius.ca/beancount/doc/bug-negative-prices for workaround) | PASS, same first line |
| `booking-line-no-matching-lot.beancount` | booking-line-no-matching-lot.beancount:10: No position matches "Posting(account='Assets:Stock', units=-1 BTC, cost=CostSpec(number_per=Decimal('20'), number_total=None, currency='EUR', date=None, label=None, merge=False), price=None, flag=None, meta={'filename': '/Users/rodolphe/Developer/ledgermill/.worktrees/pad-balance-check/spec/beancount/probes/booking-line-no-matching-lot.beancount', 'lineno': 12})" against balance (1 BTC {10 EUR, 2026-01-02}) | PASS, same first line |
| `booking-line-no-units-currency.beancount` | booking-line-no-units-currency.beancount:7: Could not resolve units currency | PASS, same first line |
| `booking-line-not-enough-lots.beancount` | booking-line-not-enough-lots.beancount:10: Not enough lots to reduce "-2 BTC {}": 1 BTC {10 EUR, 2026-01-02} | PASS, same first line |
| `booking-line-price-without-units.beancount` | booking-line-price-without-units.beancount:7: Could not resolve units currency | PASS, same first line |
| `booking-line-second-posting.beancount` | booking-line-second-posting.beancount:8: Failed to categorize posting 2 | PASS, same first line |
| `booking-line-three-elided.beancount` | booking-line-three-elided.beancount:9: You may not have more than one auto-posting per currency | PASS, same first line |
| `booking-line-units-missing-number.beancount` | booking-line-units-missing-number.beancount:7: Too many missing numbers for currency group 'EUR' | PASS, same first line |
| `booking-strict-ambiguous.beancount` | booking-strict-ambiguous.beancount:9: Ambiguous matches for "-1 BTC {}": 1 BTC {10 EUR, 2026-01-02}, 1 BTC {20 EUR, 2026-01-02} | PASS, same first line |
| `check-account-dates.beancount` | check-account-dates.beancount:6: Invalid reference to inactive account 'Expenses:Food' | PASS, same first line |
| `check-account-names.beancount` | check-account-names.beancount:5: Invalid account name: Asset:Typo | PASS, same first line |
| `check-account-unknown.beancount` | check-account-unknown.beancount:3: Invalid reference to unknown account 'Expenses:Food' | PASS, same first line |
| `check-close-errors.beancount` | check-close-errors.beancount:6: Unopened account Assets:B is being closed | PASS, same first line |
| `check-commodity-duplicate.beancount` | check-commodity-duplicate.beancount:2: Duplicate commodity directives for 'EUR' | PASS, same first line |
| `check-currency-constraint.beancount` | check-currency-constraint.beancount:6: Invalid currency USD for account 'Expenses:Food' | PASS, same first line |
| `check-directive-accounts.beancount` | check-directive-accounts.beancount:4: Invalid reference to unknown account 'Assets:Never' | PASS, same first line |
| `check-directive-dates.beancount` | check-directive-dates.beancount:8: Invalid reference to inactive account 'Assets:Late' | PASS, same first line |
| `check-document-missing.beancount` | check-document-missing.beancount:2: File does not exist: "/nonexistent/statement.pdf" | SKIP (a `document` file is not looked at) |
| `check-open-duplicate.beancount` | check-open-duplicate.beancount:6: Duplicate open directive for Assets:C | PASS, same first line |
| `check-order-dates.beancount` | check-order-dates.beancount:18: Invalid reference to unknown account 'Assets:UnBal' | PASS, same first line |
| `check-order-phases.beancount` | check-order-phases.beancount:39: Invalid account name: Foo:Bad | PASS, same first line |
| `check-parse-errors.beancount` | check-parse-errors.beancount:7: Invalid token: 'this' | PASS, same first line |
| `check-tags-and-meta.beancount` | check-tags-and-meta.beancount:1: Attempting to pop absent tag: 'never-pushed' | PASS, same first line |
| `lots-inferred-cost-currency-without-payment.beancount` | lots-inferred-cost-currency-without-payment.beancount:5: Transaction does not balance: (-50.00 USD) | PASS, same first line |
| `lots-inferred-cost-elided-posting.beancount` | lots-inferred-cost-elided-posting.beancount:6: Failed to categorize posting 1 | PASS, same first line |
| `lots-inferred-cost-integers.beancount` | lots-inferred-cost-integers.beancount:6: Transaction does not balance: (-0.000000000000000000000000001 EUR) | PASS, same first line |
| `lots-inferred-cost-negative-result.beancount` | lots-inferred-cost-negative-result.beancount:7: Cost is negative: "Cost(number=Decimal('-12.50'), currency='EUR', date=datetime.date(2026, 1, 2), label=None)" | PASS, same first line |
| `lots-inferred-cost-price-currency.beancount` | lots-inferred-cost-price-currency.beancount:6: Transaction does not balance: (-50.00 EUR) | PASS, same first line |
| `lots-inferred-cost-two-currencies.beancount` | lots-inferred-cost-two-currencies.beancount:6: Failed to categorize posting 1 | PASS, same first line |
| `lots-inferred-cost-two-lots.beancount` | lots-inferred-cost-two-lots.beancount:8: Too many missing numbers for currency group 'EUR' | PASS, same first line |
| `lots-inferred-cost-unbalanced.beancount` | lots-inferred-cost-unbalanced.beancount:6: Transaction does not balance: (-10.00 USD) | PASS, same first line |
| `lots-inferred-cost-zero-units.beancount` | lots-inferred-cost-zero-units.beancount:4: Transaction does not balance: (-5.00 EUR) | PASS, same first line |
| `margin-lines-errors.beancount` | margin-lines-errors.beancount:3: syntax error, unexpected TAG | PASS, same first line |
| `pad-account-errors.beancount` | pad-account-errors.beancount:17: Invalid currency 'USD' for Balance directive:  | PASS, same first line |
| `pad-child-balance-parent.beancount` | pad-child-balance-parent.beancount:8: Unused Pad entry | PASS, same first line |
| `pad-consumed.beancount` | pad-consumed.beancount:8: Unused Pad entry | PASS, same first line |
| `pad-lots-at-cost.beancount` | pad-lots-at-cost.beancount:13: Attempt to pad an entry with cost for balance: (3 BTC {10 EUR, 2026-01-02}, 2 BTC {12 EUR, 2026-01-03}) | PASS, same first line |
| `pad-parent-balance-child.beancount` | pad-parent-balance-child.beancount:12: Balance failed for 'Assets:Bank:Sub': expected 500.00 EUR != accumulated 100.00 EUR (400.00 too little) | PASS, same first line |
| `pad-rejected-transactions.beancount` | pad-rejected-transactions.beancount:13: You may not have more than one auto-posting per currency | PASS, same first line |
| `pad-same-date-as-balance.beancount` | pad-same-date-as-balance.beancount:4: Unused Pad entry | PASS, same first line |
| `pad-unused.beancount` | pad-unused.beancount:9: Unused Pad entry | PASS, same first line |
| `prices-inferred-currency-two-groups.beancount` | prices-inferred-currency-two-groups.beancount:8: Failed to categorize posting 1 | PASS, same first line |
| `prices-inferred-other-currency.beancount` | prices-inferred-other-currency.beancount:6: Transaction does not balance: (-50.00 EUR) | PASS, same first line |
| `prices-inferred-two-currencies.beancount` | prices-inferred-two-currencies.beancount:6: Transaction does not balance: (-10.00 USD) | PASS, same first line |
| `prices-inferred-wrong-sign.beancount` | prices-inferred-wrong-sign.beancount:6: Transaction does not balance: (100.00 EUR) | PASS, same first line |
| `refused/average-booking.beancount` | average-booking.beancount:8: AVERAGE method is not supported | not run (refused) |
| `refused/cost-merge.beancount` | cost-merge.beancount:5: Cost merging is not supported yet | not run (refused) |
| `refused/include.beancount` | <load>:0: File glob "other.beancount" does not match any files | not run (refused) |
| `refused/inferred-price-with-cost.beancount` | inferred-price-with-cost.beancount:12: Too many missing numbers for currency group 'EUR' | not run (refused) |
| `refused/option-tolerance.beancount` | option-tolerance.beancount:7: Transaction does not balance: (-0.4 EUR) | not run (refused) |
