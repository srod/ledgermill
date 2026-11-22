# Balancing

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21; the
`balance-fails-large-residual` and `refused/balance-fails-residual-1e28` probes 2026-11-22.

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

The size of the residual, up to about 1e24, does not change this (the one known limit follows). bean-check reports a residual of 9999 with four places
on each side (`10000.0000 AAPL {1.0000 EUR}` against `-1.0000 EUR`) and one of about 1e24
(`(1000000000000 * 1000000000000) AAPL {1.0000 EUR}`), positive or negative, with four, ten or no
places, and bean-query lists each transaction. ledgermill does the same, with the residual bean-check
prints.

ledgermill has one known limit. A residual of about 1e28 made of whole-number weights, next to an
amount with decimals, is refused: `(100000000000000 * 100000000000000) AAPL {1 EUR}` against
`-1.0000 EUR` gives `booking-error` (`details.reason: unbalanced`) at the header line, and the
transaction is not returned. bean-check reports it as not balancing
(`9999999999999999999999999999 EUR`), and bean-query lists it. The probe is under `probes/refused/`,
so the harness does not run it.

Two postings without an amount: bean-check reports "more than one auto-posting per currency" on
the second one, and bean-query leaves the transaction out. ledgermill reports `booking-error`
(`details.reason: ambiguous-auto-posting`) at the header line and does not return it.

The total-price case (`3 DOT @@ 10 EUR`, 1e-27 over) is in prices.md.

## Probe

`probes/balance-tolerance.beancount`, `probes/balance-fails-integers.beancount`, `probes/balance-fails-price.beancount`, `probes/balance-fails-tolerance.beancount`, `probes/balance-fails-two-elided.beancount`, `probes/balance-fails-large-residual.beancount`, `probes/refused/balance-fails-residual-1e28.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/balance-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-integers.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-price.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/balance-fails-two-elided.beancount
```

The two newer probes ran through `spec/beancount/run-ref.sh`, with the posting query of `show.sh`:

```sh
spec/beancount/run-ref.sh bean-check FILE
spec/beancount/run-ref.sh bean-query -f csv FILE "SELECT id, account, posting_flag, number, currency, cost_number, cost_currency, cost_date, cost_label, number(price), currency(price), filename, lineno FROM postings"
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

balance-fails-large-residual (every error line bean-check prints, without the entry it shows under each):

```text
bean-check: balance-fails-large-residual.beancount:8: Transaction does not balance: (9999.00000000 EUR)
bean-check: balance-fails-large-residual.beancount:12: Transaction does not balance: (999999999999999999999999.0000 EUR)
bean-check: balance-fails-large-residual.beancount:16: Transaction does not balance: (-999999999999999999999999.0000 EUR)
bean-check: balance-fails-large-residual.beancount:20: Transaction does not balance: (999999999999999999999999.0000 EUR)
bean-check: balance-fails-large-residual.beancount:24: Transaction does not balance: (999999999999999999999999 EUR)
e4ee64,Assets:A,,10000.0000,AAPL,1.0000,EUR,2026-01-02,,,,9
e4ee64,Assets:Cash,,-1.0000,EUR,,,,,,,10
c37cab,Assets:B,,1000000000000000000000000,AAPL,1.0000,EUR,2026-01-03,,,,13
c37cab,Assets:Cash,,-1.0000,EUR,,,,,,,14
a28032,Assets:C,,-1000000000000000000000000,AAPL,1.0000,EUR,2026-01-04,,,,17
a28032,Assets:Cash,,1.0000,EUR,,,,,,,18
86471d,Assets:D,,1000000000000000000000000,AAPL,1.0000000000,EUR,2026-01-05,,,,21
86471d,Assets:Cash,,-1.0000000000,EUR,,,,,,,22
0be834,Assets:E,,1000000000000000000000000,AAPL,1,EUR,2026-01-06,,,,25
0be834,Assets:Cash,,-1,EUR,,,,,,,26
```

refused/balance-fails-residual-1e28:

```text
bean-check: balance-fails-residual-1e28.beancount:4: Transaction does not balance: (9999999999999999999999999999 EUR)
432205,Assets:A,,1.000000000000000000000000000E+28,AAPL,1,EUR,2026-01-02,,,,5
432205,Assets:Cash,,-1.0000,EUR,,,,,,,6
```

