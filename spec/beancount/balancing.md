# Balancing

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

Each group (the weight currency, as in numbers-and-interpolation.md) must sum to zero within a
tolerance. The weight of a posting is its units, units times price, or units times cost (the
cost wins when there are both).

Tolerance: half of the last place of the coarsest amount written in that currency, that is
`5 * 10^-(p+1)` where `p` is the fewest decimal places among the written amounts (integers do not
count). A residual equal to the tolerance passes (`10.00 EUR` against `-10.005 EUR`), a larger one
does not. When no amount of the currency has a decimal place the tolerance is zero: the sum must
be exact.

Not balanced: bean-check reports "Transaction does not balance" at the header line, with the
residual; bean-query still lists the transaction and its postings. ledgermill reports
`unbalanced-transaction` at the header line (`details.residual`: one `{commodity, quantity}` per
group, the sum of the weights, same sign as the reference's) and still returns the transaction.

Two postings without an amount: bean-check reports "more than one auto-posting per currency" on
the second one, and bean-query leaves the transaction out. ledgermill reports `booking-error`
(`details.reason: ambiguous-auto-posting`) at the header line and does not return it.

The total-price case (`3 DOT @@ 10 EUR`, 1e-27 over) is in prices.md.

## Probe

`probes/balance-tolerance.beancount`, `probes/balance-fails-integers.beancount`, `probes/balance-fails-price.beancount`, `probes/balance-fails-tolerance.beancount`, `probes/balance-fails-two-elided.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/balance-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-integers.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-price.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-two-elided.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

balance-tolerance:

```text
bean-check: (no output, exit 0)
f411d6,Assets:A,,10.00,EUR,,,,,,,5
f411d6,Assets:B,,-10.004,EUR,,,,,,,6
80709c,Assets:A,,10.00,EUR,,,,,,,8
80709c,Assets:B,,-10.005,EUR,,,,,,,9
d7c58d,Assets:A,,2.5,STK,,,,,100.25,EUR,11
d7c58d,Assets:B,,-250.625,EUR,,,,,,,12
1d1b44,Assets:A,,2,STK,1.005,EUR,2026-01-06,,,,14
1d1b44,Assets:B,,-2.01,EUR,,,,,,,15
947ab1,Assets:A,,10,EUR,,,,,,,17
947ab1,Assets:B,,-10,EUR,,,,,,,18
```

balance-fails-integers:

```text
bean-check: balance-fails-integers.beancount:4: Transaction does not balance: (-0.4 EUR)
4439f9,Assets:A,,10,EUR,,,,,,,5
4439f9,Assets:B,,-10.4,EUR,,,,,,,6
```

balance-fails-price:

```text
bean-check: balance-fails-price.beancount:4: Transaction does not balance: (-0.005 EUR)
cc292e,Assets:A,,1.5,STK,,,,,3.33,EUR,5
cc292e,Assets:B,,-5,EUR,,,,,,,6
```

balance-fails-tolerance:

```text
bean-check: balance-fails-tolerance.beancount:4: Transaction does not balance: (-0.011 EUR)
2cf913,Assets:A,,1.000,EUR,,,,,,,5
2cf913,Assets:B,,-1.011,EUR,,,,,,,6
```

balance-fails-two-elided:

```text
bean-check: balance-fails-two-elided.beancount:6: You may not have more than one auto-posting per currency
```

