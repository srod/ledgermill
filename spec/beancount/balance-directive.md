# Balance assertions

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

`DATE balance ACCOUNT NUMBER [~ TOLERANCE] CURRENCY` asserts what an account holds. It adds no
transaction and no posting; a wrong assertion is an error, and a `pad` (pad.md) may add a transaction
before the check. Only the plain form exists: a cost, a price or the tolerance in the wrong place is a
syntax error (`balance-directive-syntax`).

## Rule

**When.** An assertion is checked at the start of its day: it sees every transaction dated *earlier*,
none dated the same day, wherever the transactions are written in the file (`balance-directive-day`).
Padding transactions made for earlier assertions count; so do transactions that do not balance, but not
transactions the reference leaves out for a booking error (`balance-directive-rejected-transactions`).

**What.** The sum of the units of `CURRENCY` in `ACCOUNT` and all its sub-accounts. An assertion on a
parent counts the children, one on a child only its own subtree (`balance-directive-subaccounts`). Cost
and price are ignored: lots are added up, an `@` price changes nothing, and the cost currency of a lot
is not held (`balance-directive-lots`). A currency the account never held sums to 0, so `0 USD` holds and
`5 USD` fails with "accumulated 0 USD" (`balance-directive-day`, `balance-directive-fails`). Negative
amounts are asserted as written (`balance-directive-tolerance`).

**Tolerance.** The assertion passes when `|accumulated - expected| <= tolerance` (equal passes).

- Without `~`, the tolerance is `10^-p`, `p` being the number of decimal places written in the
  assertion's own number: `500.00` allows 0.01, `500.0` allows 0.1, `500.000` allows 0.001, and `500`
  (no decimal place) allows nothing. It does not depend on the places of the transactions, and it is
  twice the half-place that transaction balancing allows (balancing.md).
- With `~ T`, the tolerance is exactly `T`, whatever the places (`500 ~ 0.5` allows 0.5, `500.00 ~ 0.001`
  allows 0.001, `~ 0` allows nothing). A negative `T` passes nothing, not even an exact match.

The same tolerance decides whether a pad pads (pad.md). Options that change tolerances are refused
(refused.md).

**Message.** One error per failed assertion, on the assertion's line:

```text
Balance failed for 'ACCOUNT': expected EXPECTED CURRENCY != accumulated ACCUMULATED CURRENCY (DIFFERENCE too much|too little)
```

`EXPECTED` is the number as written (its places, no tolerance); `ACCUMULATED` is the sum with the
finer places of its terms, `0` when nothing is held. `DIFFERENCE` is `|accumulated - expected|`; it says
"too much" when the account holds more than expected and "too little" otherwise, also when the
difference is 0 (a negative tolerance).

**Several on a day.** Each is checked alone. Assertions on the same account, currency and date that
differ from the *first* of them are also an error, "Duplicate balance assertion with different
amounts", on each such later one (in date, then line order). Numbers are compared exactly, places
aside: `-10` equals `-10.00`, but `-10.001` differs although it passes the tolerance. A third equal to
the first is fine (`balance-directive-duplicates`).

**Account errors** (`balance-directive-account-errors`).

- An account that is not open gives "Invalid reference to unknown account 'A'" **twice** on the line;
  an account whose `open` is after the assertion gives "Invalid reference to inactive account 'A'"
  once. An account closed before the assertion is fine.
- A currency the account's `open` does not list gives "Invalid currency 'USD' for Balance directive: "
  (with the trailing space) on the line.

The order in which these and the others are printed is in check-order.md.

## Probe

`probes/balance-directive.beancount`, `probes/balance-directive-day.beancount`,
`probes/balance-directive-subaccounts.beancount`, `probes/balance-directive-lots.beancount`,
`probes/balance-directive-tolerance.beancount`, `probes/balance-directive-fails.beancount`,
`probes/balance-directive-duplicates.beancount`, `probes/balance-directive-account-errors.beancount`,
`probes/balance-directive-rejected-transactions.beancount`, `probes/balance-directive-syntax.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/balance-directive.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-day.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-subaccounts.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-lots.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-tolerance.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-fails.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-duplicates.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-account-errors.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-rejected-transactions.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-syntax.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` lines are the errors it printed (the copy of the directive after each is left out), in its order. Accepted journals (no `bean-check` line) are the ones ledgermill must return unchanged; for the others, bean-query still lists the transactions, which the harness does not use.

## Output

balance-directive:

```text
bean-check: (no output, exit 0)
0223e6,transaction,4,2026-01-02,*,,pay
0223e6,Assets:Bank,,500.00,EUR,,,,,,,5
0223e6,Income:Pay,,-500.00,EUR,,,,,,,6
```

balance-directive-day:

```text
bean-check: (no output, exit 0)
2e0e9e,transaction,4,2026-01-02,*,,spends on the day of the first assertion
2e0e9e,Expenses:Food,,10.00,EUR,,,,,,,5
2e0e9e,Assets:Bank,,-10.00,EUR,,,,,,,6
```

balance-directive-subaccounts:

```text
bean-check: (no output, exit 0)
72dc4f,transaction,6,2026-01-02,*,,child
4a5a2d,transaction,9,2026-01-02,*,,grandchild
48f7f8,transaction,12,2026-01-02,*,,parent
72dc4f,Assets:Bank:Sub,,100.00,EUR,,,,,,,7
72dc4f,Equity:Opening,,-100.00,EUR,,,,,,,8
4a5a2d,Assets:Bank:Sub:Deep,,50.00,EUR,,,,,,,10
4a5a2d,Equity:Opening,,-50.00,EUR,,,,,,,11
48f7f8,Assets:Bank,,1.00,EUR,,,,,,,13
48f7f8,Equity:Opening,,-1.00,EUR,,,,,,,14
```

balance-directive-lots:

```text
bean-check: (no output, exit 0)
af51d5,transaction,6,2026-01-02,*,,buy lot 1
141b3a,transaction,9,2026-01-03,*,,buy lot 2
00f25c,transaction,12,2026-01-03,*,,"bought with a price,no lot"
af51d5,Assets:Stock,,3,BTC,10,EUR,2026-01-02,,,,7
af51d5,Assets:Bank,,-30,EUR,,,,,,,8
141b3a,Assets:Stock,,2,BTC,12,EUR,2026-01-03,,,,10
141b3a,Assets:Bank,,-24,EUR,,,,,,,11
00f25c,Assets:Other,,3,BTC,,,,,10,EUR,13
00f25c,Assets:Bank,,-30,EUR,,,,,,,14
```

balance-directive-tolerance (every assertion holds):

```text
bean-check: (no output, exit 0)
17a12a,transaction,14,2026-01-02,*,,all the accounts
17a12a,Assets:A,,500.01,EUR,,,,,,,15
17a12a,Assets:B,,499.99,EUR,,,,,,,16
17a12a,Assets:C,,500.1,EUR,,,,,,,17
17a12a,Assets:D,,500.001,EUR,,,,,,,18
17a12a,Assets:E,,500.0100,EUR,,,,,,,19
17a12a,Assets:F,,-500.01,EUR,,,,,,,20
17a12a,Assets:G,,500.5,EUR,,,,,,,21
17a12a,Assets:H,,500.001,EUR,,,,,,,22
17a12a,Assets:I,,500,EUR,,,,,,,23
17a12a,Assets:J,,0.01,EUR,,,,,,,24
17a12a,Assets:K,,1000.2,EUR,,,,,,,25
17a12a,Equity:Opening,,-4500.8,EUR,,,,,,,26
```

balance-directive-fails (dates run backwards in the file; the errors come out in date order):

```text
bean-check: balance-directive-fails.beancount:29: Balance failed for 'Assets:G': expected 5 USD != accumulated 0 USD (5 too little)
bean-check: balance-directive-fails.beancount:28: Balance failed for 'Assets:F': expected -500.00 EUR != accumulated 500 EUR (1000.00 too much)
bean-check: balance-directive-fails.beancount:27: Balance failed for 'Assets:E': expected 500 EUR != accumulated 500 EUR (0 too little)
bean-check: balance-directive-fails.beancount:26: Balance failed for 'Assets:D': expected 500 EUR != accumulated 500.51 EUR (0.51 too much)
bean-check: balance-directive-fails.beancount:25: Balance failed for 'Assets:C': expected 500 EUR != accumulated 500.0001 EUR (0.0001 too much)
bean-check: balance-directive-fails.beancount:24: Balance failed for 'Assets:B': expected 500.00 EUR != accumulated 499.989 EUR (0.011 too little)
bean-check: balance-directive-fails.beancount:23: Balance failed for 'Assets:A': expected 500.00 EUR != accumulated 500.011 EUR (0.011 too much)
bean-check: balance-directive-fails.beancount:30: Balance failed for 'Assets:H': expected 500.000 EUR != accumulated 500.0011 EUR (0.0011 too much)
bean-check: balance-directive-fails.beancount:31: Balance failed for 'Assets:I': expected 500.0 EUR != accumulated 500.11 EUR (0.11 too much)
```

balance-directive-duplicates:

```text
bean-check: balance-directive-duplicates.beancount:11: Balance failed for 'Assets:Bank': expected 1 USD != accumulated 0 USD (1 too little)
bean-check: balance-directive-duplicates.beancount:13: Balance failed for 'Assets:Bank': expected 0 EUR != accumulated -10.00 EUR (10.00 too little)
bean-check: balance-directive-duplicates.beancount:9: Duplicate balance assertion with different amounts
bean-check: balance-directive-duplicates.beancount:11: Duplicate balance assertion with different amounts
```

balance-directive-account-errors:

```text
bean-check: balance-directive-account-errors.beancount:6: Invalid currency 'USD' for Balance directive: 
bean-check: balance-directive-account-errors.beancount:7: Invalid reference to unknown account 'Assets:Never'
bean-check: balance-directive-account-errors.beancount:7: Invalid reference to unknown account 'Assets:Never'
bean-check: balance-directive-account-errors.beancount:8: Invalid reference to inactive account 'Assets:Late'
```

balance-directive-rejected-transactions:

```text
bean-check: balance-directive-rejected-transactions.beancount:13: You may not have more than one auto-posting per currency
bean-check: balance-directive-rejected-transactions.beancount:7: Transaction does not balance: (1 EUR)
d28a15,transaction,7,2026-01-02,*,,"does not balance,but the postings count"
d28a15,Assets:Unbalanced,,10,EUR,,,,,,,8
d28a15,Assets:Other,,-9,EUR,,,,,,,9
```

balance-directive-syntax:

```text
bean-check: balance-directive-syntax.beancount:7: syntax error, unexpected LCURL, expecting end of file or EOL
bean-check: balance-directive-syntax.beancount:8: syntax error, unexpected TILDE, expecting end of file or EOL
bean-check: balance-directive-syntax.beancount:9: syntax error, unexpected TILDE, expecting end of file or EOL
bean-check: balance-directive-syntax.beancount:10: syntax error, unexpected AT, expecting end of file or EOL
bean-check: balance-directive-syntax.beancount:12: syntax error, unexpected EOL
5606b6,transaction,4,2026-01-02,*,,buy
5606b6,Assets:Stock,,3,BTC,10,EUR,2026-01-02,,,,5
5606b6,Assets:Bank,,-30,EUR,,,,,,,6
```

