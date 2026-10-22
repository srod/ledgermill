# Balance assignments: the amount filled in

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

A posting with an assertion and no amount (`acc  = B`; also `==`, `=*`, `==*`; two blanks before
the `=`) is a balance assignment. hledger works out its amount so that the assertion holds, and
`print -O json` shows that amount in `pamount`. How the assignment takes part in balancing,
and in what order it is worked out, is in [assignment-processing](assignment-processing.md);
costs and lots are in [assignment-lots-and-costs](assignment-lots-and-costs.md). Here is the
amount itself.

**The balance** is the one of [assertions](assertions.md), at the posting's place in the order
of [assertion-order](assertion-order.md) (assignments are worked out in the same walk).

**The amount, `=`.** `B - balance` in the asserted commodity C, as one amount. Other commodities
of the account are left alone. An account never used has the balance 0 (`d = 5 EUR` gives 5 EUR);
the result can be negative; a number with no commodity is the commodity `""`.

**The amount, `==`.** The amount for C as above, then, for every other commodity D the account
holds, `-balance(D)`: all as separate amounts of the one posting, in code point order of the
commodity (`""`, `AA`, `EUR`, `ZZ`, `mm`). A commodity whose result is zero is not listed
(`3 USD` then `-3 USD`, or EUR already right). `== 10 JPY` on an account holding EUR and USD
gives `-EUR`, `10 JPY`, `-USD`. `== 0 EUR` and `== 0` empty the account.

**The amount, `=*` and `==*`.** The same, but the balance of each commodity is the sum over the
account and its sub-accounts, and the amount still goes on the posting's own account: with
`a` 1, `a:b` 2, `a:c:d` 3, `a =* 10 EUR` gives 4 EUR (`ab` is not below `a`). `==*` zeroes the
other commodities of the whole subtree on the account itself, even those held by sub-accounts.

**A zero result is no amount.** When the difference is zero the posting gets an empty `pamount`
(`[]`), not a `0`; so does the posting that takes the balancing amount when nothing else is in
the group (the `z` of the probes). For `==` the posting is empty when every commodity is already
right.

**Places.** The difference is computed like the sums of
[elided-amounts](elided-amounts.md): the running balance is the sum of the amounts, in order, where
a zero term gives back the other term with its places and two non-zero terms give the sum with
the larger number of places; the amount is `B - balance` by the same rule (so `B` or `-balance`
comes back as written when the other side is zero).

| balance | `= B` | amount |
|---|---|---|
| `3` | `0.00` | `-3` (zero `B`: the balance comes back, negated) |
| `3.00` | `0` | `-3.00` |
| `3.00` | `1.5` | `-1.50` |
| `3` | `1.50` | `-1.50` |
| `1.5` | `3` | `1.5` |
| `1.250` | `3.5` | `2.250` |
| `0.00` | `3` | `3` (a zero balance gives `B` back) |
| `0.00` | `3.00` | `3.00` |
| `3` | `3.00` | none |
| `-3.00` | `3` | `6.00` |
| `2.5` | `-2.5` | `-5.0` |
| `0.5` | `0.51` | `0.01` |
| `1` | `1.001` | `0.001` |

The running balance itself is built that way too: `1.25`, `3`, `-1.25` give `3.00`, so
asserting `4` gives `1.00`; `1.25`, `-1.25`, `3` give `3` (the sum went through zero), so `1`;
`1.5` and `0.50` give `2.00`; a zero amount before or after a `3` leaves `3`.
A plain assertion does not change the balance: after `1 EUR = 1.00 EUR`, asserting `4` gives `3`.

**What the balance is afterwards.** For `=`, exactly the asserted amount, as written, with its
places: after `a = 2.250 EUR` (zero difference) the balance is `2.250`, so `a = 3` gives
`0.750`; after `a = 3 EUR` it is `3` again with no places. This holds when the difference was
zero and when the assertion has more places than the sum. For `==`, the account then holds the
asserted amount only: the other commodities are gone, so a later `a = 0 ZZ` gives nothing.
For `=*`, the account's own balance becomes `B` minus the sub-accounts' sum, computed as above,
also on a zero difference (`i` 1 and `i:b` 2.5, `i =* 3.5` gives nothing, and the own balance is
now `3.5 - 2.5 = 1.0`, so `i = 2` gives `1.0`). The sub-accounts keep their balances.

**Places of a `=*` amount.** With a star, the amount for the asserted commodity is worked out in two steps,
`(B - sub-accounts' sum) - own`, which is also the account's balance afterwards less its own; the
result is that of `B - (own + sub-accounts)` except in the places when the whole is zero. With `a` -3.00,
`a:d` 3.00 and `a =* 1 EUR` the amount is `1.00` (`1 - 3.00` is `-2.00`, minus `-3.00`), where
one step, `1 - 0.00`, would give `1` (probe `assign-inclusive-two-steps`, line 6: `a [1.00 EUR]`, `z [-1.00 EUR]`).

**Other commodities with a star.** `=*` alone (no second `=`) also zeroes the commodities that
sub-accounts hold, though not those held only by the account itself. For each commodity D other than
the asserted one that some sub-account holds, the posting gets `-(sub-accounts' share of D)`, where the
share is the subtree's sum less the account's own balance in D; the account then holds its own D
plus that amount, so a later `a = 0 D` gives the share back. A zero share is not listed.

- The places of the share: plain arithmetic, `whole - own`, with two exceptions. When the account's own
  balance in D is zero nothing is taken off, so the share is the subtree's sum as it is. When the share
  equals the own balance in value, it has the places of the own balance (`f` 1 and `f:g` 1.00 give `-1`,
  not `-1.00`; `f` 1.0 and `f:g` 1.000 give `-1.0`; `f` 1 and `f:g` 1.01 give `-1.01`; a share of the
  same value with fewer places is rare and is not probed).
  A zero whole is no exception to the plain arithmetic (`f` 1 and `f:g` -1.00 give `1.00`).
- `==*` already zeroes these (it zeroes every commodity of the subtree); the rule above is for `=*`.

Probe `probes/assign-inclusive-other.journal`, command `spec/hledger/show.sh` on it. Lines 9 and 15
(`a` holds -2.00 X and `a:b` -1 X; `d:b:c` 2 Y; `f` 1 Y and `f:g` 1.00 Y):

```text
9:  a [4 EUR, 1.00 X]   d [-2 Y]   f [-1 Y]   z [-4 EUR, -1.00 X, 3 Y]
15: a [1.00 X]          (a = 0 X: the own -2.00 plus the added 1.00 is -1.00)
```

**Not settled: the order of the sub-accounts' sum.** The sum over sub-accounts is made in an order
that is not the order of the names or of the file, and the places of the result depend on it
only when a partial sum is exactly zero: three sub-accounts holding `1.25`, `3`, `-1.25`, and
`=* 4`, give `1.00` when they are named `a:b`, `a:c`, `a:d` and `1` when they are `a:x1`, `a:x2`,
`a:x3`. No order by name, by first posting or by depth explains both; ledgermill should add them in
name order and accept a difference of places in this case. The value is never affected.

## Probe

`probes/assign-basic.journal`, `probes/assign-diff-places.journal`,
`probes/assign-sum-places.journal`, `probes/assign-total.journal`,
`probes/assign-inclusive.journal`, `probes/assign-inclusive-order-names-bcd.journal`,
`probes/assign-inclusive-order-names-x123.journal`, `probes/assign-inclusive-other.journal`,
`probes/assign-inclusive-two-steps.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/assign-basic.journal
spec/hledger/show.sh spec/hledger/probes/assign-diff-places.journal
spec/hledger/show.sh spec/hledger/probes/assign-sum-places.journal
spec/hledger/show.sh spec/hledger/probes/assign-total.journal
spec/hledger/show.sh spec/hledger/probes/assign-inclusive.journal
spec/hledger/show.sh spec/hledger/probes/assign-inclusive-other.journal
spec/hledger/show.sh spec/hledger/probes/assign-inclusive-two-steps.journal
spec/hledger/show.sh spec/hledger/probes/assign-inclusive-order-names-bcd.journal
spec/hledger/show.sh spec/hledger/probes/assign-inclusive-order-names-x123.journal
```

## Output

assign-basic, lines 9-17 (the balance `a` is 2.25, then 3 after an assignment of 3; every `z` is
the posting without an amount that takes the balance):

```text
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a zero difference leaves the posting with no amount, and z too","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[]},{"account":"z","type":"RegularPosting","comment":"","amounts":[]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"the balance is now 2.250, three places","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",750,3]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-750,3]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"and 3 again: no amount, and the balance is now exactly 3 (no places)","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[]},{"account":"z","type":"RegularPosting","comment":"","amounts":[]}]}
```

assign-basic, the rest, as `[commodity, mantissa, places]` (line 5: 0.75; line 21: -2; line 25: 5 on
an unused account; line 29: none; line 33: -5.5):

```text
5: a [["EUR",75,2]]    21: a [["EUR",-2,0]]    25: c [["EUR",5,0]]    29: d []    33: e [["EUR",-55,1]]
```

assign-basic line 37, a commodity of its own and another commodity; line 42, `c = 5 EUR` and `c = 0 USD`
(the EUR was right already):

```text
{"date":"2026-01-10","status":"Unmarked","code":"","description":"a commodity of its own, and another commodity left alone","comment":"","line":37,"postings":[{"account":"c","type":"RegularPosting","comment":"","amounts":[["",7,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["USD",2,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["",-7,0],["USD",-2,0]]}]}
{"date":"2026-01-11","status":"Unmarked","code":"","description":"an assignment of another commodity does not touch EUR","comment":"","line":42,"postings":[{"account":"c","type":"RegularPosting","comment":"","amounts":[]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["USD",-2,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["USD",2,0]]}]}
```

assign-diff-places: each account `p1`..`p16` has one posting of the balance (a transaction in
February), then an assignment (March). The table above is read from the March lines, e.g. `p1`
(3, asserting 0.00) and `p6` (1.250, asserting 3.5):

```text
{"date":"2026-03-01","status":"Unmarked","code":"","description":"assert 0.00","comment":"","line":5,"postings":[{"account":"p1","type":"RegularPosting","comment":"","amounts":[["EUR",-3,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]}]}
{"date":"2026-03-06","status":"Unmarked","code":"","description":"assert 3.5","comment":"","line":45,"postings":[{"account":"p6","type":"RegularPosting","comment":"","amounts":[["EUR",2250,3]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-2250,3]]}]}
```

assign-sum-places, line 45: eight accounts with the balances of the setup (`x1` 1.25, 3, -1.25;
`x2` 1.25, -1.25, 3; `x3` 1.5 and 0.50; `x4` 0.00, 3; `x5` 3, 0.00; `x6` 1.25, -1.25; `x7` and `x8`
a plain assertion of `1.00` on a balance of 1), all asserting 4:

```text
{"date":"2026-01-05","status":"Unmarked","code":"","description":"now the assignments, all asserting 4","comment":"","line":45,"postings":[{"account":"x1","type":"RegularPosting","comment":"","amounts":[["EUR",100,2]]},{"account":"x2","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"x3","type":"RegularPosting","comment":"","amounts":[["EUR",200,2]]},{"account":"x4","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"x5","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"x6","type":"RegularPosting","comment":"","amounts":[["EUR",4,0]]},{"account":"x7","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]},{"account":"x8","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-1600,2]]}]}
```

assign-total, lines 22 (`a` held 5 EUR, 2 ZZ, 3 AA, 4 mm; `== 7 EUR`), 26 (`b` 10 EUR and a USD
that went to zero; `== 10 EUR`), 30 (`== 12 EUR`), 38 (`g` held 3 USD and 4 EUR; `== 10 JPY`), 54
(after `== 11 EUR` nothing else is left: `= 0 ZZ`, `= 0 mm` and `== 11.0 EUR` give nothing), 61
(`h` held 4 EUR and 3 USD: `h = 9 EUR` leaves the USD, then `h == 9 EUR` takes it away):

```text
{"date":"2026-01-02","status":"Unmarked","code":"","description":"== makes the account hold the asserted amount and nothing else","comment":"","line":22,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["AA",-3,0],["EUR",2,0],["ZZ",-2,0],["mm",-4,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["AA",3,0],["EUR",-2,0],["ZZ",2,0],["mm",4,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"the zero USD sum is not listed, so the posting has no amount at all","comment":"","line":26,"postings":[{"account":"b","type":"RegularPosting","comment":"","amounts":[]},{"account":"z","type":"RegularPosting","comment":"","amounts":[]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"zero USD sum is not listed, but EUR differs","comment":"","line":30,"postings":[{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",2,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-2,0]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"asserted commodity not in the account","comment":"","line":38,"postings":[{"account":"g","type":"RegularPosting","comment":"","amounts":[["EUR",-4,0],["JPY",10,0],["USD",-3,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",4,0],["JPY",-10,0],["USD",3,0]]}]}
{"date":"2026-01-10","status":"Unmarked","code":"","description":"after ==, the balance is exactly the asserted amount","comment":"","line":54,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",4,0]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[]},{"account":"a","type":"RegularPosting","comment":"","amounts":[]},{"account":"a","type":"RegularPosting","comment":"","amounts":[]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-4,0]]}]}
{"date":"2026-01-11","status":"Unmarked","code":"","description":"a single-commodity assignment leaves other commodities alone: the USD is still there","comment":"","line":61,"postings":[{"account":"h","type":"RegularPosting","comment":"","amounts":[["EUR",5,0]]},{"account":"h","type":"RegularPosting","comment":"","amounts":[["USD",-3,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-5,0],["USD",3,0]]}]}
```

assign-inclusive: line 16 (`a =* 10 EUR`: 10 minus 1, 2 and 3), 20 (`a = 5 EUR` afterwards:
nothing, so the own balance is now 5), 24 (`h` has no postings of its own, `h:b` 2), 28 (`i`),
33 (`k ==* 10 EUR`, with `k:b` 2 EUR, `k:c` 3 USD, `k` 4 GBP), 37 (`k:b = 0 EUR`, then `k ==* 10 EUR`
adds the 2 EUR back):

```text
{"date":"2026-01-02","status":"Unmarked","code":"","description":"=* counts a and everything under a:, not ab","comment":"","line":16,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",4,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-4,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"then = sees only a itself: asserted minus the sub-accounts","comment":"","line":20,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[]},{"account":"z","type":"RegularPosting","comment":"","amounts":[]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"an account with no postings of its own","comment":"","line":24,"postings":[{"account":"h","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-3,0]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"zero difference, but the account's own balance becomes 3.5 - 2.5 = 1.0","comment":"","line":28,"postings":[{"account":"i","type":"RegularPosting","comment":"","amounts":[]},{"account":"i","type":"RegularPosting","comment":"","amounts":[["EUR",10,1]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-10,1]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"==* zeroes every other commodity, in the account itself","comment":"","line":33,"postings":[{"account":"k","type":"RegularPosting","comment":"","amounts":[["EUR",8,0],["GBP",-4,0],["USD",-3,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-8,0],["GBP",4,0],["USD",3,0]]}]}
{"date":"2026-01-07","status":"Unmarked","code":"","description":"a sub-account changes, then ==* again: only the change is added","comment":"","line":37,"postings":[{"account":"k:b","type":"RegularPosting","comment":"","amounts":[["EUR",-2,0]]},{"account":"k","type":"RegularPosting","comment":"","amounts":[["EUR",2,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",0,0]]}]}
```

The two ordering journals (line 7 of each, `a =* 4 EUR`; names `b c d`, then `x1 x2 x3`):

```text
{"date":"2026-01-02","status":"Unmarked","code":"","description":"asserting 4 for a and all below","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",100,2]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-100,2]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"asserting 4 for a and all below","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"z","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]}]}
```
