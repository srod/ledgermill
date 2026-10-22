# bean-check's verdict on every journal of the corpus

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

## Rule

`bean-check` accepts a journal (exit 0, nothing printed) or rejects it (exit 1, one or more errors
printed to stderr, `FILE:LINE: MESSAGE`). `check()` has to give the same verdict, and as its first
error the first one `bean-check` prints, on the same file and line (check-order.md says which that is,
and check-errors.md, balance-directive.md, pad.md, balancing.md, lots-and-booking.md and
inferred-cost-and-price.md say on which line each error is reported). The harness compares the verdict
only (README of the harness); this note records both, for every journal that is in the corpus:
`harness/journals/app/*.beancount`, `probes/*.beancount`, `probes/refused/*.beancount` and
`probes/pending/*.beancount` (in `spec/beancount/`).

**Accepted**: 89 of the 141 journals. That is all 12 of `harness/journals/app/*.beancount` and every
probe not listed below. The list is what `verdicts.sh` prints with `accepts`.

**Rejected**: the 52 below, with the first line `bean-check` printed (the directory is cut from the
file name). The last column is what the harness says today: `both fail` (ledgermill reports an error
too, so the verdict agrees; the line is not compared yet), `no error yet` (ledgermill reports none:
`SKIP`, "reference rejects, check() comes later"), `unsupported (SKIP)` (ledgermill refuses the file
for `pad`/`balance`, to build), or not run (`refused/` and `pending/` are outside the harness).

## Probe

All the journals under `harness/journals/app/` and `probes/`, `probes/refused/`, `probes/pending/`.

Command:

```sh
spec/beancount/verdicts.sh
```

run from the repository root; it prints one line per journal: `FILE | accepts |` or
`FILE | rejects | FIRST-ERROR-LINE`. (`bean-check FILE` is the only tool it runs.)

## Output

Rejected journals (`probes/` unless a directory is given):

| Journal | First line bean-check printed | Harness today |
|---|---|---|
| `balance-directive-account-errors.beancount` | balance-directive-account-errors.beancount:6: Invalid currency 'USD' for Balance directive: | unsupported (SKIP) |
| `balance-directive-duplicates.beancount` | balance-directive-duplicates.beancount:11: Balance failed for 'Assets:Bank': expected 1 USD != accumulated 0 USD (1 too little) | unsupported (SKIP) |
| `balance-directive-fails.beancount` | balance-directive-fails.beancount:29: Balance failed for 'Assets:G': expected 5 USD != accumulated 0 USD (5 too little) | both fail |
| `balance-directive-rejected-transactions.beancount` | balance-directive-rejected-transactions.beancount:13: You may not have more than one auto-posting per currency | unsupported (SKIP) |
| `balance-directive-syntax.beancount` | balance-directive-syntax.beancount:7: syntax error, unexpected LCURL, expecting end of file or EOL | both fail |
| `balance-fails-integers.beancount` | balance-fails-integers.beancount:4: Transaction does not balance: (-0.4 EUR) | both fail |
| `balance-fails-price.beancount` | balance-fails-price.beancount:4: Transaction does not balance: (-0.005 EUR) | both fail |
| `balance-fails-tolerance.beancount` | balance-fails-tolerance.beancount:4: Transaction does not balance: (-0.011 EUR) | both fail |
| `balance-fails-two-elided.beancount` | balance-fails-two-elided.beancount:6: You may not have more than one auto-posting per currency | both fail |
| `balance-total-price-rounding.beancount` | balance-total-price-rounding.beancount:4: Transaction does not balance: (-0.000000000000000000000000001 EUR) | both fail |
| `booking-default-is-strict.beancount` | booking-default-is-strict.beancount:9: Ambiguous matches for "-1 BTC {}": 1 BTC {10 EUR, 2026-01-02}, 1 BTC {20 EUR, 2026-01-02} | both fail |
| `booking-strict-ambiguous.beancount` | booking-strict-ambiguous.beancount:9: Ambiguous matches for "-1 BTC {}": 1 BTC {10 EUR, 2026-01-02}, 1 BTC {20 EUR, 2026-01-02} | both fail |
| `check-account-dates.beancount` | check-account-dates.beancount:6: Invalid reference to inactive account 'Expenses:Food' | no error yet |
| `check-account-names.beancount` | check-account-names.beancount:5: Invalid account name: Asset:Typo | both fail |
| `check-account-unknown.beancount` | check-account-unknown.beancount:3: Invalid reference to unknown account 'Expenses:Food' | no error yet |
| `check-close-errors.beancount` | check-close-errors.beancount:6: Unopened account Assets:B is being closed | no error yet |
| `check-commodity-duplicate.beancount` | check-commodity-duplicate.beancount:2: Duplicate commodity directives for 'EUR' | no error yet |
| `check-currency-constraint.beancount` | check-currency-constraint.beancount:6: Invalid currency USD for account 'Expenses:Food' | no error yet |
| `check-directive-accounts.beancount` | check-directive-accounts.beancount:4: Invalid reference to unknown account 'Assets:Never' | no error yet |
| `check-directive-dates.beancount` | check-directive-dates.beancount:8: Invalid reference to inactive account 'Assets:Late' | unsupported (SKIP) |
| `check-document-missing.beancount` | check-document-missing.beancount:2: File does not exist: "/nonexistent/statement.pdf" | no error yet |
| `check-open-duplicate.beancount` | check-open-duplicate.beancount:6: Duplicate open directive for Assets:C | no error yet |
| `check-order-dates.beancount` | check-order-dates.beancount:18: Invalid reference to unknown account 'Assets:UnBal' | unsupported (SKIP) |
| `check-order-phases.beancount` | check-order-phases.beancount:39: Invalid account name: Foo:Bad | both fail |
| `check-parse-errors.beancount` | check-parse-errors.beancount:7: Invalid token: 'this' | both fail |
| `check-tags-and-meta.beancount` | check-tags-and-meta.beancount:1: Attempting to pop absent tag: 'never-pushed' | both fail |
| `lots-inferred-cost-currency-without-payment.beancount` | lots-inferred-cost-currency-without-payment.beancount:5: Transaction does not balance: (-50.00 USD) | both fail |
| `lots-inferred-cost-elided-posting.beancount` | lots-inferred-cost-elided-posting.beancount:6: Failed to categorize posting 1 | both fail |
| `lots-inferred-cost-integers.beancount` | lots-inferred-cost-integers.beancount:6: Transaction does not balance: (-0.000000000000000000000000001 EUR) | both fail |
| `lots-inferred-cost-negative-result.beancount` | lots-inferred-cost-negative-result.beancount:7: Cost is negative: "Cost(number=Decimal('-12.50'), currency='EUR', date=datetime.date(2026, 1, 2), label=None)" | both fail |
| `lots-inferred-cost-price-currency.beancount` | lots-inferred-cost-price-currency.beancount:6: Transaction does not balance: (-50.00 EUR) | both fail |
| `lots-inferred-cost-two-currencies.beancount` | lots-inferred-cost-two-currencies.beancount:6: Failed to categorize posting 1 | both fail |
| `lots-inferred-cost-two-lots.beancount` | lots-inferred-cost-two-lots.beancount:8: Too many missing numbers for currency group 'EUR' | both fail |
| `lots-inferred-cost-unbalanced.beancount` | lots-inferred-cost-unbalanced.beancount:6: Transaction does not balance: (-10.00 USD) | both fail |
| `lots-inferred-cost-zero-units.beancount` | lots-inferred-cost-zero-units.beancount:4: Transaction does not balance: (-5.00 EUR) | both fail |
| `pad-account-errors.beancount` | pad-account-errors.beancount:17: Invalid currency 'USD' for Balance directive: | unsupported (SKIP) |
| `pad-child-balance-parent.beancount` | pad-child-balance-parent.beancount:8: Unused Pad entry | unsupported (SKIP) |
| `pad-consumed.beancount` | pad-consumed.beancount:8: Unused Pad entry | unsupported (SKIP) |
| `pad-lots-at-cost.beancount` | pad-lots-at-cost.beancount:13: Attempt to pad an entry with cost for balance: (3 BTC {10 EUR, 2026-01-02}, 2 BTC {12 EUR, 2026-01-03}) | unsupported (SKIP) |
| `pad-parent-balance-child.beancount` | pad-parent-balance-child.beancount:12: Balance failed for 'Assets:Bank:Sub': expected 500.00 EUR != accumulated 100.00 EUR (400.00 too little) | unsupported (SKIP) |
| `pad-rejected-transactions.beancount` | pad-rejected-transactions.beancount:13: You may not have more than one auto-posting per currency | unsupported (SKIP) |
| `pad-same-date-as-balance.beancount` | pad-same-date-as-balance.beancount:4: Unused Pad entry | unsupported (SKIP) |
| `pad-unused.beancount` | pad-unused.beancount:9: Unused Pad entry | unsupported (SKIP) |
| `prices-inferred-currency-two-groups.beancount` | prices-inferred-currency-two-groups.beancount:8: Failed to categorize posting 1 | both fail |
| `prices-inferred-other-currency.beancount` | prices-inferred-other-currency.beancount:6: Transaction does not balance: (-50.00 EUR) | both fail |
| `prices-inferred-two-currencies.beancount` | prices-inferred-two-currencies.beancount:6: Transaction does not balance: (-10.00 USD) | both fail |
| `prices-inferred-wrong-sign.beancount` | prices-inferred-wrong-sign.beancount:6: Transaction does not balance: (100.00 EUR) | both fail |
| `refused/average-booking.beancount` | average-booking.beancount:8: AVERAGE method is not supported | not run (refused) |
| `refused/cost-merge.beancount` | cost-merge.beancount:5: Cost merging is not supported yet | not run (refused) |
| `refused/include.beancount` | <load>:0: File glob "other.beancount" does not match any files | not run (refused) |
| `refused/inferred-price-with-cost.beancount` | inferred-price-with-cost.beancount:12: Too many missing numbers for currency group 'EUR' | not run (refused) |
| `refused/option-tolerance.beancount` | option-tolerance.beancount:7: Transaction does not balance: (-0.4 EUR) | not run (refused) |
