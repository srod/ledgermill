# Prices

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

A price is `@ number currency` (per unit) or `@@ number currency` (total). The reference reports
a price per unit, with these rules (the number rules are in numbers-and-interpolation.md):

- `@` keeps the places as written: `@ 100.00 EUR` is `100.00`.
- `@@` is divided by the absolute value of the units, so a sale (negative units) has a positive
  price. An exact quotient keeps as few places as it needs (`4 BTC @@ 10 EUR` is `2.5`); an inexact
  one has 28 significant digits (`3 BTC @@ 10.00 EUR` is `3.333333333333333333333333333`).
- The weight of a total price is the units times that rounded per-unit price, not the total. With
  an elided counter-posting, `3 BTC @@ 10 EUR` gives `-9.999999999999999999999999999` and
  `7 BTC @@ 1 EUR` gives `-1.000000000000000000000000000`.
- For the same reason `3 DOT @@ 10 EUR` against `-10 EUR` does not balance: the residual is
  1e-27 and a currency with no written decimal place is checked exactly. bean-check says so, and
  ledgermill reports `unbalanced-transaction` (the balancing rule is in balancing.md).
- A posting with a cost and a price keeps both; the cost decides the weight (`2 BTC {10 EUR} @
  12.50 EUR` weighs 20 EUR). A price without a cost gives the weight (units times price) and
  the currency group.

## Probe

`probes/prices.beancount`, `probes/balance-total-price-rounding.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/prices.beancount
spec/beancount/show.sh spec/beancount/probes/balance-total-price-rounding.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

prices:

```text
bean-check: (no output, exit 0)
9f3cb0,Assets:Btc,,1,BTC,,,,,100.00,EUR,5
9f3cb0,Assets:Bank,,-100.00,EUR,,,,,,,6
53974c,Assets:Btc,,2,BTC,,,,,100.50,EUR,8
53974c,Assets:Bank,,-201.00,EUR,,,,,,,9
a1a69f,Assets:Btc,,4,BTC,,,,,2.5,EUR,11
a1a69f,Assets:Bank,,-10,EUR,,,,,,,12
cf8b7a,Assets:Btc,,3,BTC,,,,,3.333333333333333333333333333,EUR,14
cf8b7a,Assets:Bank,,-10.00,EUR,,,,,,,15
f9dffc,Assets:Btc,,-2,BTC,,,,,50,EUR,17
f9dffc,Assets:Bank,,100,EUR,,,,,,,18
2a628e,Assets:Btc,,3,BTC,,,,,3.333333333333333333333333333,EUR,20
2a628e,Assets:Bank,,-9.999999999999999999999999999,EUR,,,,,,,21
8366e6,Assets:Btc,,7,BTC,,,,,0.1428571428571428571428571429,EUR,23
8366e6,Assets:Bank,,-1.000000000000000000000000000,EUR,,,,,,,24
94d864,Assets:Btc,,2,BTC,10,EUR,2026-01-09,,12.50,EUR,26
94d864,Assets:Bank,,-20,EUR,,,,,,,27
e99c13,Assets:Btc,,2,BTC,,,,,0.5,EUR,29
e99c13,Assets:Bank,,-1.0,EUR,,,,,,,30
```

balance-total-price-rounding:

```text
bean-check: balance-total-price-rounding.beancount:4: Transaction does not balance: (-0.000000000000000000000000001 EUR)
518b30,Assets:A,,3,DOT,,,,,3.333333333333333333333333333,EUR,5
518b30,Assets:B,,-10,EUR,,,,,,,6
```

