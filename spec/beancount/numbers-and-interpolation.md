# Numbers and interpolation

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

Numbers. A number may have digit-group commas (`1,234,567.89`), a leading `+`, and a trailing dot
(`5.` is `5`). Places are kept as written (`1.50` stays `1.50`) and `-0.00` comes back as `0.00`.
The reference does exact decimal arithmetic on a coefficient and an exponent, not on places:

- a sum keeps the finer of the two exponents, a product adds exponents;
- every result is rounded half to even to 28 significant digits;
- a division whose result is exact keeps as few digits as it can without going past the exponent
  of the first operand minus the exponent of the second; an inexact one has 28 digits;
- a number prints plainly unless its exponent is positive or it is smaller than 0.000001; then it
  prints as `d.dddE+n`, so `0.000000000000001` is `1E-15`, `0.0000001` is `1E-7`, and `200` can
  print as `2.0E+2`. Large numbers without a positive exponent stay plain.

ledgermill does these operations itself (`pydec.rs`). A number with arithmetic, or more than 22
characters, is refused (see refused.md).

Elided amount (interpolation). A posting with an account and nothing else gets, for each group,
minus the sum of the weights of the others. A posting's weight is its units, or units times
price, or units times cost; its group is the cost currency, else the price currency, else the
units' currency. Rows are grouped by group, in the order the groups first appear among the postings
that have an amount, and keep their source order inside a group; the elided posting stands in
each group that is left over (one row per currency, in alphabetical order) and sits where it was
written inside the group. A residual of zero adds no row: a lone elided posting vanishes, and a
transaction with no postings stays, with none.

Places of an elided amount: it is rounded half to even to the fewest decimal places among the
amounts written in that currency. Integers do not count, nor do the places of a price or a cost
(`1 STK @ 1.2345 EUR` next to `2.25 EUR` gives two places); trailing zeros do count (`1.50`).
With no written amount that has places, the sum is exact: `2.5 STK @ 100.25 EUR` alone gives
`-250.625`. A price number with nine places is kept whole in the weight.

Total prices and costs (`@@`, `{{ }}`), and their exponents:

- a price per unit from a total is `total / |units|` with the division rule above, so
  `0.5 BTC @@ 100 EUR` has a price of `2.0E+2` (exponent 0 minus -1 is 1), `0.05 BTC @@ 1000 EUR`
  has `2.00E+4`, `0.2 BTC @@ 3 EUR` has `15` (no exponent fits), a zero total has `0E+1`;
- a cost per unit from a total prints without an exponent (`0.5 ETH {{100 EUR}}` is `200`).

## Probe

`probes/number-forms.beancount`, `probes/interpolation.beancount`, `probes/interpolation-groups.beancount`, `probes/interpolation-several-currencies.beancount`, `probes/prices-exponent.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/number-forms.beancount
spec/beancount/show.sh spec/beancount/probes/interpolation.beancount
spec/beancount/show.sh spec/beancount/probes/interpolation-groups.beancount
spec/beancount/show.sh spec/beancount/probes/interpolation-several-currencies.beancount
spec/beancount/show.sh spec/beancount/probes/prices-exponent.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

number-forms:

```text
bean-check: (no output, exit 0)
670b4c,Assets:A,,1234567.89,EUR,,,,,,,5
670b4c,Assets:B,,-1234567.89,EUR,,,,,,,6
54deb7,Assets:A,,5,EUR,,,,,,,8
54deb7,Assets:B,,-5,EUR,,,,,,,9
20c7c9,Assets:A,,5,EUR,,,,,,,11
20c7c9,Assets:B,,-5,EUR,,,,,,,12
0b0063,Assets:A,,1E-15,EUR,,,,,,,14
0b0063,Assets:B,,-1E-15,EUR,,,,,,,15
5ab93a,Assets:A,,0.00,EUR,,,,,,,17
5ab93a,Assets:B,,0,EUR,,,,,,,18
```

interpolation:

```text
bean-check: (no output, exit 0)
f50b85,transaction,4,2026-01-02,*,,10.00 and -3.333333 leave -6.67
c9a540,transaction,8,2026-01-03,*,,a price with nine places is kept whole
e89557,transaction,11,2026-01-04,*,,integers do not count for the places
32359c,transaction,15,2026-01-05,*,,the fewest places among the amounts
4df377,transaction,19,2026-01-06,*,,only integers
179712,transaction,23,2026-01-07,*,,digit groups
09418f,transaction,26,2026-01-08,*,,trailing zeros count as places
cbd749,transaction,30,2026-01-09,*,,half to even down
15eccd,transaction,34,2026-01-10,*,,half to even up
98404b,transaction,38,2026-01-11,*,,negative half to even
1a6231,transaction,42,2026-01-12,*,,places can shrink the sum
a4c69b,transaction,47,2026-01-13,*,,units times price add places
2a69c7,transaction,50,2026-01-14,*,,other currency places do not count
e21c18,transaction,54,2026-01-15,*,,a cost leaves the places of the units
7e1987,transaction,58,2026-01-16,*,,zero residual drops the elided posting
9ca46d,transaction,62,2026-01-17,*,,"one elided posting,nothing else"
f2ed8f,transaction,64,2026-01-18,*,,no postings
c66555,transaction,65,2026-01-19,*,,a zero posting stays
f50b85,Assets:A,,10.00,EUR,,,,,,,5
f50b85,Assets:A,,-3.333333,EUR,,,,,,,6
f50b85,Assets:B,,-6.67,EUR,,,,,,,7
c9a540,Assets:A,,3,STK,,,,,1.123456789,EUR,9
c9a540,Assets:B,,-3.370370367,EUR,,,,,,,10
e89557,Assets:A,,10,EUR,,,,,,,12
e89557,Assets:A,,-3.333,EUR,,,,,,,13
e89557,Assets:B,,-6.667,EUR,,,,,,,14
32359c,Assets:A,,1.123456,EUR,,,,,,,16
32359c,Assets:A,,1.1234567,EUR,,,,,,,17
32359c,Assets:B,,-2.246913,EUR,,,,,,,18
4df377,Assets:A,,10,EUR,,,,,,,20
4df377,Assets:A,,5,EUR,,,,,,,21
4df377,Assets:B,,-15,EUR,,,,,,,22
179712,Assets:A,,1000.5,EUR,,,,,,,24
179712,Assets:B,,-1000.5,EUR,,,,,,,25
09418f,Assets:A,,1.50,EUR,,,,,,,27
09418f,Assets:A,,1.5000,EUR,,,,,,,28
09418f,Assets:B,,-3.00,EUR,,,,,,,29
cbd749,Assets:A,,1.00,EUR,,,,,,,31
cbd749,Assets:A,,1.005,EUR,,,,,,,32
cbd749,Assets:B,,-2.00,EUR,,,,,,,33
15eccd,Assets:A,,1.00,EUR,,,,,,,35
15eccd,Assets:A,,1.015,EUR,,,,,,,36
15eccd,Assets:B,,-2.02,EUR,,,,,,,37
98404b,Assets:A,,-1.00,EUR,,,,,,,39
98404b,Assets:A,,-1.025,EUR,,,,,,,40
98404b,Assets:B,,2.02,EUR,,,,,,,41
1a6231,Assets:A,,1.5,EUR,,,,,,,43
1a6231,Assets:A,,1.5,EUR,,,,,,,44
1a6231,Assets:A,,1.25,EUR,,,,,,,45
1a6231,Assets:B,,-4.2,EUR,,,,,,,46
a4c69b,Assets:A,,2.5,STK,,,,,100.25,EUR,48
a4c69b,Assets:B,,-250.625,EUR,,,,,,,49
2a69c7,Assets:A,,1,STK,,,,,1.2345,EUR,51
2a69c7,Assets:A,,2.25,EUR,,,,,,,52
2a69c7,Assets:B,,-3.48,EUR,,,,,,,53
e21c18,Assets:A,,2.5,STK,1.333,EUR,2026-01-15,,,,55
e21c18,Assets:A,,1.5,EUR,,,,,,,56
e21c18,Assets:B,,-4.8,EUR,,,,,,,57
7e1987,Assets:A,,10,EUR,,,,,,,59
7e1987,Assets:B,,-10,EUR,,,,,,,60
c66555,Assets:A,,0,EUR,,,,,,,66
```

interpolation-groups:

```text
bean-check: (no output, exit 0)
76a6bc,Assets:A,,1,USD,,,,,,,5
76a6bc,Assets:B,,-1,USD,,,,,,,7
76a6bc,Assets:A,,2,EUR,,,,,,,6
76a6bc,Assets:B,,-2,EUR,,,,,,,8
219cc8,Assets:B,,-1,USD,,,,,,,10
219cc8,Assets:A,,1,USD,,,,,,,11
219cc8,Assets:A,,2,EUR,,,,,,,12
219cc8,Assets:B,,-2,EUR,,,,,,,13
161a2a,Assets:A,,2,BTC,,,,,100,USD,15
161a2a,Assets:B,,-200,USD,,,,,,,17
161a2a,Assets:A,,5,EUR,,,,,,,16
161a2a,Assets:B,,-5,EUR,,,,,,,18
05d7fc,Assets:A,,5,EUR,,,,,,,20
05d7fc,Assets:B,,-5,EUR,,,,,,,22
05d7fc,Assets:A,,2,BTC,100,USD,2026-01-05,,,,21
05d7fc,Assets:B,,-200,USD,,,,,,,23
```

interpolation-several-currencies:

```text
bean-check: (no output, exit 0)
60ca95,Assets:A,,10.00,EUR,,,,,,,5
60ca95,Assets:A,,-3.3333,EUR,,,,,,,6
60ca95,Assets:B,,-6.67,EUR,,,,,,,8
60ca95,Assets:A,,5.5,USD,,,,,,,7
60ca95,Assets:B,,-5.5,USD,,,,,,,8
b38572,Assets:A,,10,EUR,,,,,,,10
b38572,Assets:B,,-10,EUR,,,,,,,11
b38572,Assets:A,,3,USD,,,,,,,12
b38572,Assets:A,,-3,USD,,,,,,,13
```

prices-exponent:

```text
bean-check: (no output, exit 0)
4c907d,Assets:A,,0.5,BTC,,,,,2.0E+2,EUR,5
4c907d,Assets:B,,-100,EUR,,,,,,,6
cc13ae,Assets:A,,0.25,BTC,,,,,4E+2,EUR,8
cc13ae,Assets:B,,-100,EUR,,,,,,,9
e63da1,Assets:A,,0.5,BTC,,,,,2.1E+2,EUR,11
e63da1,Assets:B,,-105,EUR,,,,,,,12
568d41,Assets:A,,0.05,BTC,,,,,2.00E+4,EUR,14
568d41,Assets:B,,-1000,EUR,,,,,,,15
44b744,Assets:A,,0.2,BTC,,,,,15,EUR,17
44b744,Assets:B,,-3.0,EUR,,,,,,,18
031031,Assets:A,,0.5,BTC,,,,,0E+1,EUR,20
98c941,Assets:A,,5,BTC,,,,,10.0,EUR,23
98c941,Assets:B,,-50.0,EUR,,,,,,,24
f5de04,Assets:A,,0.5,ETH,200,EUR,2026-01-09,,,,26
f5de04,Assets:B,,-100.0,EUR,,,,,,,27
9c3355,Assets:A,,1,BTC,,,,,1E-7,EUR,29
9c3355,Assets:B,,-1E-7,EUR,,,,,,,30
98149b,Assets:A,,1,BTC,1E-7,EUR,2026-01-11,,,,32
98149b,Assets:B,,-1E-7,EUR,,,,,,,33
925189,Assets:A,,1E-7,EUR,,,,,,,35
925189,Assets:B,,-1E-7,EUR,,,,,,,36
c51b59,Assets:A,,10000000,STK,1000000000,EUR,2026-01-13,,,,38
c51b59,Assets:B,,-10000000000000000,EUR,,,,,,,39
```

