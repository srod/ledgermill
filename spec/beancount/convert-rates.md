# Prices used by `convert()`

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-23.

Status: to build (`Ledger::value()` and `Ledger::holdings()` for Beancount). The conversion of a whole
position (units, cost currency) is in convert-positions.md; this note is the rate of one pair.

## Rule

The queries Plainledger runs are `convert(position, "V", date(Y, M, D))` for a value on a date, and
`convert(position, "V")`, with no date, in the holdings query. The rate from commodity `F` to `V`:

- **Only `price` directives make rates.** `@`, `@@` and a cost make none (convert-positions.md).
- **The latest entry on or before the date.** The entries of the pair are the prices of `F` in `V` and
  the inverse of the prices of `V` in `F`, taken together, in the order of the prices table
  (price-table.md: by date, the same date in the order written). A price dated on the day asked counts.
  The latest wins, whichever way it points: a newer inverse price replaces an older direct one. On one
  day the one written last wins, direct or inverse. A price written lower in the file but dated earlier
  does not hide a later date. With no entry up to the date, there is no rate and the amount stays as it is.
- **With no date, the latest entry there is,** even one dated far in the future: today's date is never
  looked at, and a `WHERE date <= D` on the query does not move it.
- **An inverse rate is `1 / price`,** divided like any Decimal (28 digits; numbers-and-interpolation.md:
  `1 / 4` is `0.25`, `1 / 0.1` is `1E+1`), and the amount is multiplied by that rate. It is not the amount
  divided by the price: ten units against `EUR 4 QTR` give `2.50`, with the places of the product.
- **A price of zero is a rate of zero** (the amount converts to a zero in `V`), and **adds no inverse entry**:
  the inverse of zero does not exist. Going the other way, an older inverse entry of the pair stays the latest
  one there is (below: `EUR` in `ZER` is `5.0` after the price of zero as before it; `NIL`, priced at zero and
  nothing else, gives `EUR` no rate to `NIL`). A negative price is a negative rate.
- **No chain.** Only a direct or an inverse entry is a rate; going from `CAD` to `EUR` through `USD` is not
  done, although both prices exist. (A cost's currency is the one bridge: convert-positions.md.)
- An amount that is already in `V` stays as it is; so does one with no rate. The product is rounded to 28
  significant digits, like every result (numbers-and-interpolation.md).
- The product of a zero and a negative number is a negative zero, printed `-0` (`-2.50` against a zero
  price gives `-0.00`). The sums of holdings.md never show that sign, because they drop zeros.

## Probe

`probes/convert-rate-dates.beancount`, `probes/convert-rate-inverse.beancount`

Command:

```sh
spec/beancount/show-value.sh spec/beancount/probes/convert-rate-dates.beancount EUR 2026 2 3 '^Assets:'
# ... and the same for the other dates below; then, for the inverse journal
spec/beancount/show-value.sh spec/beancount/probes/convert-rate-inverse.beancount EUR 2026 2 2 '^Assets:'
spec/beancount/show-holdings.sh spec/beancount/probes/convert-rate-dates.beancount EUR
```

`show-value.sh` runs Plainledger's value query, with the posting columns cut to account, units, the
amount and commodity of `convert()`, and the line; the last argument keeps the accounts that match.
`show-holdings.sh` runs its units and holdings queries (holdings.md). `bean-check` has no output (exit 0)
for both journals.

## Output

convert-rate-dates (ten units of each commodity; `AAA` has a history dated 2026-01-10, 02-10, 03-10 and
2099-01-01; `BBB` two prices on 2026-02-01; `CCC` a price then its inverse on 2026-02-01, `DDD` the inverse
then the price; `EEE` direct on 02-03, inverse of 0.1 on 02-04, direct on 02-05, and direct on 02-02 written
last):

```text
== convert-rate-dates.beancount V=EUR on 2026-1-9
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,10,AAA,33
Assets:SameDay,10,BBB,10,BBB,34
Assets:DirectFirst,10,CCC,10,CCC,35
Assets:InverseFirst,10,DDD,10,DDD,36
Assets:Mixed,10,EEE,10,EEE,37
== convert-rate-dates.beancount V=EUR on 2026-2-1
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,10,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,10,EEE,37
== convert-rate-dates.beancount V=EUR on 2026-2-2
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,10,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,70,EUR,37
== convert-rate-dates.beancount V=EUR on 2026-2-3
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,10,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,30,EUR,37
== convert-rate-dates.beancount V=EUR on 2026-2-4
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,10,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,1.0E+2,EUR,37
== convert-rate-dates.beancount V=EUR on 2026-2-5
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,10,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,50,EUR,37
== convert-rate-dates.beancount V=EUR on 2026-3-9
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,20,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,50,EUR,37
== convert-rate-dates.beancount V=EUR on 2099-1-1
-- value: account,number,currency,amount,commodity,lineno
Assets:Dated,10,AAA,90,EUR,33
Assets:SameDay,10,BBB,30,EUR,34
Assets:DirectFirst,10,CCC,2.50,EUR,35
Assets:InverseFirst,10,DDD,20,EUR,36
Assets:Mixed,10,EEE,50,EUR,37
```

Without a date (the holdings query), every commodity takes its latest entry, the one dated 2099 for `AAA`;
`EEE` takes its entry of 02-05:

```text
== convert-rate-dates.beancount V=EUR 
-- bean-check
-- units: account,currency,units
Assets:Dated,AAA,(10 AAA)
Equity:Other,AAA,(-10 AAA)
Assets:SameDay,BBB,(10 BBB)
Equity:Other,BBB,(-10 BBB)
Assets:DirectFirst,CCC,(10 CCC)
Equity:Other,CCC,(-10 CCC)
Assets:InverseFirst,DDD,(10 DDD)
Equity:Other,DDD,(-10 DDD)
Assets:Mixed,EEE,(10 EEE)
Equity:Other,EEE,(-10 EEE)
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
Assets:Dated,AAA,(10 AAA),(10 AAA),(90 EUR),(90 EUR)
Equity:Other,AAA,(-10 AAA),(-10 AAA),(-90 EUR),(-90 EUR)
Assets:SameDay,BBB,(10 BBB),(10 BBB),(30 EUR),(30 EUR)
Equity:Other,BBB,(-10 BBB),(-10 BBB),(-30 EUR),(-30 EUR)
Assets:DirectFirst,CCC,(10 CCC),(10 CCC),(2.50 EUR),(2.50 EUR)
Equity:Other,CCC,(-10 CCC),(-10 CCC),(-2.50 EUR),(-2.50 EUR)
Assets:InverseFirst,DDD,(10 DDD),(10 DDD),(20 EUR),(20 EUR)
Equity:Other,DDD,(-10 DDD),(-10 DDD),(-20 EUR),(-20 EUR)
Assets:Mixed,EEE,(10 EEE),(10 EEE),(50 EUR),(50 EUR)
Equity:Other,EEE,(-10 EEE),(-10 EEE),(-50 EUR),(-50 EUR)
```

convert-rate-inverse (`QTR`: `EUR 4 QTR`; `TNT`: `EUR 0.1 TNT`; `ZER`: price 2 on 02-01, zero on 02-02;
`NIL`: zero only; `NEG`: -0.50; `CAD` 2 `USD`, `USD` 0.40 `EUR`):

```text
== convert-rate-inverse.beancount V=EUR on 2026-2-1
-- value: account,number,currency,amount,commodity,lineno
Assets:Quarter,10,QTR,2.50,EUR,27
Assets:Tenth,10,TNT,1.0E+2,EUR,28
Assets:Zero,10,ZER,20,EUR,29
Assets:Short,-2.50,ZER,-5.00,EUR,31
Assets:Negative,-4,NEG,2.00,EUR,30
Assets:Chain,3,CAD,3,CAD,32
Assets:Cash,10,EUR,10,EUR,33
== convert-rate-inverse.beancount V=EUR on 2026-2-2
-- value: account,number,currency,amount,commodity,lineno
Assets:Quarter,10,QTR,2.50,EUR,27
Assets:Tenth,10,TNT,1.0E+2,EUR,28
Assets:Zero,10,ZER,0,EUR,29
Assets:Short,-2.50,ZER,-0.00,EUR,31
Assets:Negative,-4,NEG,2.00,EUR,30
Assets:Chain,3,CAD,3,CAD,32
Assets:Cash,10,EUR,10,EUR,33
== convert-rate-inverse.beancount V=USD on 2026-2-2
-- value: account,number,currency,amount,commodity,lineno
Assets:Quarter,10,QTR,10,QTR,27
Assets:Tenth,10,TNT,10,TNT,28
Assets:Zero,10,ZER,10,ZER,29
Assets:Short,-2.50,ZER,-2.50,ZER,31
Assets:Negative,-4,NEG,-4,NEG,30
Assets:Chain,3,CAD,6,USD,32
Assets:Cash,10,EUR,25.0,USD,33
== convert-rate-inverse.beancount V=QTR on 2026-2-2
-- value: account,number,currency,amount,commodity,lineno
Assets:Quarter,10,QTR,10,QTR,27
Assets:Tenth,10,TNT,10,TNT,28
Assets:Zero,10,ZER,10,ZER,29
Assets:Short,-2.50,ZER,-2.50,ZER,31
Assets:Negative,-4,NEG,-4,NEG,30
Assets:Chain,3,CAD,3,CAD,32
Assets:Cash,10,EUR,40,QTR,33
== convert-rate-inverse.beancount V=ZER on 2026-2-2
-- value: account,number,currency,amount,commodity,lineno
Assets:Quarter,10,QTR,10,QTR,27
Assets:Tenth,10,TNT,10,TNT,28
Assets:Zero,10,ZER,10,ZER,29
Assets:Short,-2.50,ZER,-2.50,ZER,31
Assets:Negative,-4,NEG,-4,NEG,30
Assets:Chain,3,CAD,3,CAD,32
Assets:Cash,10,EUR,5.0,ZER,33
== convert-rate-inverse.beancount V=NIL on 2026-2-2
-- value: account,number,currency,amount,commodity,lineno
Assets:Quarter,10,QTR,10,QTR,27
Assets:Tenth,10,TNT,10,TNT,28
Assets:Zero,10,ZER,10,ZER,29
Assets:Short,-2.50,ZER,-2.50,ZER,31
Assets:Negative,-4,NEG,-4,NEG,30
Assets:Chain,3,CAD,3,CAD,32
Assets:Cash,10,EUR,10,EUR,33
```
