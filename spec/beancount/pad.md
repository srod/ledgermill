# Pad

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Status: **built**. `transactions()` and `postings()` carry the padding transactions as described here, and `check()` reports
`unused-pad` and `pad-with-cost`. Probes: the `pad-*` journals, all run by the harness.

`pad ACCOUNT SOURCE` writes nothing by itself. It waits for a `balance` assertion (balance-directive.md)
and, if the account does not hold what the assertion says, the reference adds a transaction that
makes up the difference. The entries are walked in the order of check-order.md: by date, a
`balance` before everything else of its day, the rest by line.

## Rule

**Which balance pads.** A pad is *active* from its place in that order until another `pad` for the
same `ACCOUNT` replaces it. A `balance` on account `A` in currency `C` is handled by every active pad
whose `ACCOUNT` is `A` or an ancestor of `A` (`Assets:Bank` for `Assets:Bank:Sub`), unless that pad has
already handled `C`. A pad on a child, or on an unrelated account, ignores the assertion. A pad dated
the same day as the balance, or later, is not active yet: the balance comes first, so it is never
padded by it (`pad-same-date-as-balance`).

**Amount.** For each pad that handles the balance: `actual` is the sum of the units of `C` over the
padded account and all its descendants, in every transaction the reference keeps (booking-error
transactions are left out, unbalanced ones are kept) dated before the balance, including padding
transactions made for earlier balances, but not those made for this very balance (so a parent's pad
does not see what a child's pad adds for the same assertion, `pad-subaccounts`). The difference is
`expected - actual`, exact, with as many decimal places as the finer of the two. Transactions written
before or after the pad, on the pad's date or later, all count when they are dated before the balance
(`pad-same-day-order`), and the balance's own day does not count. A transaction the reference leaves out
does not count (`pad-rejected-transactions`: 500 asked, 10 held, 490 padded). Cost and price do not matter, only units.

**Tolerance.** If `|difference|` is within the tolerance of the assertion (balance-directive.md,
including an explicit `~`), no transaction is made. Equal to the tolerance counts as within
(`pad-unused`, `pad-amount`).

**The transaction.** Date of the pad, flag `P`, no payee, narration

```text
(Padding inserted for Balance of EXPECTED for difference DIFFERENCE)
```

where both amounts are plain decimals with their places, then a space and the currency
(`pad-narration-numbers`, where `render_commas` is on and changes nothing: `1,000.50` prints `1000.50`, `007.50` prints `7.50`, `+5` prints `5`, a
tiny number stays plain, `0.0000005`, though bean-query's `number` column then shows `5E-7`). Two
postings, in this order, with units only (no cost, price or flag): `ACCOUNT` gets `+difference`, then
`SOURCE` gets `-difference` (a negative difference swaps the signs, not the order). The transaction
sits on the pad's line; **both postings sit on the balance's line**. It takes part in later
calculations as a transaction of the pad's date.

**Currencies.** A pad handles each currency once: after the first assertion on `C` that it sees, a later
assertion on `C` is not padded, even when that first assertion needed nothing (`pad-consumed`). Assertions
in other currencies are still padded, whenever they come, each making its own transaction
(`pad-two-currencies`). A new `pad` on the account starts afresh (`pad-again`).

**Order of the generated transactions.** Made in the order the balances are walked (balance date,
then balance line), all with the pad's date and line. `transactions()` sorts like any transaction:
date, then line, so a pad transaction sits where the `pad` line stands, and several from one pad stay in
the order they were made. The flat postings list (postings.md) sorts by date and then the line of the
*posting*, which for padding is the **balance** line: padding rows move to where the balance lines
stand. In `pad-same-day-order` the padding transaction sits between transactions A and B in
`transactions()` (the `pad` is between them) but after B in the flat list (the `balance` line is after
both); in `pad-two-currencies` the USD transaction comes first in `transactions()` (its balance is the
earlier date) and the EUR rows first in the flat list (its balance line is the earlier line).

**Unused pad.** A pad that never made a transaction (no assertion arrived, the account already held
the amount or was within tolerance, or a later pad replaced it first) is an error: "Unused Pad entry"
on the pad's line. They are reported before any balance error, in pad order (date, then line)
(`pad-unused`). A replaced pad that did make a transaction is fine. bean-query still lists everything
else.

**Lots.** If the padded account (with descendants) holds a position with a cost in `C` at that moment
and a transaction is due, the reference reports "Attempt to pad an entry with cost for balance: (...)"
on the **balance** line (twice), and still adds the transaction, a plain-units one, without cost
(`pad-lots-at-cost`). Positions without cost, or lots in another currency, do not matter.

**Accounts.** The pad directive and its transaction are checked like any entry (check-errors.md). Each
reports its own errors on the **pad's line**: an unknown or inactive `ACCOUNT` or `SOURCE`, and a currency
that `ACCOUNT` or `SOURCE` does not allow. The transaction is still made (`pad-account-errors`). An
assertion on an unknown account, or on a currency its account does not allow, is also an error of the
balance itself (balance-directive.md).

The sub-account cases, on their own: a pad on the child with a balance on the parent is unused and the
balance fails (`pad-child-balance-parent`); a pad on the parent with a balance on the child pads the
parent, and the child's own assertion then fails (`pad-parent-balance-child`).

## Probe

`probes/pad.beancount`, `probes/pad-amount.beancount`, `probes/pad-narration-numbers.beancount`,
`probes/pad-same-day-order.beancount`, `probes/pad-two-currencies.beancount`, `probes/pad-again.beancount`,
`probes/pad-subaccounts.beancount`, `probes/pad-unused.beancount`, `probes/pad-consumed.beancount`,
`probes/pad-same-date-as-balance.beancount`, `probes/pad-child-balance-parent.beancount`,
`probes/pad-parent-balance-child.beancount`, `probes/pad-lots-at-cost.beancount`,
`probes/pad-account-errors.beancount`, `probes/pad-rejected-transactions.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/pad.beancount
spec/beancount/show.sh spec/beancount/probes/pad-amount.beancount
spec/beancount/show.sh spec/beancount/probes/pad-narration-numbers.beancount
spec/beancount/show.sh spec/beancount/probes/pad-same-day-order.beancount
spec/beancount/show.sh spec/beancount/probes/pad-two-currencies.beancount
spec/beancount/show.sh spec/beancount/probes/pad-again.beancount
spec/beancount/show.sh spec/beancount/probes/pad-subaccounts.beancount
spec/beancount/show.sh spec/beancount/probes/pad-unused.beancount
spec/beancount/show.sh spec/beancount/probes/pad-consumed.beancount
spec/beancount/show.sh spec/beancount/probes/pad-same-date-as-balance.beancount
spec/beancount/show.sh spec/beancount/probes/pad-child-balance-parent.beancount
spec/beancount/show.sh spec/beancount/probes/pad-parent-balance-child.beancount
spec/beancount/show.sh spec/beancount/probes/pad-lots-at-cost.beancount
spec/beancount/show.sh spec/beancount/probes/pad-account-errors.beancount
spec/beancount/show.sh spec/beancount/probes/pad-rejected-transactions.beancount
spec/beancount/show-postings.sh spec/beancount/probes/pad-same-day-order.beancount
spec/beancount/show-postings.sh spec/beancount/probes/pad-two-currencies.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` lines are the errors it printed (the copy of the directive after each is left out), in its order; a journal with an error still lists its transactions here, which the harness does not use.

## Output

pad:

```text
bean-check: (no output, exit 0)
f84e89,transaction,4,2026-01-02,P,,(Padding inserted for Balance of 500.00 EUR for difference 500.00 EUR)
f84e89,Assets:Bank,,500.00,EUR,,,,,,,5
f84e89,Equity:Opening,,-500.00,EUR,,,,,,,5
```

pad-amount:

```text
bean-check: (no output, exit 0)
bb2c56,transaction,9,2026-01-02,P,,(Padding inserted for Balance of 500.00 EUR for difference 510.00 EUR)
55a1d2,transaction,15,2026-01-02,*,,more than the balance asks for
27a46b,transaction,21,2026-01-02,*,,finer places than the balance
305156,transaction,30,2026-01-02,*,,just over an explicit tolerance
5d6b2a,transaction,10,2026-01-03,*,Shop,spends between the pad and the balance
26a99b,transaction,18,2026-01-03,P,,(Padding inserted for Balance of 500 EUR for difference -300 EUR)
b3c6e0,transaction,24,2026-01-03,P,,(Padding inserted for Balance of 500.1 EUR for difference 489.845 EUR)
3ec48c,transaction,27,2026-01-03,P,,(Padding inserted for Balance of 500.1 EUR for difference 500.1 EUR)
874321,transaction,33,2026-01-03,P,,(Padding inserted for Balance of 500.51 EUR for difference 0.51 EUR)
bb2c56,Assets:A,,510.00,EUR,,,,,,,13
bb2c56,Equity:Opening,,-510.00,EUR,,,,,,,13
55a1d2,Assets:B,,800,EUR,,,,,,,16
55a1d2,Equity:Opening,,-800,EUR,,,,,,,17
27a46b,Assets:C,,10.255,EUR,,,,,,,22
27a46b,Equity:Opening,,-10.255,EUR,,,,,,,23
305156,Assets:E,,500,EUR,,,,,,,31
305156,Equity:Opening,,-500,EUR,,,,,,,32
5d6b2a,Expenses:Food,,10.00,EUR,,,,,,,11
5d6b2a,Assets:A,,-10.00,EUR,,,,,,,12
26a99b,Assets:B,,-300,EUR,,,,,,,19
26a99b,Expenses:Food,,300,EUR,,,,,,,19
b3c6e0,Assets:C,,489.845,EUR,,,,,,,25
b3c6e0,Equity:Opening,,-489.845,EUR,,,,,,,25
3ec48c,Assets:D,,500.1,EUR,,,,,,,28
3ec48c,Equity:Opening,,-500.1,EUR,,,,,,,28
874321,Assets:E,,0.51,EUR,,,,,,,34
874321,Equity:Opening,,-0.51,EUR,,,,,,,34
```

pad-narration-numbers:

```text
bean-check: (no output, exit 0)
5b7eb3,transaction,9,2026-01-02,P,,(Padding inserted for Balance of 1000.50 EUR for difference 1000.50 EUR)
ce94e4,transaction,11,2026-01-02,P,,(Padding inserted for Balance of 7.50 EUR for difference 7.50 EUR)
84a9a9,transaction,13,2026-01-02,P,,(Padding inserted for Balance of -12.0 EUR for difference -12.0 EUR)
45f4e3,transaction,15,2026-01-02,P,,(Padding inserted for Balance of 5 EUR for difference 5 EUR)
26a99d,transaction,17,2026-01-02,P,,(Padding inserted for Balance of 0.0000005 EUR for difference 0.0000005 EUR)
5b7eb3,Assets:A,,1000.50,EUR,,,,,,,10
5b7eb3,Equity:Opening,,-1000.50,EUR,,,,,,,10
ce94e4,Assets:B,,7.50,EUR,,,,,,,12
ce94e4,Equity:Opening,,-7.50,EUR,,,,,,,12
84a9a9,Assets:C,,-12.0,EUR,,,,,,,14
84a9a9,Equity:Opening,,12.0,EUR,,,,,,,14
45f4e3,Assets:D,,5,EUR,,,,,,,16
45f4e3,Equity:Opening,,-5,EUR,,,,,,,16
26a99d,Assets:E,,5E-7,EUR,,,,,,,18
26a99d,Equity:Opening,,-5E-7,EUR,,,,,,,18
```

pad-same-day-order:

```text
bean-check: (no output, exit 0)
747856,transaction,5,2026-01-05,*,,"A: written before the pad,same date"
fb1ee4,transaction,8,2026-01-05,P,,(Padding inserted for Balance of 100.00 EUR for difference 103.00 EUR)
5d8975,transaction,9,2026-01-05,*,,"B: written after the pad,same date"
e2b28d,transaction,13,2026-01-06,*,,"C: same date as the balance,not counted"
747856,Expenses:Food,,1.00,EUR,,,,,,,6
747856,Assets:Bank,,-1.00,EUR,,,,,,,7
fb1ee4,Assets:Bank,,103.00,EUR,,,,,,,12
fb1ee4,Equity:Opening,,-103.00,EUR,,,,,,,12
5d8975,Expenses:Food,,2.00,EUR,,,,,,,10
5d8975,Assets:Bank,,-2.00,EUR,,,,,,,11
e2b28d,Expenses:Food,,4.00,EUR,,,,,,,14
e2b28d,Assets:Bank,,-4.00,EUR,,,,,,,15
```

pad-same-day-order, flat postings (show-postings.sh; the padding rows carry the balance line 12):

```text
2026-01-05,*,,"A: written before the pad,same date",Expenses:Food,1.00,EUR,F,6
2026-01-05,*,,"A: written before the pad,same date",Assets:Bank,-1.00,EUR,F,7
2026-01-05,*,,"B: written after the pad,same date",Expenses:Food,2.00,EUR,F,10
2026-01-05,*,,"B: written after the pad,same date",Assets:Bank,-2.00,EUR,F,11
2026-01-05,P,,(Padding inserted for Balance of 100.00 EUR for difference 103.00 EUR),Assets:Bank,103.00,EUR,F,12
2026-01-05,P,,(Padding inserted for Balance of 100.00 EUR for difference 103.00 EUR),Equity:Opening,-103.00,EUR,F,12
2026-01-06,*,,"C: same date as the balance,not counted",Expenses:Food,4.00,EUR,F,14
2026-01-06,*,,"C: same date as the balance,not counted",Assets:Bank,-4.00,EUR,F,15
```

pad-two-currencies:

```text
bean-check: (no output, exit 0)
9a9c32,transaction,4,2026-01-02,P,,(Padding inserted for Balance of 20 USD for difference 20 USD)
daa36b,transaction,4,2026-01-02,P,,(Padding inserted for Balance of 600.00 EUR for difference 600.00 EUR)
9a9c32,Assets:Bank,,20,USD,,,,,,,6
9a9c32,Equity:Opening,,-20,USD,,,,,,,6
daa36b,Assets:Bank,,600.00,EUR,,,,,,,5
daa36b,Equity:Opening,,-600.00,EUR,,,,,,,5
```

pad-two-currencies, flat postings (the lines are 5 for EUR, 6 for USD, the opposite of the transactions list):

```text
2026-01-02,P,,(Padding inserted for Balance of 600.00 EUR for difference 600.00 EUR),Assets:Bank,600.00,EUR,F,5
2026-01-02,P,,(Padding inserted for Balance of 600.00 EUR for difference 600.00 EUR),Equity:Opening,-600.00,EUR,F,5
2026-01-02,P,,(Padding inserted for Balance of 20 USD for difference 20 USD),Assets:Bank,20,USD,F,6
2026-01-02,P,,(Padding inserted for Balance of 20 USD for difference 20 USD),Equity:Opening,-20,USD,F,6
```

pad-again:

```text
bean-check: (no output, exit 0)
86b88b,transaction,5,2026-01-02,P,,(Padding inserted for Balance of 100.00 EUR for difference 100.00 EUR)
e84c50,transaction,7,2026-01-04,P,,(Padding inserted for Balance of 150.00 EUR for difference 50.00 EUR)
86b88b,Assets:Bank,,100.00,EUR,,,,,,,6
86b88b,Equity:Opening,,-100.00,EUR,,,,,,,6
e84c50,Assets:Bank,,50.00,EUR,,,,,,,8
e84c50,Expenses:Food,,-50.00,EUR,,,,,,,8
```

pad-subaccounts:

```text
bean-check: (no output, exit 0)
ae28b7,transaction,7,2026-01-02,*,,the child holds 100
436dce,transaction,10,2026-01-02,*,,the parent holds 1
ddc496,transaction,16,2026-01-02,P,,(Padding inserted for Balance of 100.00 EUR for difference 100.00 EUR)
aa767f,transaction,13,2026-01-03,P,,(Padding inserted for Balance of 500.00 EUR for difference 399.00 EUR)
b5586c,transaction,17,2026-01-03,P,,(Padding inserted for Balance of 100.00 EUR for difference 100.00 EUR)
ae28b7,Assets:Parent:Child,,100.00,EUR,,,,,,,8
ae28b7,Equity:Opening,,-100.00,EUR,,,,,,,9
436dce,Assets:Parent,,1.00,EUR,,,,,,,11
436dce,Equity:Opening,,-1.00,EUR,,,,,,,12
ddc496,Assets:Q:Sub,,100.00,EUR,,,,,,,18
ddc496,Equity:Opening,,-100.00,EUR,,,,,,,18
aa767f,Assets:Parent,,399.00,EUR,,,,,,,14
aa767f,Equity:Opening,,-399.00,EUR,,,,,,,14
b5586c,Assets:Q,,100.00,EUR,,,,,,,18
b5586c,Equity:Opening,,-100.00,EUR,,,,,,,18
```

pad-unused:

```text
bean-check: pad-unused.beancount:9: Unused Pad entry
bean-check: pad-unused.beancount:13: Unused Pad entry
bean-check: pad-unused.beancount:18: Unused Pad entry
bean-check: pad-unused.beancount:20: Unused Pad entry
bean-check: pad-unused.beancount:23: Unused Pad entry
bean-check: pad-unused.beancount:28: Unused Pad entry
476a7e,transaction,10,2026-01-02,*,,B already holds the balance
a39281,transaction,15,2026-01-02,*,,C is within the tolerance of the balance
6c02f8,transaction,25,2026-01-02,*,,F is within an explicit tolerance of the balance
836c9b,transaction,21,2026-01-04,P,,(Padding inserted for Balance of 300.00 EUR for difference 300.00 EUR)
476a7e,Assets:B,,500.00,EUR,,,,,,,11
476a7e,Equity:Opening,,-500.00,EUR,,,,,,,12
a39281,Assets:C,,500.004,EUR,,,,,,,16
a39281,Equity:Opening,,-500.004,EUR,,,,,,,17
6c02f8,Assets:F,,500,EUR,,,,,,,26
6c02f8,Equity:Opening,,-500,EUR,,,,,,,27
836c9b,Assets:D,,300.00,EUR,,,,,,,22
836c9b,Equity:Opening,,-300.00,EUR,,,,,,,22
```

pad-consumed:

```text
bean-check: pad-consumed.beancount:8: Unused Pad entry
bean-check: pad-consumed.beancount:7: Balance failed for 'Assets:A': expected 600.00 EUR != accumulated 500.00 EUR (100.00 too little)
bean-check: pad-consumed.beancount:10: Balance failed for 'Assets:B': expected 600.00 EUR != accumulated 0 EUR (600.00 too little)
5ab063,transaction,5,2026-01-02,P,,(Padding inserted for Balance of 500.00 EUR for difference 500.00 EUR)
5ab063,Assets:A,,500.00,EUR,,,,,,,6
5ab063,Equity:Opening,,-500.00,EUR,,,,,,,6
```

pad-same-date-as-balance:

```text
bean-check: pad-same-date-as-balance.beancount:4: Unused Pad entry
bean-check: pad-same-date-as-balance.beancount:5: Balance failed for 'Assets:Bank': expected 500.00 EUR != accumulated 0 EUR (500.00 too little)
```

pad-child-balance-parent:

```text
bean-check: pad-child-balance-parent.beancount:8: Unused Pad entry
bean-check: pad-child-balance-parent.beancount:9: Balance failed for 'Assets:Bank': expected 500.00 EUR != accumulated 100.00 EUR (400.00 too little)
f4e2e6,transaction,5,2026-01-02,*,,the child holds 100
f4e2e6,Assets:Bank:Sub,,100.00,EUR,,,,,,,6
f4e2e6,Equity:Opening,,-100.00,EUR,,,,,,,7
```

pad-parent-balance-child:

```text
bean-check: pad-parent-balance-child.beancount:12: Balance failed for 'Assets:Bank:Sub': expected 500.00 EUR != accumulated 100.00 EUR (400.00 too little)
d842c5,transaction,5,2026-01-02,*,,the child holds 100
ffe558,transaction,8,2026-01-02,*,,the parent holds 1
cf957e,transaction,11,2026-01-03,P,,(Padding inserted for Balance of 500.00 EUR for difference 399.00 EUR)
d842c5,Assets:Bank:Sub,,100.00,EUR,,,,,,,6
d842c5,Equity:Opening,,-100.00,EUR,,,,,,,7
ffe558,Assets:Bank,,1.00,EUR,,,,,,,9
ffe558,Equity:Opening,,-1.00,EUR,,,,,,,10
cf957e,Assets:Bank,,399.00,EUR,,,,,,,12
cf957e,Equity:Opening,,-399.00,EUR,,,,,,,12
```

pad-lots-at-cost:

```text
bean-check: pad-lots-at-cost.beancount:13: Attempt to pad an entry with cost for balance: (3 BTC {10 EUR, 2026-01-02}, 2 BTC {12 EUR, 2026-01-03})
bean-check: pad-lots-at-cost.beancount:13: Attempt to pad an entry with cost for balance: (3 BTC {10 EUR, 2026-01-02}, 2 BTC {12 EUR, 2026-01-03})
235f15,transaction,5,2026-01-02,*,,buy lot 1
5e4f36,transaction,8,2026-01-03,*,,buy lot 2
77dd84,transaction,12,2026-01-03,P,,(Padding inserted for Balance of 7 BTC for difference 2 BTC)
4c0b0e,transaction,12,2026-01-03,P,,(Padding inserted for Balance of 50 EUR for difference 50 EUR)
235f15,Assets:Stock,,3,BTC,10,EUR,2026-01-02,,,,6
235f15,Assets:Bank,,-30,EUR,,,,,,,7
5e4f36,Assets:Stock,,2,BTC,12,EUR,2026-01-03,,,,9
5e4f36,Assets:Bank,,-24,EUR,,,,,,,10
77dd84,Assets:Stock,,2,BTC,,,,,,,13
77dd84,Equity:Opening,,-2,BTC,,,,,,,13
4c0b0e,Assets:Stock,,50,EUR,,,,,,,14
4c0b0e,Equity:Opening,,-50,EUR,,,,,,,14
```

pad-account-errors:

```text
bean-check: pad-account-errors.beancount:17: Invalid currency 'USD' for Balance directive: 
bean-check: pad-account-errors.beancount:19: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:10: Invalid reference to unknown account 'Equity:Missing'
bean-check: pad-account-errors.beancount:10: Invalid reference to unknown account 'Equity:Missing'
bean-check: pad-account-errors.beancount:12: Invalid reference to inactive account 'Equity:Closed'
bean-check: pad-account-errors.beancount:12: Invalid reference to inactive account 'Equity:Closed'
bean-check: pad-account-errors.beancount:18: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:18: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:19: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:14: Invalid currency EUR for account 'Equity:Usd'
bean-check: pad-account-errors.beancount:16: Invalid currency USD for account 'Assets:D'
0690e0,transaction,10,2026-01-03,P,,(Padding inserted for Balance of 10.00 EUR for difference 10.00 EUR)
82d031,transaction,12,2026-01-03,P,,(Padding inserted for Balance of 10.00 EUR for difference 10.00 EUR)
977038,transaction,14,2026-01-03,P,,(Padding inserted for Balance of 10.00 EUR for difference 10.00 EUR)
700074,transaction,16,2026-01-03,P,,(Padding inserted for Balance of 10.00 USD for difference 10.00 USD)
3d5065,transaction,18,2026-01-03,P,,(Padding inserted for Balance of 10.00 EUR for difference 10.00 EUR)
0690e0,Assets:A,,10.00,EUR,,,,,,,11
0690e0,Equity:Missing,,-10.00,EUR,,,,,,,11
82d031,Assets:B,,10.00,EUR,,,,,,,13
82d031,Equity:Closed,,-10.00,EUR,,,,,,,13
977038,Assets:C,,10.00,EUR,,,,,,,15
977038,Equity:Usd,,-10.00,EUR,,,,,,,15
700074,Assets:D,,10.00,USD,,,,,,,17
700074,Equity:Opening,,-10.00,USD,,,,,,,17
3d5065,Assets:Never,,10.00,EUR,,,,,,,19
3d5065,Equity:Opening,,-10.00,EUR,,,,,,,19
```

pad-rejected-transactions:

```text
bean-check: pad-rejected-transactions.beancount:13: You may not have more than one auto-posting per currency
bean-check: pad-rejected-transactions.beancount:7: Transaction does not balance: (1 EUR)
5c3b82,transaction,7,2026-01-02,*,,"does not balance,but the postings count"
eacc0a,transaction,14,2026-01-03,P,,(Padding inserted for Balance of 500 EUR for difference 490 EUR)
5c3b82,Assets:Bank,,10,EUR,,,,,,,8
5c3b82,Assets:Other,,-9,EUR,,,,,,,9
eacc0a,Assets:Bank,,490,EUR,,,,,,,15
eacc0a,Equity:Opening,,-490,EUR,,,,,,,15
```

