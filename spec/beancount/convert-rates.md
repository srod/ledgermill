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
- **A pair priced several times on one day is inverted from the day's last price only,** the one written last.
  When that one is zero, the day gives no inverse entry at all: an earlier price of the same day does not stand
  in, and the inverse rate is the one of an earlier day, or none (probes/convert-rate-zero-same-day.beancount: `ZA`,
  `4` then `0` on 01-02, has no rate; `ZB`, `0` then `4`, is `0.25`; `ZF`, `5` on 01-02 then `4` and `0` on
  01-04, stays `0.2`). Written out of date order, the last of the day is still the one written last (`ZJ`, `ZK`).
- **A pair priced both ways builds its rates in two steps,** which shows when an inverse is not exact (3, 7, 11):
  each pair starts with its own prices, then the pairs are visited one after another and each appends the inverses
  of what it holds *at that moment* to the list of the opposite pair. A pair visited first is therefore given
  the inverses of its opposite's inverses, that is its own prices again as `1 / (1 / price)` (`7` comes back
  as `6.999999999999999999999999998`), and for one date the entry appended last wins, so that doubled price beats
  the plain one. A pair that was priced one way only is never doubled. The order of the visits that fits every
  journal tried (500 random ones, three or four commodities, up to nine prices, two or three pairs priced
  both ways at rates like 3, 7, 11, 0.3, in any order and on any of six days; `for s in 1 2 3 4 5; do bun
  harness/fuzz.ts --both-ways 100 $s; done`, 2026-10-27: every `value()` and `holdings()` question equal to
  bean-query, 0 FAIL, 0 SKIP; a zero price is never generated): the pair with the shortest list of its own prices first, and among equals the pair that appears last in
  the prices table first. With one price each way that doubles the pair that appears second in the table; the
  earlier probes of this note use inverses that are exact (`4` and `0.25`), which hides it.
- **A zero price in a pair that is priced both ways is not settled.** With `CCC 4 EUR` on 01-01 and `EUR 0 CCC`
  on 01-02, the reference still values euros in `CCC` at 0.25 on 01-03 (probes/refused/price-both-ways-zero.beancount,
  below), where the rules above give zero; random journals with such a pair also disagree in other ways. ledgermill
  refuses `value()` and `holdings()` for a ledger that has one: `unsupported-syntax`, construct `price-both-ways`, on the
  line of the zero price. A zero price in a pair priced one way only is the rate of zero described above.
  The refusal covers the whole ledger, every question and every pair, not only the pair with the zero. That stays:
  the random journals that disagree also disagree on other pairs, and no probe shows which ones are safe. The
  fuzzers count these refusals by name (`harness/fuzz.ts --beancount`, `value: … skip (price-both-ways N)`);
  `--save DIR --save-skipped` keeps the journals to rerun one with `harness/real.ts`.
- **No chain.** Only a direct or an inverse entry is a rate; going from `CAD` to `EUR` through `USD` is not
  done, although both prices exist. (A cost's currency is the one bridge: convert-positions.md.)
- An amount that is already in `V` stays as it is; so does one with no rate. The product is rounded to 28
  significant digits, like every result (numbers-and-interpolation.md).
- The product of a zero and a negative number is a negative zero, printed `-0` (`-2.50` against a zero
  price gives `-0.00`). The sums of holdings.md never show that sign, because they drop zeros.

## Probe

`probes/convert-rate-dates.beancount`, `probes/convert-rate-inverse.beancount`, `probes/convert-rate-both-ways.beancount`, `probes/convert-rate-zero-same-day.beancount`, `probes/refused/price-both-ways-zero.beancount`

Command:

```sh
spec/beancount/show-value.sh spec/beancount/probes/convert-rate-dates.beancount EUR 2026 2 3 '^Assets:'
# ... and the same for the other dates below; then, for the inverse journal
spec/beancount/show-value.sh spec/beancount/probes/convert-rate-inverse.beancount EUR 2026 2 2 '^Assets:'
spec/beancount/show-holdings.sh spec/beancount/probes/convert-rate-dates.beancount EUR
spec/beancount/show-rates.sh spec/beancount/probes/convert-rate-both-ways.beancount A1 B1
```

`show-value.sh` runs Plainledger's value query, with the posting columns cut to account, units, the
amount and commodity of `convert()`, and the line; the last argument keeps the accounts that match.
`show-holdings.sh` runs its units and holdings queries (holdings.md). `show-rates.sh` runs the value query for one posting
(one unit of a commodity) on six dates at once, and prints the six numbers in a row (`-` where the unit stays). `bean-check` has no output (exit 0)
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

convert-rate-both-ways (one unit of each; the rate from FROM to TO on 2026-01-01 to 2026-01-06, one column a day):

```text
== convert-rate-both-ways.beancount (A1 priced 3 B1 on 01-01, B1 priced 7 A1 on 01-02; pair 2 has the days swapped; pair 3 one day; pair 4 two prices each way; pair 5 one price one way and two back)
A1>B1:  3  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429
B1>A1:  0.3333333333333333333333333333  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998
A2>B2:  0.1428571428571428571428571429  3.000000000000000000000000000  3.000000000000000000000000000  3.000000000000000000000000000  3.000000000000000000000000000  3.000000000000000000000000000
B2>A2:  7  0.3333333333333333333333333333  0.3333333333333333333333333333  0.3333333333333333333333333333  0.3333333333333333333333333333  0.3333333333333333333333333333
A3>B3:  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429  0.1428571428571428571428571429
B3>A3:  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998  6.999999999999999999999999998
A4>B4:  3  0.1428571428571428571428571429  11  0.07692307692307692307692307692  0.07692307692307692307692307692  0.07692307692307692307692307692
B4>A4:  0.3333333333333333333333333333  6.999999999999999999999999998  0.09090909090909090909090909091  13.00000000000000000000000000  13.00000000000000000000000000  13.00000000000000000000000000
B5>C5:  1000.500000000000000000000000  1000.500000000000000000000000  1000.500000000000000000000000  3.333333333333333333333333333  2  2
C5>B5:  0.0009995002498750624687656171914  0.0009995002498750624687656171914  0.0009995002498750624687656171914  0.3  0.5  0.5
```

price-both-ways-zero (ten euros valued in `CCC`; `CCC 4 EUR` on 01-01, `EUR 0 CCC` on 01-02):

```text
on 2026-01-01: Assets:A,10,EUR,2.50,CCC,10
on 2026-01-02: Assets:A,10,EUR,2.50,CCC,10
on 2026-01-03: Assets:A,10,EUR,2.50,CCC,10
```
