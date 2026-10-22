# Balance assignments: lots and prices

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

The amounts of an assignment ([assignments](assignments.md)) follow the account's running
balance, which has no costs and, at most, a lot.

**Costs of the postings.** When a posting is added to the balance its price is dropped: `3 X @ 2 EUR`,
`2 X @ 3 EUR` and `4 X` make one entry of 9 X. So `d = 10 X` gives plain `1 X`.

**A lot on the zeroing amounts.** The amounts that `==` and `==*` emit to bring the other
commodities to zero keep the lot (cost, date and label) of the account's balance in that commodity
when exactly one posting made that balance, and have no lot otherwise. Counted: every posting to
the account (to the account or a sub-account for `==*`) in that commodity, on any day, zero
amounts too; the same lot twice is two postings. A posting that has a lot and a price keeps
the lot and loses the price. `{{T}}` makes a per-unit lot of `T` over the quantity.

- Kept: one posting `2 X [2026-01-01] (lbl) {10 EUR}` (the whole lot, date and label); one
  `2 X {{30 EUR}}` (a lot of 15 EUR); `2 X {10 EUR} @ 12 EUR`; a lot posted on a sub-account for
  `==*`.
- Lost: `1 X {10 EUR}` twice; `1 X {10 EUR}` and `1 X`; `1 X {10 EUR}` and `0 X` (either order);
  `2 X {10 EUR}` and `3 X {11 EUR}`; the same lot in two transactions.
- The amount for the **asserted** commodity never has a lot: `= 1 X` on `2 X {10 EUR}` gives `-1 X`.
  After an assignment of a commodity (`=`, or `==` for the commodity asserted) its balance is the
  asserted amount, which has no lot, so a later zeroing of it has none either. A plain assertion
  leaves the balance as it was (and a posting `0 Y` beside a lot still counts as a second posting).
- The partner posting that takes the balancing amount (`z`) carries the same lot when exactly one
  amount contributed ([elided-amounts](elided-amounts.md)).

**A price in the asserted amount.** The assigned amount then has that price, and the
balance's old entries for the commodity are subtracted as they are, so the result can hold
several amounts of the same commodity. The difference is `B - (all entries of C)`, where the
entries and `B` are grouped like the amounts of [elided-amounts](elided-amounts.md): same
commodity, same kind of price, same price commodity and, for a unit price, the same price value;
units add up, and so do the totals of `@@` prices.

- The balance before an assignment with a price is plain (costs were dropped), so `3 X` and
  `= 3 X @ 2 EUR` give `-3 X` and `3 X @ 2 EUR`, not nothing. With no balance, or a zero one,
  only `3 X @ 2 EUR`.
- After it, the balance in X is exactly that priced entry. A plain posting added next is a
  separate, price-less entry, and the next assignment subtracts both:
  `X 3 @ 2 EUR` then `1 X` then `= 4 X` gives `3 X` and `-3 X @ 2 EUR`.
  A plain assertion adds all entries of the commodity, price or not, as always (line 65 of
  `assign-cost`: `k = 3 X @ 2 EUR`, `k 1 X`, `k 0 X = 4 X` holds).
- The same price twice makes one entry: `= 1 X @ 1 EUR` then `= 2 X @ 1 EUR` gives `1 X @ 1 EUR`.
  Totals add up: `1 X @@ 3 EUR` then `2 X @@ 1 EUR` gives `1 X @@ -2 EUR`.
- The amounts of the posting are listed by commodity, then costless first, then by price
  commodity, total price before unit price, then by price value. `= 1 X @ 1 USD` after
  `= 1 X @@ 9 EUR` gives `-1 X @@ -9 EUR` then `1 X @ 1 USD`; `= 1 X` after that gives `1 X` then
  `-1 X @ 1 USD`; `= 0 X` after that gives `-1 X`.
- The price takes part in balancing like a written one: the partner gets `-6 EUR` for
  `3 X @ 2 EUR`, and `+3 X` for the `-3 X` beside it.
- With `==`, the zeroed commodities come in the same list: `f` holding `2 Y`, `f == 3 X @ 2 EUR`
  gives `3 X @ 2 EUR` and `-2 Y`.

ledgermill may leave a price in the asserted amount of an assignment out of its first version
(`unsupported-syntax`), as nothing in the app's journals does it; the rule above is complete.

## Probe

`probes/assign-lots.journal`, `probes/assign-cost.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/assign-lots.journal
spec/hledger/show.sh spec/hledger/probes/assign-cost.journal
```

## Output

Only the transactions with assignments are listed (the setup is the first transaction of each
journal), as `account amounts` of `[commodity, mantissa, places]`, with `{cost, date, label}` for a
lot and `["@"|"@@", commodity, mantissa, places]` for a price. assign-lots:

```text
line 22 (l1: 1 X {10 EUR} twice; l2: a lot and a plain X; == 0 EUR):
  l1 [["X",-2,0]]   l2 [["X",-2,0]]   z [["X",4,0]]
line 32 (l6: 2 X [2026-01-01] (lbl) {10 EUR}; l7: 2 X {{30 EUR}}):
  l6 [["X",-2,0,{"cost":["EUR",10,0],"date":"2026-01-01","label":"lbl"}]]
  l7 [["X",-2,0,{"cost":["EUR",15,0],"date":null,"label":null}]]   z [["X",4,0]]
line 37 (l8: 2 X {10 EUR}, then = 1 X):   l8 [["X",-1,0]]   z [["X",1,0]]
line 41 (l9: 2 X {10 EUR} @ 12 EUR, == 0 EUR):
  l9 [["X",-2,0,{"cost":["EUR",10,0],"date":null,"label":null}]]
  z [["X",2,0,{"cost":["EUR",10,0],"date":null,"label":null}]]
line 45 (l10: 2 X {10 EUR} and 3 EUR, == 3 EUR):
  l10 [["X",-2,0,{"cost":["EUR",10,0],"date":null,"label":null}]]   (and z the same, positive)
line 58 (l12: 2 Y {10 EUR}, = 3 Y):   l12 [["Y",1,0]]   z [["Y",-1,0]]
line 62 (l12 == 1 EUR):                l12 [["Y",-3,0]]  z [["Y",3,0]]
line 66 (l13:s holds 2 Y {10 EUR}, l13 ==* 1 EUR):
  l13 [["Y",-2,0,{"cost":["EUR",10,0],"date":null,"label":null}]]
```

(The lines 27 and 54 are `l4`, `l5` with a `0 X` beside the lot, and `l11` with two lots: all
`[["X",-1,0]]` / `[["X",-5,0]]`, no lot.) assign-cost:

```text
line 13 (d: 3 X @ 2 EUR, 2 X @ 3 EUR, 4 X; d = 10 X):      d [["X",1,0]]
line 17 (a: 3 X; a = 3 X @ 2 EUR):
  a [["X",-3,0],["X",3,0,["@","EUR",2,0]]]   z [["EUR",-6,0],["X",3,0]]
line 21 (a = 5 X):          a [["X",5,0],["X",-3,0,["@","EUR",2,0]]]   z [["EUR",6,0],["X",-5,0]]
line 25 (b: 3 X; b = 3 X @@ 6 EUR):
  b [["X",-3,0],["X",3,0,["@@","EUR",6,0]]]   z [["EUR",-6,0],["X",3,0]]
line 29 (c: 1 X and -1 X; c = 3 X @ 2 EUR):   c [["X",3,0,["@","EUR",2,0]]]   z [["EUR",-6,0]]
line 37 (g after a priced assignment: g 1 X, then g = 4 X):
  g [["X",1,0]]   g [["X",3,0],["X",-3,0,["@","EUR",2,0]]]   z [["EUR",6,0],["X",-4,0]]
line 46 (h = 1 X @ 1 EUR, then h = 2 X @ 1 EUR):  h [["X",1,0,["@","EUR",1,0]]]  (twice)
line 51 (i = 1 X @@ 3 EUR, then i = 2 X @@ 1 EUR): i [["X",1,0,["@@","EUR",3,0]]]  i [["X",1,0,["@@","EUR",-2,0]]]
line 56 (j, six assignments in a row, each line one posting):
  [["X",1,0,["@","EUR",2,0]]]
  [["X",1,0,["@","EUR",1,0]],["X",-1,0,["@","EUR",2,0]]]
  [["X",1,0,["@@","EUR",9,0]],["X",-1,0,["@","EUR",1,0]]]
  [["X",-1,0,["@@","EUR",-9,0]],["X",1,0,["@","USD",1,0]]]
  [["X",1,0],["X",-1,0,["@","USD",1,0]]]
  [["X",-1,0]]
```

(`line 42`, `f == 3 X @ 2 EUR` with `2 Y` held: `f [["X",3,0,["@","EUR",2,0]],["Y",-2,0]]`,
`z [["EUR",-6,0],["Y",2,0]]`.) The lines are shortened for reading; `show.sh` prints each transaction
on one line.
