# Lots and booking

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

Cost of a lot. `{100 EUR}` is a cost per unit, `{{300.00 EUR}}` a total (divided by the units, and
written plainly, 28 digits when inexact: `3 XRP {{10 EUR}}` is `3.333333333333333333333333333`),
and `{100 # 5 EUR}` a per-unit part plus a total part divided by the units (100 + 5/4 = `101.25`).
The lot's date is the transaction's date unless one is written (`{1.50 EUR, 2025-12-31, "lbl"}`),
and it may carry a label; a `;` inside the label is part of it. The weight of the posting is its
units times the cost per unit.

Adding and reducing. A posting with positive units and a cost adds a lot to its account. A
negative one reduces matching lots; the cost spec (`{}`, `{20 EUR}`, `{2026-01-02}`, `{"second"}`)
selects among the lots the account holds in that commodity, and the account's booking method
decides what happens when several lots match. A reduction spanning lots gives one row per lot (in
the order consumed), all with the line of the posting written; a price may follow (`{} @ 20 EUR`).
Each row shows the lot's cost, date and label.

A negative posting with a cost, in an account that holds none of that commodity (or only others),
adds a negative lot instead of failing; a positive posting with a matching cost then reduces it
(and shows the first lot's date).

Booking method, in order: the quoted word on the account's `open` directive, else
`option "booking_method"`, else STRICT.

- STRICT: one matching lot is used, also for a partial sale; several matching lots are an error.
- STRICT_WITH_SIZE: as STRICT, but when several match and one lot's size equals the sale, that
  lot is used.
- FIFO, LIFO, HIFO: lots are consumed oldest first (by date), newest first, or dearest first.
- NONE: no matching; a sale with a cost is another lot, a sale without one has no lot.
- AVERAGE: refused (refused.md).

An ambiguous reduction is rejected by bean-check at the posting's transaction, which bean-query
then leaves out of its results. ledgermill reports `booking-error` (`details.reason:
ambiguous-lot-match`) and does not return the transaction. The lot matching itself is done by
the booking crate (through `lima.rs`); the figures are ours.

## Probe

`probes/lots-costs.beancount`, `probes/lots-negative-without-lot.beancount`, `probes/booking-default-is-strict.beancount`, `probes/booking-strict-ambiguous.beancount`, `probes/booking-strict-single-lot.beancount`, `probes/booking-strict-with-size.beancount`, `probes/booking-fifo.beancount`, `probes/booking-lifo.beancount`, `probes/booking-hifo.beancount`, `probes/booking-none.beancount`, `probes/booking-option.beancount`, `probes/booking-selectors.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/lots-costs.beancount
spec/beancount/show.sh spec/beancount/probes/lots-negative-without-lot.beancount
spec/beancount/show.sh spec/beancount/probes/booking-default-is-strict.beancount
spec/beancount/show.sh spec/beancount/probes/booking-strict-ambiguous.beancount
spec/beancount/show.sh spec/beancount/probes/booking-strict-single-lot.beancount
spec/beancount/show.sh spec/beancount/probes/booking-strict-with-size.beancount
spec/beancount/show.sh spec/beancount/probes/booking-fifo.beancount
spec/beancount/show.sh spec/beancount/probes/booking-lifo.beancount
spec/beancount/show.sh spec/beancount/probes/booking-hifo.beancount
spec/beancount/show.sh spec/beancount/probes/booking-none.beancount
spec/beancount/show.sh spec/beancount/probes/booking-option.beancount
spec/beancount/show.sh spec/beancount/probes/booking-selectors.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

lots-costs:

```text
bean-check: (no output, exit 0)
c6387f,Assets:S,,2,BTC,100,EUR,2026-01-02,,,,5
c6387f,Assets:Bank,,-200,EUR,,,,,,,6
fe6950,Assets:S,,3,ETH,100.00,EUR,2026-01-03,,,,8
fe6950,Assets:Bank,,-300.00,EUR,,,,,,,9
f854e6,Assets:S,,3,XRP,3.333333333333333333333333333,EUR,2026-01-04,,,,11
f854e6,Assets:Bank,,-9.999999999999999999999999999,EUR,,,,,,,12
ab3aab,Assets:S,,4,SOL,101.25,EUR,2026-01-05,,,,14
ab3aab,Assets:Bank,,-405.00,EUR,,,,,,,15
fe07a0,Assets:S,,1,ADA,1.50,EUR,2025-12-31,lbl,,,17
fe07a0,Assets:Bank,,-1.50,EUR,,,,,,,18
526bac,Assets:S,,1,DOT,2,EUR,2026-01-07,a;b,,,20
526bac,Assets:Bank,,-2,EUR,,,,,,,21
```

lots-negative-without-lot:

```text
bean-check: (no output, exit 0)
35c7bc,Assets:A,,-10,EUR,2,BTC,2026-01-02,,,,5
35c7bc,Assets:S,,20,BTC,,,,,,,6
a5b2a5,Assets:S,,-5,EUR,4,BTC,2026-01-03,,,,8
a5b2a5,Assets:S,,20,BTC,,,,,,,9
c75310,Assets:A,,4,EUR,2,BTC,2026-01-02,,,,11
c75310,Assets:S,,-8,BTC,,,,,,,12
```

booking-default-is-strict:

```text
bean-check: booking-default-is-strict.beancount:9: Ambiguous matches for "-1 BTC {}": 1 BTC {10 EUR, 2026-01-02}, 1 BTC {20 EUR, 2026-01-02}
d0df1b,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
d0df1b,Assets:Stock,,1,BTC,20,EUR,2026-01-02,,,,7
d0df1b,Assets:Bank,,-30,EUR,,,,,,,8
```

booking-strict-ambiguous:

```text
bean-check: booking-strict-ambiguous.beancount:9: Ambiguous matches for "-1 BTC {}": 1 BTC {10 EUR, 2026-01-02}, 1 BTC {20 EUR, 2026-01-02}
9b6afd,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
9b6afd,Assets:Stock,,1,BTC,20,EUR,2026-01-02,,,,7
9b6afd,Assets:Bank,,-30,EUR,,,,,,,8
```

booking-strict-single-lot:

```text
bean-check: (no output, exit 0)
556032,Assets:Stock,,2,BTC,10,EUR,2026-01-02,,,,6
556032,Assets:Bank,,-20,EUR,,,,,,,7
4399df,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,,,,9
4399df,Assets:Bank,,15,EUR,,,,,,,10
4399df,Income:Gain,,-5,EUR,,,,,,,11
d25f52,Assets:Stock,,1,BTC,20,EUR,2026-01-11,,,,13
d25f52,Assets:Bank,,-20,EUR,,,,,,,14
318a55,Assets:Stock,,-1,BTC,20,EUR,2026-01-11,,,,16
318a55,Assets:Bank,,25,EUR,,,,,,,17
318a55,Income:Gain,,-5,EUR,,,,,,,18
7c5e47,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,,,,20
7c5e47,Assets:Bank,,25,EUR,,,,,,,21
7c5e47,Income:Gain,,-15,EUR,,,,,,,22
```

booking-strict-with-size:

```text
bean-check: (no output, exit 0)
29f2c3,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
29f2c3,Assets:Bank,,-10,EUR,,,,,,,7
9a4c1b,Assets:Stock,,2,BTC,20,EUR,2026-01-03,,,,9
9a4c1b,Assets:Bank,,-40,EUR,,,,,,,10
7ca7a9,Assets:Stock,,-2,BTC,20,EUR,2026-01-03,,,,12
7ca7a9,Assets:Bank,,50,EUR,,,,,,,13
7ca7a9,Income:Gain,,-10,EUR,,,,,,,14
```

booking-fifo:

```text
bean-check: (no output, exit 0)
f08dcb,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
f08dcb,Assets:Bank,,-10,EUR,,,,,,,7
40682d,Assets:Stock,,1,BTC,30,EUR,2026-01-03,,,,9
40682d,Assets:Bank,,-30,EUR,,,,,,,10
3a5c8a,Assets:Stock,,1,BTC,20,EUR,2026-01-04,,,,12
3a5c8a,Assets:Bank,,-20,EUR,,,,,,,13
e466b0,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,,,,15
e466b0,Assets:Stock,,-1,BTC,30,EUR,2026-01-03,,,,15
e466b0,Assets:Bank,,100,EUR,,,,,,,16
e466b0,Income:Gain,,-60,EUR,,,,,,,17
f77fa4,Assets:Stock,,-1,BTC,20,EUR,2026-01-04,,,,19
f77fa4,Assets:Bank,,25,EUR,,,,,,,20
f77fa4,Income:Gain,,-5,EUR,,,,,,,21
```

booking-lifo:

```text
bean-check: (no output, exit 0)
400713,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
400713,Assets:Bank,,-10,EUR,,,,,,,7
18df1d,Assets:Stock,,1,BTC,30,EUR,2026-01-03,,,,9
18df1d,Assets:Bank,,-30,EUR,,,,,,,10
94e09f,Assets:Stock,,1,BTC,20,EUR,2026-01-04,,,,12
94e09f,Assets:Bank,,-20,EUR,,,,,,,13
33de73,Assets:Stock,,-1,BTC,20,EUR,2026-01-04,,,,15
33de73,Assets:Stock,,-1,BTC,30,EUR,2026-01-03,,,,15
33de73,Assets:Bank,,100,EUR,,,,,,,16
33de73,Income:Gain,,-50,EUR,,,,,,,17
d3fc8e,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,,,,19
d3fc8e,Assets:Bank,,25,EUR,,,,,,,20
d3fc8e,Income:Gain,,-15,EUR,,,,,,,21
```

booking-hifo:

```text
bean-check: (no output, exit 0)
d806f6,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
d806f6,Assets:Bank,,-10,EUR,,,,,,,7
4bf2fd,Assets:Stock,,1,BTC,30,EUR,2026-01-03,,,,9
4bf2fd,Assets:Bank,,-30,EUR,,,,,,,10
0f1cc5,Assets:Stock,,1,BTC,20,EUR,2026-01-04,,,,12
0f1cc5,Assets:Bank,,-20,EUR,,,,,,,13
df7cfb,Assets:Stock,,-1,BTC,30,EUR,2026-01-03,,,,15
df7cfb,Assets:Stock,,-1,BTC,20,EUR,2026-01-04,,,,15
df7cfb,Assets:Bank,,100,EUR,,,,,,,16
df7cfb,Income:Gain,,-50,EUR,,,,,,,17
21fee1,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,,,,19
21fee1,Assets:Bank,,25,EUR,,,,,,,20
21fee1,Income:Gain,,-15,EUR,,,,,,,21
```

booking-none:

```text
bean-check: (no output, exit 0)
7e0ac6,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,6
7e0ac6,Assets:Bank,,-10,EUR,,,,,,,7
cb71d5,Assets:Stock,,1,BTC,30,EUR,2026-01-03,,,,9
cb71d5,Assets:Bank,,-30,EUR,,,,,,,10
3275fe,Assets:Stock,,-1,BTC,15,EUR,2026-01-10,,,,12
3275fe,Assets:Bank,,15,EUR,,,,,,,13
9b6ef9,Assets:Stock,,-1,BTC,,,,,20,EUR,15
9b6ef9,Assets:Bank,,20,EUR,,,,,,,16
```

booking-option:

```text
bean-check: (no output, exit 0)
4eccc1,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,8
4eccc1,Assets:Other,,1,BTC,10,EUR,2026-01-02,,,,9
4eccc1,Assets:Bank,,-20,EUR,,,,,,,10
9df09f,Assets:Stock,,1,BTC,30,EUR,2026-01-03,,,,12
9df09f,Assets:Other,,1,BTC,30,EUR,2026-01-03,,,,13
9df09f,Assets:Bank,,-60,EUR,,,,,,,14
7b9f8d,Assets:Stock,,-1,BTC,30,EUR,2026-01-03,,,,16
7b9f8d,Assets:Bank,,35,EUR,,,,,,,17
7b9f8d,Income:Gain,,-5,EUR,,,,,,,18
0a40d7,Assets:Other,,-1,BTC,10,EUR,2026-01-02,,,,20
0a40d7,Assets:Bank,,35,EUR,,,,,,,21
0a40d7,Income:Gain,,-25,EUR,,,,,,,22
```

booking-selectors:

```text
bean-check: (no output, exit 0)
99b69f,Assets:Stock,,1,BTC,10,EUR,2026-01-02,first,,,6
99b69f,Assets:Bank,,-10,EUR,,,,,,,7
bf732f,Assets:Stock,,1,BTC,10,EUR,2026-01-03,second,,,9
bf732f,Assets:Bank,,-10,EUR,,,,,,,10
286e5f,Assets:Stock,,1,BTC,12,EUR,2026-01-04,,,,12
286e5f,Assets:Bank,,-12,EUR,,,,,,,13
c3a12c,Assets:Stock,,-1,BTC,10,EUR,2026-01-03,second,,,15
c3a12c,Assets:Bank,,15,EUR,,,,,,,16
c3a12c,Income:Gain,,-5,EUR,,,,,,,17
9509e3,Assets:Stock,,-1,BTC,12,EUR,2026-01-04,,,,19
9509e3,Assets:Bank,,15,EUR,,,,,,,20
9509e3,Income:Gain,,-3,EUR,,,,,,,21
c658b0,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,first,20,EUR,23
c658b0,Assets:Bank,,20,EUR,,,,,,,24
c658b0,Income:Gain,,-10,EUR,,,,,,,25
```

