# Inferred cost and price

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Status: **built**, except the shapes listed at the end of the Rule section (they stay refused with
`unsupported-syntax`: `inferred-price` and `inferred-cost` for the ones named there, and
`cost-partial`). This note is what the reference does.
Written by the agent session listed in EXPOSURE.md for 2026-10-22 (it saw two lines of the
reference's booking code in a tool crash); a human should check that it only records tool output.

## Rule

### Inferred cost

A posting that **adds** a lot (positive units, or negative units into an account that holds none
of that commodity, which adds a negative lot, see lots-and-booking.md) may leave the cost number
out. Any spec without a number does: `{}`, `{EUR}`, `{"lot1"}`, `{2025-12-30}`, `{ , "lot1"}`
(a leading comma is allowed), `{{}}`. The cost per unit is then the figure that balances the
rest of the transaction in the cost currency:

    cost = -(sum of the weights of the other postings in that currency) / units

`4 BTC {}` against `-50.00 EUR` is `12.50 EUR`; with `-30.00 EUR` and `-20.00 EUR` it is the
same; `-4 BTC {}` against `+50.00 EUR` is `12.50` too (the sign of the units is in the
division). The date is the transaction's unless one is written, the label as written, and the
price, if any, is kept as written (`4 BTC {} @ 13 EUR` is cost `12.50`, price `13`). Other
postings that carry a price count by their weight (`-10.00 USD @ 5 EUR` pays `50 EUR`).

A posting that **reduces** (the account holds the commodity with units of the opposite sign) is
not an inference: `{}` there selects lots as usual.

The currency of the cost: the one written; else the price's currency (`{} @ 13 USD` is a cost in
USD even when the payment is in EUR, and then the EUR payment does not balance); else the one
currency of the other postings.

Digits: the division is the one of `{{ }}` totals (lots-and-booking.md, prices.md). The result
keeps the places the quotient needs from its operands (`50.00 / 4` is `12.50`, `50 / 4` is
`12.5`), and has 28 significant digits when inexact: `3 BTC {}` against `-10.00 EUR` is
`3.333333333333333333333333333`, `4.5 BTC` against `-50.00 EUR` is
`11.11111111111111111111111111`. The weight of the posting is units times that cost, so a
non-terminating cost leaves a residual of 1e-27: it passes when the other amounts are written
with decimal places (`-10.00 EUR`), and is `Transaction does not balance` when they have none
(`-10 EUR`), as in prices.md.

Nothing to balance: when the other postings in the cost currency sum to zero, or there are none
(`{EUR}` with only USD paid), the cost is `0` (written `0`, not `0.00`) and the cost itself is no
error; the transaction may still not balance in the other currency.

Errors, all with the transaction left out of bean-query (bean-check prints the first line):

- a wrong-sign payment gives a negative cost: `Cost is negative` (bean-query still lists the
  row, with cost `-12.50`);
- more than one thing to work out: two `{}` postings, `{}` and an elided posting, `{}` with
  payments in two currencies and no currency written: `Failed to categorize posting`;
  elided units (`BTC {}`): `Too many missing numbers for currency group`;
- `0 BTC {}` against a payment: `Transaction does not balance` (the units are zero, so nothing
  is added).

Equal lots: two buys with the same account, date, label and cost value are one lot to a later
sale, and the sale's rows show the digits of the first of them (`12.50`, not the `12.5` of a
later buy written `{12.5 EUR}`); an inferred cost takes part like any other.

A **partial** cost has one of its two parts left out, `{# 5 EUR}` or `{2 # EUR}`: the missing
part is worked out so the cost per unit comes out as above (`3 BTC {# 1 EUR}` against
`-10.00 EUR` is `3.333333333333333333333333333`). It is not `{{5 EUR}}`, the total written in
full, which is a cost of 5 EUR in all. ledgermill's text rewrite turns `{{ total }}` into
`{# total}` for the parser (header-and-flags.md), so after it the two cannot be told apart;
build the inference and the rewrite together, or refuse the written `#` with a missing part.

### Inferred price

A price (`@` or `@@`) without a number, without a currency, or without either:
`@ EUR`, `@@ EUR`, `@ 12.5`, `@@ 50`, a bare `@`. The price per unit is the same figure, in the
price's currency (`@@ EUR` and `@ EUR` give the same per-unit price):

    price = -(sum of the weights of the other postings in that currency) / units

`4 BTC @ EUR` against `-50.00 EUR` is `12.50 EUR`; `-4 BTC @ EUR` against `+50.00 EUR` is
`12.50` (a positive price); `10.00 USD @ EUR` against `-9.00 EUR` is `0.9 EUR`; `3 BTC @@ EUR`
against `-10.00 EUR` has the 28-digit price of the cost above. The currency is the one written,
else the one currency of the other postings.

Errors: a wrong-sign payment (`4 BTC @ EUR` against `+50.00 EUR`) lists a price of `12.50` but
does not balance (`100.00 EUR`); a price in a currency nothing else is in (`4 BTC @ USD` against
`-50.00 EUR`) is `0` and the EUR payment does not balance; a currency left out with payments in
two currencies is `Failed to categorize posting`; a payment in a second currency (`@ EUR`, then
`-10.00 USD`) does not balance. An elided counter-posting, elided units, or two prices to work
out are `Too many missing numbers for currency group`.

With a cost, a price left to work out is not mapped. `4 BTC {12 EUR} @ USD` (the number left
out, a cost written) and `0 BTC @ EUR` (zero units, a price to work out) make the reference
tools crash with a Python error, so no probe of them is kept and they must not be re-run; and
`-3 BTC {} @ EUR` (cost and price both open) is `Too many missing numbers`. ledgermill keeps
refusing a price with a missing part next to a cost (`inferred-price`), and should refuse zero
units with a price to work out.

### What ledgermill should report

Cases the reference accepts give the postings above. Cases it rejects give `booking-error`
(`details.reason`: `negative-cost`, `negative-price`, `too-many-missing-numbers`,
`cannot-infer-currency`, ...) or `unbalanced-transaction` and no transaction, which is all the
harness compares for two failures (the reason need not match).

`{ , "lot1"}` and `{{}}` are rewritten in `text.rs` (`{ , "lot1"}` loses its comma, `{{}}` becomes
`{}`) so the parser reads them.

### Still refused

- `cost-partial`: `{# 5 EUR}` and `{2 # EUR}` (`probes/refused/cost-partial-*`), found in the text as
  written, because the rewrite of `{{ total }}` makes them look like a written total.
- `inferred-price`: a price with a number or currency left out on a posting that also has a cost
  (`probes/refused/inferred-price-with-cost.beancount`; the reference crashes on some of these), and
  zero units with a price to work out (the reference crashes).
- `inferred-cost`: an addition with a cost to work out in a transaction that also holds a
  reduction with an open cost (`-1 BTC {}`), whose weight is known only after lot matching; a
  cost to work out with zero units and nothing to pay (not probed, the reference may crash); a
  posting whose cost has a number but no currency, next to a cost to work out.

## Probe

`probes/lots-inferred-cost.beancount`, `probes/lots-inferred-cost-currency.beancount`, `probes/lots-inferred-cost-label-comma.beancount`, `probes/lots-inferred-cost-date-and-label.beancount`, `probes/lots-inferred-cost-double-braces.beancount`, `probes/lots-inferred-cost-two-payments.beancount`, `probes/lots-inferred-cost-division.beancount`, `probes/lots-inferred-cost-integers.beancount`, `probes/lots-inferred-cost-zero-residual.beancount`, `probes/lots-inferred-cost-currency-without-payment.beancount`, `probes/lots-inferred-cost-with-price.beancount`, `probes/lots-inferred-cost-currency-with-price.beancount`, `probes/lots-inferred-cost-price-currency.beancount`, `probes/lots-inferred-cost-price-currency-paid.beancount`, `probes/lots-inferred-cost-paid-with-price.beancount`, `probes/lots-inferred-cost-negative-units.beancount`, `probes/lots-inferred-cost-negative-result.beancount`, `probes/lots-inferred-cost-then-sale.beancount`, `probes/lots-inferred-cost-equal-lots.beancount`, `probes/lots-inferred-cost-two-currencies.beancount`, `probes/lots-inferred-cost-elided-posting.beancount`, `probes/lots-inferred-cost-two-lots.beancount`, `probes/lots-inferred-cost-unbalanced.beancount`, `probes/lots-inferred-cost-zero-units.beancount`, `probes/refused/cost-partial-per-unit.beancount`, `probes/refused/cost-partial-total.beancount`, `probes/prices-inferred-number.beancount`, `probes/prices-inferred-currency.beancount`, `probes/prices-inferred-total.beancount`, `probes/prices-inferred-bare.beancount`, `probes/prices-inferred-total-number-only.beancount`, `probes/prices-inferred-division.beancount`, `probes/prices-inferred-negative-units.beancount`, `probes/prices-inferred-wrong-sign.beancount`, `probes/prices-inferred-other-currency.beancount`, `probes/prices-inferred-two-currencies.beancount`, `probes/prices-inferred-currency-two-groups.beancount`, `probes/prices-inferred-conversion.beancount`, `probes/refused/inferred-price-with-cost.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-currency.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-label-comma.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-date-and-label.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-double-braces.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-two-payments.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-division.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-integers.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-zero-residual.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-currency-without-payment.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-with-price.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-currency-with-price.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-price-currency.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-price-currency-paid.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-paid-with-price.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-negative-units.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-negative-result.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-then-sale.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-equal-lots.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-two-currencies.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-elided-posting.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-two-lots.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-unbalanced.beancount
spec/beancount/show.sh spec/beancount/probes/lots-inferred-cost-zero-units.beancount
spec/beancount/show.sh spec/beancount/probes/refused/cost-partial-per-unit.beancount
spec/beancount/show.sh spec/beancount/probes/refused/cost-partial-total.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-number.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-currency.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-total.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-bare.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-total-number-only.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-division.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-negative-units.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-wrong-sign.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-other-currency.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-two-currencies.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-currency-two-groups.beancount
spec/beancount/show.sh spec/beancount/probes/prices-inferred-conversion.beancount
spec/beancount/show.sh spec/beancount/probes/refused/inferred-price-with-cost.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any. Below, only the posting rows are shown.

## Output

Inferred cost:

lots-inferred-cost:

```text
bean-check: (no output, exit 0)
a891b6,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,5
a891b6,Assets:Bank,,-50.00,EUR,,,,,,,6
```

lots-inferred-cost-currency:

```text
bean-check: (no output, exit 0)
564e8b,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,6
564e8b,Assets:Bank,,-50.00,EUR,,,,,,,7
```

lots-inferred-cost-label-comma:

```text
bean-check: (no output, exit 0)
b157aa,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,lot1,,,6
b157aa,Assets:Bank,,-50.00,EUR,,,,,,,7
```

lots-inferred-cost-date-and-label:

```text
bean-check: (no output, exit 0)
b18032,Assets:Stock,,4,BTC,12.50,EUR,2025-12-30,x,,,8
b18032,Assets:Bank,,-50.00,EUR,,,,,,,9
```

lots-inferred-cost-double-braces:

```text
bean-check: (no output, exit 0)
68e07b,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,6
68e07b,Assets:Bank,,-50.00,EUR,,,,,,,7
```

lots-inferred-cost-two-payments:

```text
bean-check: (no output, exit 0)
d2c534,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,6
d2c534,Assets:Bank,,-30.00,EUR,,,,,,,7
d2c534,Assets:Other,,-20.00,EUR,,,,,,,8
```

lots-inferred-cost-division:

```text
bean-check: (no output, exit 0)
8fa3d3,Assets:Stock,,3,BTC,3.333333333333333333333333333,EUR,2026-01-02,,,,6
8fa3d3,Assets:Bank,,-10.00,EUR,,,,,,,7
```

lots-inferred-cost-integers:

```text
bean-check: lots-inferred-cost-integers.beancount:6: Transaction does not balance: (-0.000000000000000000000000001 EUR)
52fbc4,Assets:Stock,,3,BTC,3.333333333333333333333333333,EUR,2026-01-02,,,,7
52fbc4,Assets:Bank,,-10,EUR,,,,,,,8
```

lots-inferred-cost-zero-residual:

```text
bean-check: (no output, exit 0)
41fbe0,Assets:Stock,,4,BTC,0,EUR,2026-01-02,,,,6
41fbe0,Assets:Bank,,-50.00,EUR,,,,,,,7
41fbe0,Assets:Other,,50.00,EUR,,,,,,,8
```

lots-inferred-cost-currency-without-payment:

```text
bean-check: lots-inferred-cost-currency-without-payment.beancount:5: Transaction does not balance: (-50.00 USD)
cd6a68,Assets:Stock,,4,BTC,0,EUR,2026-01-02,,,,6
cd6a68,Assets:Bank,,-50.00,USD,,,,,,,7
```

lots-inferred-cost-with-price:

```text
bean-check: (no output, exit 0)
9a9500,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,13,EUR,6
9a9500,Assets:Bank,,-50.00,EUR,,,,,,,7
```

lots-inferred-cost-currency-with-price:

```text
bean-check: (no output, exit 0)
1239df,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,20,EUR,8
1239df,Assets:Bank,,-50.00,EUR,,,,,,,9
```

lots-inferred-cost-price-currency:

```text
bean-check: lots-inferred-cost-price-currency.beancount:6: Transaction does not balance: (-50.00 EUR)
e78d4d,Assets:Stock,,4,BTC,0,USD,2026-01-02,,13,USD,7
e78d4d,Assets:Bank,,-50.00,EUR,,,,,,,8
```

lots-inferred-cost-price-currency-paid:

```text
bean-check: (no output, exit 0)
f2ce9f,Assets:Stock,,4,BTC,12.50,USD,2026-01-02,,13,USD,7
f2ce9f,Assets:Bank,,-50.00,USD,,,,,,,8
```

lots-inferred-cost-paid-with-price:

```text
bean-check: (no output, exit 0)
c98892,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,8
c98892,Assets:Bank,,-10.00,USD,,,,,5,EUR,9
```

lots-inferred-cost-negative-units:

```text
bean-check: (no output, exit 0)
66b1aa,Assets:Stock,,-4,BTC,12.50,EUR,2026-01-02,,,,6
66b1aa,Assets:Bank,,50.00,EUR,,,,,,,7
```

lots-inferred-cost-negative-result:

```text
bean-check: lots-inferred-cost-negative-result.beancount:7: Cost is negative: "Cost(number=Decimal('-12.50'), currency='EUR', date=datetime.date(2026, 1, 2), label=None)"
def193,Assets:Stock,,4,BTC,-12.50,EUR,2026-01-02,,,,7
def193,Assets:Bank,,50.00,EUR,,,,,,,8
```

lots-inferred-cost-then-sale:

```text
bean-check: (no output, exit 0)
fb281f,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,7
fb281f,Assets:Bank,,-50.00,EUR,,,,,,,8
6303a1,Assets:Stock,,3,BTC,3.333333333333333333333333333,EUR,2026-01-03,,,,11
6303a1,Assets:Bank,,-10.00,EUR,,,,,,,12
fcd757,Assets:Stock,,-4,BTC,12.50,EUR,2026-01-02,,20,EUR,15
fcd757,Assets:Stock,,-1,BTC,3.333333333333333333333333333,EUR,2026-01-03,,20,EUR,15
fcd757,Assets:Bank,,100.00,EUR,,,,,,,16
fcd757,Income:Gain,,-46.67,EUR,,,,,,,17
```

lots-inferred-cost-equal-lots:

```text
bean-check: (no output, exit 0)
7cf7aa,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,8
7cf7aa,Assets:Bank,,-50.00,EUR,,,,,,,9
aa4470,Assets:Stock,,4,BTC,12.5,EUR,2026-01-02,,,,12
aa4470,Assets:Bank,,-50.00,EUR,,,,,,,13
95bb59,Assets:Stock,,2,BTC,12.50,EUR,2026-01-02,,,,16
95bb59,Assets:Bank,,-25.00,EUR,,,,,,,17
ee89de,Assets:Stock,,-10,BTC,12.50,EUR,2026-01-02,,,,20
ee89de,Assets:Bank,,125.00,EUR,,,,,,,21
```

lots-inferred-cost-two-currencies:

```text
bean-check: lots-inferred-cost-two-currencies.beancount:6: Failed to categorize posting 1
(bean-query lists no posting)
```

lots-inferred-cost-elided-posting:

```text
bean-check: lots-inferred-cost-elided-posting.beancount:6: Failed to categorize posting 1
(bean-query lists no posting)
```

lots-inferred-cost-two-lots:

```text
bean-check: lots-inferred-cost-two-lots.beancount:8: Too many missing numbers for currency group 'EUR'
(bean-query lists no posting)
```

lots-inferred-cost-unbalanced:

```text
bean-check: lots-inferred-cost-unbalanced.beancount:6: Transaction does not balance: (-10.00 USD)
e62b38,Assets:Stock,,4,BTC,12.50,EUR,2026-01-02,,,,7
e62b38,Assets:Bank,,-50.00,EUR,,,,,,,8
e62b38,Assets:Other,,-10.00,USD,,,,,,,9
```

lots-inferred-cost-zero-units:

```text
bean-check: lots-inferred-cost-zero-units.beancount:4: Transaction does not balance: (-5.00 EUR)
b89651,Assets:Bank,,-5.00,EUR,,,,,,,6
```

cost-partial-per-unit:

```text
bean-check: (no output, exit 0)
5cd304,Assets:Stock,,3,BTC,3.333333333333333333333333333,EUR,2026-01-02,,,,7
5cd304,Assets:Bank,,-10.00,EUR,,,,,,,8
```

cost-partial-total:

```text
bean-check: (no output, exit 0)
c4ff54,Assets:Stock,,3,BTC,3.333333333333333333333333333,EUR,2026-01-02,,,,7
c4ff54,Assets:Bank,,-10.00,EUR,,,,,,,8
```

Inferred price:

prices-inferred-number:

```text
bean-check: (no output, exit 0)
b2a3e5,Assets:Stock,,4,BTC,,,,,12.50,EUR,6
b2a3e5,Assets:Bank,,-50.00,EUR,,,,,,,7
```

prices-inferred-currency:

```text
bean-check: (no output, exit 0)
df33c0,Assets:Stock,,4,BTC,,,,,12.5,EUR,6
df33c0,Assets:Bank,,-50.00,EUR,,,,,,,7
```

prices-inferred-total:

```text
bean-check: (no output, exit 0)
8f3d83,Assets:Stock,,4,BTC,,,,,12.50,EUR,6
8f3d83,Assets:Bank,,-50.00,EUR,,,,,,,7
```

prices-inferred-bare:

```text
bean-check: (no output, exit 0)
05dd70,Assets:Stock,,4,BTC,,,,,12.50,EUR,6
05dd70,Assets:Bank,,-50.00,EUR,,,,,,,7
```

prices-inferred-total-number-only:

```text
bean-check: (no output, exit 0)
f76fb3,Assets:Stock,,4,BTC,,,,,12.5,EUR,8
f76fb3,Assets:Bank,,-50.00,EUR,,,,,,,9
```

prices-inferred-division:

```text
bean-check: (no output, exit 0)
ebaa76,Assets:Stock,,3,BTC,,,,,3.333333333333333333333333333,EUR,7
ebaa76,Assets:Bank,,-10.00,EUR,,,,,,,8
```

prices-inferred-negative-units:

```text
bean-check: (no output, exit 0)
364c8e,Assets:Stock,,-4,BTC,,,,,12.50,EUR,7
364c8e,Assets:Bank,,50.00,EUR,,,,,,,8
```

prices-inferred-wrong-sign:

```text
bean-check: prices-inferred-wrong-sign.beancount:6: Transaction does not balance: (100.00 EUR)
795ca9,Assets:Stock,,4,BTC,,,,,12.50,EUR,7
795ca9,Assets:Bank,,50.00,EUR,,,,,,,8
```

prices-inferred-other-currency:

```text
bean-check: prices-inferred-other-currency.beancount:6: Transaction does not balance: (-50.00 EUR)
3e1d50,Assets:Stock,,4,BTC,,,,,0,USD,7
3e1d50,Assets:Bank,,-50.00,EUR,,,,,,,8
```

prices-inferred-two-currencies:

```text
bean-check: prices-inferred-two-currencies.beancount:6: Transaction does not balance: (-10.00 USD)
cfc395,Assets:Stock,,4,BTC,,,,,12.50,EUR,7
cfc395,Assets:Bank,,-50.00,EUR,,,,,,,8
cfc395,Assets:Other,,-10.00,USD,,,,,,,9
```

prices-inferred-currency-two-groups:

```text
bean-check: prices-inferred-currency-two-groups.beancount:8: Failed to categorize posting 1
(bean-query lists no posting)
```

prices-inferred-conversion:

```text
bean-check: (no output, exit 0)
ed4ad2,Assets:Bank,,10.00,USD,,,,,0.9,EUR,8
ed4ad2,Assets:Other,,-9.00,EUR,,,,,,,9
```

inferred-price-with-cost:

```text
bean-check: inferred-price-with-cost.beancount:12: Too many missing numbers for currency group 'EUR'
883397,Assets:Fifo,,4,BTC,12.50,EUR,2026-01-02,,,,8
883397,Assets:Bank,,-50.00,EUR,,,,,,,9
```

