# Valuation: arithmetic, rounding and the amounts of a posting

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

This note covers `hledger print -O json --value=DATE,COMM` once the path is chosen
([value-prices](value-prices.md) says which prices exist and which path is followed): the rate
of a path, the rounding, what happens to the cost and the lot of an amount, and how the several amounts
of one posting are merged. The engine call is `Ledger::value(date, commodity)`.
Output lines are made by `spec/hledger/show-amounts.sh` (`DATE  ACCOUNT: AMOUNT | ...`, an amount
being a commodity and a number with exactly its places). Most probes use virtual postings, which need not balance.

Numbers are exact decimals: the engine needs integers of several hundred digits (a rate has up to 255 places,
and an amount may be 1e245), not floating point.

## Rule: the value of one amount

For an amount `Q C` (`Q` a quantity of commodity `C`, written with its places) and a target `COMM`:

1. `C` is `COMM`: the amount is left as it is: same places, **price and lot kept**.
2. No path from `C` to `COMM`: the same, price and lot kept. A commodity-less amount is an amount of the
   empty commodity `""`, and follows the same two rules (it is converted only by a price whose commodity
   is empty).
3. Otherwise the amount becomes `V COMM` with **no price and no lot**: whatever it had is dropped, and it
   is not used for the conversion (a cost is not a price, [value-prices](value-prices.md)). `V` is made so:
   1. the rate `R` of the path. For the edges `r1, r2, ... rn` of the path, from the amount's commodity to `COMM`
      (forward edges have the price as rate, inverse edges have the inverse rate): `R = r1`, then
      for each next edge `R = round255(R × rk)`;
   2. `V = round255(Q × R)`;
   3. strip trailing zeros of `V` (`3.0` is `3`; an exact zero has no places);
   4. if `V` still has more than ten places, round it half to even to ten places and keep the ten places, trailing
      zeros included (`1.0000000000`; a non-zero value that rounds to zero is `0.0000000000`).

   `round255(x)` rounds half to even to 255 decimal places, and leaves `x` alone when it has no more than 255.
   Step 4 is the only rounding to ten places, and it happens once, at the end.

The fold is from the amount's side, rate first, then the amount: `((r1 × r2) × r3)` rounded at each product, then
`Q ×` that. Multiplying the amount through each edge in turn, or folding from the target's side, or not rounding
the products of the rates, gives a different last digit (below). The last product `Q × R` is capped
too; no probe could tell whether it is, since a cap at 255 places can only change the ten printed places through a
run of 245 nines.

### Probe

`probes/value-fold-order.journal`, `probes/value-round-half-even.journal`,
`probes/value-zero-and-places.journal`, `probes/value-amount-forms.journal`

### Command and output

Rounding half to even at ten places (X is worth exactly 1 EUR; the amounts have 11 to 15 places and are exact):
5e-11 gives 0, 1.5e-10 and 2.5e-10 both give 2e-10, 3.5e-10 gives 4e-10, 2.50001e-10 gives 3e-10, the
sign changes nothing; `0.12345678905` ends on an even digit, `0.12345678915` on an odd one:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-round-half-even.journal
2026-01-02  a: EUR 0.0000000000 | b: EUR 0.0000000002 | c: EUR 0.0000000002 | d: EUR 0.0000000004 | e: EUR -0.0000000002 | f: EUR -0.0000000002 | g: EUR 0.0000000003 | h: EUR 0.1234567890 | i: EUR 0.1234567892
```

Trailing zeros and zeros (X is 2 EUR, Y is 0.5 EUR, Z is 0 EUR): `1.50 X` gives `3`; `1.25 Y` gives `0.625`; `0.00 X`
and `7 Z` give `0`; `0.0000000001 Y` rounds to zero and keeps ten places; `100.0000 Y` gives `50`; an amount already
in EUR keeps its places (`2.500`, `0.00`):

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-zero-and-places.journal
2026-01-02  a: EUR 3 | b: EUR 0.625 | c: EUR 0 | d: EUR 0 | e: EUR 0.0000000000 | f: EUR 2.500 | g: EUR 0.00 | h: EUR 50 | i: EUR 0.0000000000
```

The fold. A is 0.3 B, C is 0.3 B, D is 0.07 C; 1e245 A is valued in D along A to B (forward), B to C (inverse
of 0.3), C to D (inverse of 0.07), so `R = round255(round255(0.3 × (1/0.3)) × (1/0.07))` and the amount is
multiplied last. The printed result ends in `.4285714286`. Folding from the other end gives `.4285714284`;
multiplying the amount through each edge in turn, or not rounding the products of the rates, gives `.4285714285`.
(A throwaway script with the same 255-place arithmetic checked these alternatives on 80 random journals with
paths of 2 to 5 edges and amounts from 1e100 to 3e250: the order above gave hledger's result in all 80, the
other three in 74, 64 and 64.)

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,D spec/hledger/probes/value-fold-order.journal
2026-01-02  x: D 1428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571.4285714286
```

Cost, lot and commodity-less amounts (X is 2 EUR, EUR is 1.1 USD; a cost `@ 4 EUR`, `@@ 12 EUR`, a lot `{4 EUR}`, both,
a lot with a date and a label). Valued in EUR, X amounts become 6 EUR (3 × 2, the cost and lot dropped), while the
EUR amounts are left with their cost or lot; `Y` has no path; the bare `3` has no path. Valued in USD, EUR amounts
convert, losing their cost or lot. Valued in X, the X amounts are left whole, and the EUR ones convert (3 EUR to 1.5 X by
the inverse of the price of X):

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-amount-forms.journal
2026-01-02  a: EUR 6 | b: EUR 6 | c: EUR 6 | d: EUR 6 | e: EUR 6 | f: EUR 3 @ USD 4 | g: Y 3 @ EUR 4 | h:  3 | i: EUR 3 {USD 4}
$ spec/hledger/show-amounts.sh --value=2026-01-02,USD spec/hledger/probes/value-amount-forms.journal
2026-01-02  a: USD 6.6 | b: USD 6.6 | c: USD 6.6 | d: USD 6.6 | e: USD 6.6 | f: USD 3.3 | g: Y 3 @ EUR 4 | h:  3 | i: USD 3.3
$ spec/hledger/show-amounts.sh --value=2026-01-02,X spec/hledger/probes/value-amount-forms.journal
2026-01-02  a: X 3 @ EUR 4 | b: X 3 @@ EUR 12 | c: X 3 {EUR 4} | d: X 3 @ EUR 5 {EUR 4} | e: X 3 {EUR 4} | f: X 1.5 | g: Y 3 @ EUR 4 | h:  3 | i: X 1.5
```

## Rule: a posting with several amounts

An ordinary posting has one amount. Several come with an **elided amount** (the posting is worked out as the
negated sum of the others, one amount per commodity, in name order, `""` first; see [elided-amounts](elided-amounts.md)) and with
a `==` assignment ([assignments](assignments.md)). Each amount is valued by the rules above, but the results
are then **merged per commodity**:

1. every amount is converted at full precision: the stripped `V` of step 3, **not** yet rounded to ten places;
2. the amounts that end in the same commodity (converted ones, and the ones already in `COMM`) are summed, in the order
   the posting lists them. The sum starts from the first. Adding a zero leaves the other amount as it is, with its
   places (the first zero kept when both are zero): a zero `b` added to `a` gives `a`; a zero `a` added to
   `b` gives `b`. Otherwise the sum has the places of the one with more places (`1.25 + 0.75` is `2.00`);
3. the sum is not stripped. If it has more than ten places it is rounded to ten, half to even, as in step 4;
4. a zero sum is kept as an amount (`COMM 0`). A posting never loses an amount: unconverted commodities stay,
   each one;
5. the list is ordered by commodity name, code point order, `""` first.

200 random journals (one to four postings, 0 to 3 places, direct and inverse prices, a different target each) gave
the amounts this merge rule predicts; none had a lot.

A **lot** is kept on a merged amount only when the amount is alone in its commodity and was not converted.
The amount of a converted commodity never has a lot; two amounts that merge never do (a converted zero
counts as an amount).

### Probe

`probes/value-merge-sum.journal`, `probes/value-merge-order.journal`, `probes/value-merge-zero-places.journal`,
`probes/value-lots-merge.journal`

### Command and output

Sum at full precision, rounded once: the elided posting `b` holds `-1 T, -1 U, -1 V`; each is a third of an X
(`P X 3 T` and so on). Plain `print` first, then valued in X; `a`'s amounts are `0.3333333333` each, `b` is -1, not -0.9999999999:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/value-merge-sum.journal
2026-01-02  a: T 1 | a: U 1 | a: V 1 | b: T -1, U -1, V -1
$ spec/hledger/show-amounts.sh --value=2026-01-02,X spec/hledger/probes/value-merge-sum.journal
2026-01-02  a: X 0.3333333333 | a: X 0.3333333333 | a: X 0.3333333333 | b: X -1.0000000000
```

Order, and the empty commodity. B is 2 EUR, `b` is 3 EUR, A is 1 EUR. The first `y` is
`C -1, EUR -7, Z -1, a -1`: A, B, EUR and b all join the EUR sum, in the order A, B, EUR, b (1 + 2 + 1 + 3), then C, Z, a
stay, in code point order (`Z` before `a`). In the second, the empty commodity comes first:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-merge-order.journal
2026-01-02  x: EUR 1 | x: EUR 2 | x: C 1 | x: EUR 1 | x: EUR 3 | x: Z 1 | x: a 1 | y: C -1, EUR -7, Z -1, a -1
2026-01-02  x: EUR 2 | x:  4 | y:  -4, EUR -2
```

Places of a sum. A, B, N and O are worth 1.25 M. The elided `y` holds A 1, B -1, M 3: the first two make a zero (`0.00`
before the next step, since 1.25 − 1.25), adding `M 3` to a zero gives `3`; with `M 3, N 1, O -1` the sum is
3 + 1.25 − 1.25, which is `3.00`. Two zeros: converted zeros have no places, so `A 0` then `M 0.00` keeps `0`,
and `M 0.00` then `N 0` keeps `0.00`:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,M spec/hledger/probes/value-merge-zero-places.journal
2026-01-02  x: M -1.25 | x: M 1.25 | x: M -3 | y: M 3
2026-01-02  x: M -3 | x: M -1.25 | x: M 1.25 | y: M 3.00
2026-01-02  x: M 0 | x: M 0.00 | y: M 0
2026-01-02  x: M 0.00 | x: M 0 | y: M 0.00
```

Lots (X is 2 EUR; `Q` has no price; the amounts of `b` are the elided ones). Alone, the EUR lot stays; with a converted X amount
it is lost, and also with a converted amount that is zero (the `X 0` of the third); `Q` keeps its lot; a converted X
amount with a lot is converted, with none:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-lots-merge.journal
2026-01-02  a: EUR 5 {USD 1} | b: EUR -5 {USD 1}
2026-01-02  a: EUR 5 {USD 1} | a: EUR 6 | b: EUR -11
2026-01-02  a: EUR 5 {USD 1} | a: EUR 6 | a: EUR -6 | b: EUR -5
2026-01-02  a: Q 5 {USD 1} | a: EUR 6 | b: EUR -6, Q -5 {USD 1}
2026-01-02  a: EUR 10 | b: EUR -10
2026-01-02  a: EUR 5 {USD 1} | a: EUR 2 | b: EUR -7
```

## Rule: what is valued

- Every amount of every posting: regular, virtual `(a)`, balanced virtual `[a]`. The date of the
  transaction or of the posting plays no part.
- The amounts are the amounts as `print` shows them: an elided amount is worked out
  **before** valuation (from the original amounts, with their costs, [balancing](balancing.md)), then valued
  like any other. An assertion or an assignment runs on the original amounts, and an assigned amount is
  valued afterwards. So the valued transaction need not balance. With `-I` (skip assertions) nothing
  changes.
- A posting with no amount at all (alone in its transaction) stays without.

### Probe

`probes/value-elided-and-assertions.journal`, `probes/value-virtual.journal`

### Command and output

X is 2 EUR, Y is 3 X, EUR is 1.1 USD. Plain `print` first, then valued in EUR and in X. In the first transaction
`b` is worked out as -12 EUR from the cost of `3 X @ 4 EUR`, so it stays -12 EUR while `a` is valued 6 EUR.
`c  5 X = 5 X` passes on the 5 X, not on 10 EUR; `c  = 8 X` is 3 X, valued 6 EUR; the `==` assignment makes `c`
`X -9, Y 3`, valued 0 EUR (-18 + 18) and kept as a zero:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/value-elided-and-assertions.journal
2026-01-02  a: X 3 @ EUR 4 | b: EUR -12
2026-01-02  c: X 5 | d: X -5
2026-01-03  c: X 3 | d: X -3
2026-01-04  c: X 1 | e: Y 3 | d: X -1, Y -3
2026-01-05  c: X -9, Y 3 | d: X 9, Y -3
2026-01-06  c:s: X 4 | c: X 8 | d: X -12
$ spec/hledger/show-amounts.sh --value=2026-01-07,EUR spec/hledger/probes/value-elided-and-assertions.journal
2026-01-02  a: EUR 6 | b: EUR -12
2026-01-02  c: EUR 10 | d: EUR -10
2026-01-03  c: EUR 6 | d: EUR -6
2026-01-04  c: EUR 2 | e: EUR 18 | d: EUR -20
2026-01-05  c: EUR 0 | d: EUR 0
2026-01-06  c:s: EUR 8 | c: EUR 16 | d: EUR -24
$ spec/hledger/show-amounts.sh --value=2026-01-07,X spec/hledger/probes/value-elided-and-assertions.journal
2026-01-02  a: X 3 @ EUR 4 | b: X -6
2026-01-02  c: X 5 | d: X -5
2026-01-03  c: X 3 | d: X -3
2026-01-04  c: X 1 | e: X 9 | d: X -10
2026-01-05  c: X 0 | d: X 0
2026-01-06  c:s: X 4 | c: X 8 | d: X -12
$ spec/hledger/show-amounts.sh -I --value=2026-01-07,EUR spec/hledger/probes/value-elided-and-assertions.journal
2026-01-02  a: EUR 6 | b: EUR -12
2026-01-02  c: EUR 10 | d: EUR -10
2026-01-03  c: EUR 6 | d: EUR -6
2026-01-04  c: EUR 2 | e: EUR 18 | d: EUR -20
2026-01-05  c: EUR 0 | d: EUR 0
2026-01-06  c:s: EUR 8 | c: EUR 16 | d: EUR -24
```

Virtual and balanced virtual postings:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-07,EUR spec/hledger/probes/value-virtual.journal
2026-01-02  a: EUR 2 | v: EUR 4 | bv: EUR 6 | bw: EUR -6 | b: EUR -2
```

## Gaps

- Whether the last product `Q × R` is capped at 255 places is unknown (see above); nothing observable depends on it.
- Amounts of more than about 2^53 in the mantissa are printed by hledger in scientific notation when the amount has no
  decimal places (`3.33E+259`), which the harness cannot compare; the fold probe has ten places and is printed whole.
