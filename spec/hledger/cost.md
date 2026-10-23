# `--cost`: costs converted, conversion postings dropped

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

`hledger print -O json --cost` is the engine call `Ledger::at_cost()`. It does two things to every
transaction: it replaces each amount that has a cost (`@`, `@@`, or a price hledger inferred when it
balanced the transaction) by an amount in the cost's commodity, and it drops some postings (the
"conversion postings"). What a conversion account is, and which postings balance by their quantity,
is in [conversion](conversion.md); this note does not repeat it. Arithmetic is that of
[value-arithmetic](value-arithmetic.md) (exact decimals, `round255`, ten places at the end).

Output lines come from `spec/hledger/show-amounts.sh` (`DATE  ACCOUNT: AMOUNT | ...`; an amount is a
commodity and the number with exactly its places; `@`, `@@` for a price, `{...}` for a lot).
`spec/hledger/show-tags.sh` shows the tags hledger puts on postings (below).

## Rule: the amount of a posting

Every posting is converted: regular, virtual `(a)` and balanced virtual `[a]` ones.

- An amount with **no cost** is left as it is: same places, **its lot kept** (a lot alone is not a cost).
- A **unit cost** `Q C @ P D` becomes `V D` with `V = round255(Q × P)`, trailing zeros stripped,
  then, if `V` has more than ten places, rounded half to even to ten places, keeping the ten places
  (so a non-zero value that rounds to zero is `0.0000000000`, and an exact zero is `0`). `P` is the price
  with all its places, and a **price hledger inferred** is the unrounded 255-place one (so nine times
  a ninth is `1.0000000000`, not `0.9999999999`).
- A **total cost** `Q C @@ T D` becomes `T D`, where `T` is the total as the journal reading stores it:
  trailing zeros stripped (`7.50` is `7.5`), the sign of the quantity (a zero quantity counts as positive:
  `0 Y @@ 5 EUR` is `5 EUR`, `2 Y @@ -4 EUR` is `-4 EUR`, `-2 Y @@ -4 EUR` is `4 EUR`). A total price
  hledger inferred keeps its places (`2.0`). The ten-place rounding above applies to it too.
- The result has **no cost and no lot**, even if the amount had a lot (`2 Y {4 EUR} @ 5 EUR` is `10 EUR`).
  The cost commodity may be the amount's own (`2 X @ 3 X` is `6 X`).
- An elided posting is worked out by balancing, which already converts the costs of the other postings
  ([elided-amounts](elided-amounts.md)); its amounts carry no cost, so `--cost` leaves them (a lot stays).

### Probe

`probes/cost-unit-and-total.journal`, `probes/cost-zero-and-rounding.journal`, `probes/cost-lots.journal`,
`probes/cost-inferred-price.journal`

### Command and output

Each journal is shown as `print` makes it, then with `--cost`. Unit and total costs, signs, a zero quantity, a
total with trailing zeros, a cost in the amount's own commodity; the last amount has 15 places in its price:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/cost-unit-and-total.journal
2026-01-01  a: X 3 @ EUR 2.50 | b: X -3 @ EUR 2.50 | c: Y 2 @@ EUR 7.5 | d: Y -2 @@ EUR -7.5 | e: Y 0 @@ EUR 5 | f: Y 0 @@ EUR 5 | g: Y 2 @@ EUR -4 | h: Y -2 @@ EUR 4 | i: Y 2 @ EUR -4 | j: Z 1.50 @@ EUR 3 | k: Z 1.5 @ EUR 2.0 | l: X 2 @ X 3 | m: X 1 @ EUR 0.1234567890
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-unit-and-total.journal
2026-01-01  a: EUR 7.5 | b: EUR -7.5 | c: EUR 7.5 | d: EUR -7.5 | e: EUR 5 | f: EUR 5 | g: EUR -4 | h: EUR 4 | i: EUR -8 | j: EUR 3 | k: EUR 3 | l: X 6 | m: EUR 0.1234567890
```

Zeros and rounding. A zero quantity, a zero price and a zero total give `0`; a result that rounds to zero is
`0.0000000000`; 1.5e-10 and 2.5e-10 both give 2e-10 (half to even), also as a total:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/cost-zero-and-rounding.journal
2026-01-01  a: Y 0.00 @ EUR 5 | b: Y 1.5 @ EUR 0 | c: Y 2 @@ EUR 0 | d: Z 0.0000000001 @ EUR 0.5 | e: Z -0.0000000003 @ EUR 0.5 | f: Z 0.0000000002 @ EUR 1 | g: Z 0.0000000002 @ EUR 1 | h: Z 1 @@ EUR 0.0000000002 | i: Z 1 @@ EUR 0.0000000002 | j: Z 100.0000 @ EUR 0.5
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-zero-and-rounding.journal
2026-01-01  a: EUR 0 | b: EUR 0 | c: EUR 0 | d: EUR 0.0000000000 | e: EUR -0.0000000002 | f: EUR 0.0000000002 | g: EUR 0.0000000002 | h: EUR 0.0000000002 | i: EUR 0.0000000002 | j: EUR 50
```

Lots. Only an amount with a cost is converted; `{4 EUR}` alone and `{{8 EUR}}` alone (a lot per unit of 4) stay as they are,
with the lot; a lot with a date and a label is dropped with the cost; `{}` too:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/cost-lots.journal
2026-01-01  a: Y 2 {EUR 4} | b: Y 2 @ EUR 5 {EUR 4} | c: Y 2 @@ EUR 20 {EUR 4} | d: Y 2 @ EUR 5 {EUR 4} | e: Y 2 {EUR 4} | f: Y 2 @ EUR 5 {} | g: Y 2 {EUR 4} | h: Y 2 @@ EUR 20 | i: Y 2 @@ EUR 20 {EUR 4} | j: Y 3
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-lots.journal
2026-01-01  a: Y 2 {EUR 4} | b: EUR 10 | c: EUR 20 | d: EUR 10 | e: Y 2 {EUR 4} | f: EUR 10 | g: Y 2 {EUR 4} | h: EUR 20 | i: EUR 20 | j: Y 3
```

Inferred prices (see [inferred-prices](inferred-prices.md)). In the first transaction `9 X, 0 X, -1 Y` gets a unit
price of 1/9, kept to 255 places inside; `--cost` gives `1.0000000000` and `0` (the zero quantity stays zero);
in the second 3/7 and 4/7 of a unit; an inferred total keeps its places (`2.0`); in the last the elided posting `c`
holds `-1 Y` with a total cost of `-3.0 Z`, and becomes `-3.0 Z`:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/cost-inferred-price.journal
2026-01-01  a: X 9 @ Y 0.1111111111 | b: X 0 @ Y 0.1111111111 | c: Y -1
2026-01-02  a: X 3 @ Y 0.1428571429 | b: X 4 @ Y 0.1428571429 | c: Y -1
2026-01-03  a: X 1 @@ Y 2.0 | c: Y -2.0
2026-01-04  a: X 1 @@ Y 1.0 | c: Y -1.5 | d: Y 0.5
2026-01-05  a: X 2 | c: Y -1 @@ Z -3.0 | d: X 1 | e: X -3.0 | e: Z 3.0
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-inferred-price.journal
2026-01-01  a: Y 1.0000000000 | b: Y 0 | c: Y -1
2026-01-02  a: Y 0.4285714286 | b: Y 0.5714285714 | c: Y -1
2026-01-03  a: Y 2.0 | c: Y -2.0
2026-01-04  a: Y 1.0 | c: Y -1.5 | d: Y 0.5
2026-01-05  a: X 2 | c: Z -3.0 | d: X 1 | e: X -3.0 | e: Z 3.0
```

## Rule: a posting with several amounts

A posting holds several amounts when its amount is elided or assigned (`==`, `=`; see
[assignments](assignments.md)). After the amounts are converted they are **merged per commodity** exactly as in
[value-arithmetic](value-arithmetic.md): the amounts that end in the same commodity are added in the order the posting
lists them (a zero gives the other amount back with its places), the sum is rounded to ten places when it has more,
the list is ordered by commodity name (`""` first), and an amount that was not converted keeps its lot only when it
is alone in its commodity. A posting with one amount is not touched by this.

### Probe

`probes/assign-cost.journal` (the assigned amounts keep the price written after the asserted amount).

### Command and output

On 01-03 `a` holds `X -3` and `X 3 @ 2 EUR` (the old balance and the assigned amount), which become `EUR 6` and `X -3`, in
name order; on 01-12 `j` holds `X 1 @@ 9 EUR` with `X -1 @ 1 EUR`, which add up to `EUR 8`, and `X -1 @@ -9 EUR` with
`X 1 @ 1 USD`, which stay apart as `EUR -9, USD 1`:

```text
$ spec/hledger/show-amounts.sh spec/hledger/probes/assign-cost.journal
2026-01-03  a: X -3, X 3 @ EUR 2 | z: EUR -6, X 3
2026-01-12  j: X 1 @ EUR 2 | j: X 1 @ EUR 1, X -1 @ EUR 2 | j: X 1 @@ EUR 9, X -1 @ EUR 1 | j: X -1 @@ EUR -9, X 1 @ USD 1 | j: X 1, X -1 @ USD 1 | j: X -1 | z: EUR 0, USD 0, X 0
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/assign-cost.journal
2026-01-03  a: EUR 6, X -3 | z: EUR -6, X 3
2026-01-12  j: EUR 2 | j: EUR -1 | j: EUR 8 | j: EUR -9, USD 1 | j: USD -1, X 1 | j: X -1 | z: EUR 0, USD 0, X 0
```

(Only these two lines of the probe's output are shown; the whole file, and `assign-entry-prices.journal`, are compared by the
harness.)

## Rule: the postings that are dropped

A transaction is never dropped: one with every posting dropped stays, with no postings.

`--cost` drops each posting that carries the tag `_conversion-posting` and has no cost. The tag
comes from two places.

### The tag written in a comment

A posting whose own comment (the text after `;` on its line, or a following indented line that starts with `;`) holds the tag
`_conversion-posting:` with any value, or none, is dropped unless it has a cost. Several tags are
separated by commas; the name is case sensitive and must be a whole tag (`x_conversion-posting:` is another tag).
A tag in the comment of the transaction header, or in an indented comment before the first posting, does nothing. It
works on any posting: regular, virtual, elided, on a conversion account or not. A posting that has the tag and
a cost is kept (and converted).

#### Probe

`probes/cost-drop-tags.journal`

#### Command and output

```text
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-drop-tags.journal
2026-01-01  a: EUR 3
2026-01-02  a: EUR 3
2026-01-03  a: EUR 3
2026-01-04  a: EUR 3 | b: EUR -3
2026-01-05  a: EUR 3 | b: EUR -3
2026-01-06  a: EUR 3 | b: EUR -2 | c: EUR -1
2026-01-07  a: EUR 3 | b: EUR -3
2026-01-08  a: EUR 3
2026-01-09  
2026-01-10  a: EUR 3 | b: EUR -3 | c: EUR -5
2026-01-11  a: EUR 3
```

### The tag hledger adds

hledger tags postings by itself when a transaction holds conversion postings. In `print -O json` the tags are
in `ptags`, and `show-tags.sh` shows them as `[conversion]` (`_conversion-posting`) and `[cost]` (`_cost-posting`,
which marks the posting hledger took as the one converted; it plays no part in `--cost`). The rule, with `A`
the `[conversion]` postings it produces:

1. **Pairs.** Take the postings of the transaction in order, virtual ones included, and pair the conversion
   postings left to right: two adjacent conversion-account postings are a pair, and the next pair starts after them
   (a third in a row is alone, a fourth pairs with a fifth; `conv, a, conv, conv` pairs only the last two). A conversion
   posting that is in no pair is never tagged. If there is no pair, nothing is tagged.
2. If a **priced** posting (an explicit `@` or `@@`) is preceded, right before it, by an odd number of conversion postings
   that open the transaction, and it does **not** balance by its quantity ([conversion](conversion.md): a posting repeating
   its quantity makes it balance by cost), then **nothing is tagged**, whatever else the transaction holds.
3. Else if some priced posting **balances by its quantity** ([conversion](conversion.md), both the odd opening run and the even run
   rules), **all pairs are tagged**.
4. Else the **anchor** is the first posting that is not a conversion posting and has no cost (regular, virtual or balanced virtual, an elided
   one too; a lot is not a cost). If there is none, nothing is tagged. The pairs are tagged if no other
   such posting **repeats** the anchor. For this each posting is a number: its written quantity (the commodity does not
   matter, the sign does), and for a posting with no amount it is zero with no places. Two postings repeat each other when
   their numbers differ by at most half a unit at **the anchor's** places (at 0 places, 3 and 3.5 repeat, 3 and 3.6 do not; at 1
   place, 3.0 and 3.05 repeat, 3.0 and 3.06 do not). Conversion postings and postings of other transactions are not looked at,
   and **priced postings are not** either, with one exception: a priced posting that is preceded, right before it, by an odd
   number of conversion postings that do **not** open the transaction (rule 2 covers the ones that do) is one more number
   that can repeat the anchor, its written quantity compared at the anchor's places like any other. So a lone `equity:conversion`
   posting right before the priced one, behind a regular posting, keeps every pair when the priced quantity is the anchor's
   (`-11.85 USD` and `-11.85 AB @ 0.3 EUR`), and the amount of that conversion posting plays no part. A conversion posting
   anywhere else (after the priced posting, or with another posting between) leaves the priced posting out as before.

Then every pair posting is tagged, and `--cost` drops it. So a conversion pair is kept when the transaction
has nothing but conversion postings, when its only other posting is priced and balances by cost, and when the
first free posting is a number that another repeats.

This rule was checked against hledger on about 36,000 transactions: every arrangement of two to six postings
of ±1 and ±2 EUR on conversion and other accounts (31,360, rule 4); 2,093 random transactions of two to seven
postings in EUR and USD with 0 to 2 places, virtual and elided postings, equal and near-equal numbers (rule 4,
no difference); 992 and about 2,100 shapes with one priced posting and conversion postings of the amounts
±2 X, ±6, ±1 EUR (rules 2 to 4; the odd opening runs of one and three postings). Throwaway scripts, not kept.

#### Probe

`probes/cost-drop-pairs.journal`, `probes/cost-drop-anchor.journal`, `probes/cost-drop-priced.journal`,
`probes/cost-pair-odd-run.journal`; and the
probes of [conversion](conversion.md), `conversion.journal` and `conversion-leading-run.journal`.

#### Command and output

Pairs. Alone, one conversion posting stays; two go; of three the third stays; four go; of five the fifth stays;
non-adjacent ones stay; the account names may differ; virtual pairs go (also a virtual with a regular posting); a transaction of
conversion postings only is kept; one whose only other posting is virtual loses its pair (the virtual posting is the anchor):

```text
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-drop-pairs.journal
2026-01-01  a: EUR 3 | equity:conversion: EUR -3
2026-01-02  a: EUR 3
2026-01-03  a: EUR 6 | equity:conversion: EUR -3
2026-01-04  a: EUR 10
2026-01-05  a: EUR 15 | equity:conversion: EUR -5
2026-01-06  equity:conversion: EUR -1 | a: EUR 10
2026-01-07  equity:conversion: EUR 3 | a: EUR -6 | equity:conversion: EUR 3
2026-01-08  equity:trade: EUR -3 | a: EUR 6
2026-01-09  a: EUR 3 | b: EUR -3
2026-01-10  a: EUR 1 | b: EUR -1
2026-01-11  a: EUR 3 | b: EUR -1
2026-01-12  equity:conversion: X 2 | equity:conversion: X -2
2026-01-13  equity:conversion: EUR 1 | equity:conversion: EUR 2 | equity:conversion: EUR -3
2026-01-14  v: EUR 5
```

The tags behind them (the pairs are `[conversion]`; a lone conversion posting is not):

```text
$ spec/hledger/show-tags.sh spec/hledger/probes/cost-drop-pairs.journal
2026-01-01  a 3 EUR | equity:conversion -3 EUR
2026-01-02  a 3 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion -2 EUR [conversion]
2026-01-03  a 6 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion -2 EUR [conversion] | equity:conversion -3 EUR
2026-01-04  a 10 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion -2 EUR [conversion] | equity:conversion -3 EUR [conversion] | equity:conversion -4 EUR [conversion]
2026-01-05  a 15 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion -2 EUR [conversion] | equity:conversion -3 EUR [conversion] | equity:conversion -4 EUR [conversion] | equity:conversion -5 EUR
2026-01-06  equity:conversion -1 EUR | a 10 EUR [cost] | equity:conversion -2 EUR [conversion] | equity:conversion -7 EUR [conversion]
2026-01-07  equity:conversion 3 EUR | a -6 EUR | equity:conversion 3 EUR
2026-01-08  equity:conversion -1 EUR [conversion] | Equity:Trading:x -2 EUR [conversion] | equity:trade -3 EUR | a 6 EUR [cost]
2026-01-09  a 3 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion -2 EUR [conversion] | b -3 EUR
2026-01-10  a 1 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion 1 EUR [conversion] | b -1 EUR
2026-01-11  a 3 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion -2 EUR [conversion] | b -1 EUR
2026-01-12  equity:conversion 2 X | equity:conversion -2 X
2026-01-13  equity:conversion 1 EUR | equity:conversion 2 EUR | equity:conversion -3 EUR
2026-01-14  equity:conversion 1 EUR [conversion] | equity:conversion -1 EUR [conversion] | v 5 EUR [cost]
```

The anchor. Each transaction below holds the pair `-1 EUR`, `1 EUR`; the title says what the anchor and the others are.
Where the output shows no conversion posting, the pair went. For example 02-02 keeps it (`a` 3 is repeated by `b` 3), 02-03 keeps it
(`3 USD` repeats `3 EUR`), 02-06 drops it (3.6 is more than half a unit from 3), 02-07 drops it (at one place 3.0 and
3.4 differ), 02-20 drops it (the elided `z` is zero, 2 is not near), 02-21 keeps it (0.4 is near zero at no places):

```text
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-drop-anchor.journal
2026-02-01  a: EUR 3 | b: EUR -3
2026-02-02  a: EUR 3 | b: EUR 3 | c: EUR -6 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-03  a: EUR 3 | b: USD 3 | c: USD -3 | d: EUR -3 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-04  a: EUR 3 | b: EUR 3.4 | c: EUR -6.4 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-05  a: EUR 3 | b: EUR 3.5 | c: EUR -6.5 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-06  a: EUR 3 | b: EUR 3.6 | c: EUR -6.6
2026-02-07  a: EUR 3.0 | b: EUR 3.4 | c: EUR -6.4
2026-02-08  a: EUR 3.0 | b: EUR 3.05 | c: EUR -6.05 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-09  a: EUR 3.0 | b: EUR 3.06 | c: EUR -6.06
2026-02-10  b: EUR 3.05 | a: EUR 3.0 | c: EUR -6.05
2026-02-11  a: EUR 3 | b: EUR 4 | c: EUR 4 | d: EUR -11
2026-02-12  a: EUR 3 | b: EUR 5 | c: EUR 3 | d: EUR -11 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-13  a: EUR 3 | b: EUR -3 | c: EUR -3 | d: EUR 3 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-14  a: EUR 3 | v: EUR 3 | b: EUR -3 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-15  v: EUR 3 | a: EUR 4 | b: EUR -4
2026-02-16  v: EUR 3 | w: EUR -3 | a: EUR 4 | b: EUR -4
2026-02-17  a: EUR 3 | b: EUR 3 | c: EUR -6
2026-02-18  p: EUR 6 | b: EUR -6
2026-02-19  a: X 3 {EUR 2} | b: X 3 {EUR 3} | c: X -6 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-20  z: EUR 2 | a: EUR 2 | b: EUR -4
2026-02-21  z: EUR 0 | a: EUR 0.4 | b: EUR -0.4 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-22  z: EUR 0 | a: EUR 0.6 | b: EUR -0.6
2026-02-23  a: EUR 0.4 | b: EUR -0.4 | z: EUR 0
2026-02-24  a: EUR 0 | b: EUR 5 | c: EUR -5 | z: EUR 0 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-25  a: EUR 0 | b: USD 0 | c: EUR 1 | d: EUR -1 | equity:conversion: EUR -1 | equity:conversion: EUR 1
2026-02-26  a: EUR 0 | c: EUR 1 | d: EUR -1
2026-02-27  a: EUR 3 | b: EUR -3
```

Priced postings (see [conversion](conversion.md)). 01-01 and 01-04: the priced posting balances by quantity, the pair goes. 01-02: it balances by cost,
the anchor is `b`, unique, the pair goes. 01-03: the only other postings are conversion ones, nothing to anchor, the pair stays.
01-05 and 01-06: an odd run of conversion postings opens the transaction and `c` repeats the quantity, so nothing is tagged
although `b` is unique. 01-07: the lone conversion posting does not open the transaction, so rule 4 applies, and the pair goes:

```text
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-drop-priced.journal
2026-01-01  a: EUR 6 | b: EUR -6
2026-01-02  a: EUR 6 | b: EUR -6
2026-01-03  a: EUR 2 | equity:trading: EUR 4 | equity:trade: EUR -6
2026-01-04  equity:conversion: X -3 | a: EUR 6 | b: EUR -2
2026-01-05  equity:conversion: X -3 | a: EUR 6 | equity:conversion: EUR 1 | equity:conversion: EUR 1 | b: EUR -8 | c: X 3
2026-01-06  equity:conversion: EUR 1 | equity:conversion: EUR 1 | equity:conversion: X -3 | a: EUR 6 | b: EUR -8 | c: X 3
2026-01-07  b: EUR -12 | equity:conversion: X -3 | a: EUR 6 | c: X 6
$ spec/hledger/show-tags.sh spec/hledger/probes/cost-drop-priced.journal
2026-01-01  a 2 X @ 3 EUR [cost] | equity:conversion -2 X [conversion] | equity:conversion 6 EUR [conversion] | b -6 EUR
2026-01-02  a 2 X @ 3 EUR | b -6 EUR [cost] | equity:conversion -1 EUR [conversion] | equity:conversion 1 EUR [conversion]
2026-01-03  a 2 X @ 1 EUR | equity:trading 4 EUR | equity:trade -6 EUR
2026-01-04  equity:conversion -3 X | a 3 X @ 2 EUR [cost] | equity:conversion 1 EUR [conversion] | equity:conversion 1 EUR [conversion] | b -2 EUR
2026-01-05  equity:conversion -3 X | a 3 X @ 2 EUR | equity:conversion 1 EUR | equity:conversion 1 EUR | b -8 EUR | c 3 X
2026-01-06  equity:conversion 1 EUR | equity:conversion 1 EUR | equity:conversion -3 X | a 3 X @ 2 EUR | b -8 EUR | c 3 X
2026-01-07  b -12 EUR [cost] | equity:conversion -3 X | a 3 X @@ 6 EUR | equity:conversion -3 X [conversion] | equity:conversion 6 EUR [conversion] | c 6 X
```

A priced posting behind an odd run that does not open the transaction (`probes/cost-pair-odd-run.journal`; `z` is elided, and
every transaction has a conversion posting `-3 EUR` right before `p`, and the pair `-1989 EUR`, `359 AB` after it). 01-01: `p`
repeats the anchor `a`, nothing is tagged. 01-02, 01-03: another quantity, the opposite sign: the pair goes. 01-04: `-11.845`
is within half a unit of `-11.85` at the anchor's two places, nothing is tagged; 01-05: `-11.8` is not, the pair goes; 01-06: the
anchor `-11.8` has one place, `-11.85` is within half a unit of it, nothing is tagged. 01-07: a posting between the lone conversion
posting and `p` and 01-12 (conversion posting after `p`): the usual rule, the pairs go. 01-08: a run of three is odd, nothing is
tagged; 01-09: a run of two is even, the pairs go. 01-10: `p` repeats `c`, not the anchor: the pair goes. 01-11: no conversion
posting before `p`: the pair goes. 01-13: a transaction of zero quantities (`0.00 AB` anchor, `0.00 "A B" @ 7 EUR`) behind one
conversion posting: nothing is tagged.

```text
$ spec/hledger/show-tags.sh spec/hledger/probes/cost-pair-odd-run.journal
2026-01-01  a -11.85 USD | equity:conversion -3 EUR | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR | equity:conversion 359 AB | z -359 AB,1995.555 EUR,11.85 USD
2026-01-02  a -11.85 USD [cost] | equity:conversion -3 EUR | p -7 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | z -359 AB,1994.1 EUR,11.85 USD
2026-01-03  a -11.85 USD [cost] | equity:conversion -3 EUR | p 11.85 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | z -359 AB,1988.445 EUR,11.85 USD
2026-01-04  a -11.85 USD | equity:conversion -3 EUR | p -11.845 AB @ 0.3 EUR | equity:trade -1989 EUR | equity:conversion 359 AB | z -359 AB,1995.5535 EUR,11.85 USD
2026-01-05  a -11.85 USD [cost] | equity:conversion -3 EUR | p -11.8 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | z -359 AB,1995.54 EUR,11.85 USD
2026-01-06  a -11.8 USD | equity:conversion -3 EUR | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR | equity:conversion 359 AB | z -359 AB,1995.555 EUR,11.8 USD
2026-01-07  a -11.85 USD [cost] | equity:conversion -3 EUR | c 5 USD | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | z -359 AB,1995.555 EUR,6.85 USD
2026-01-08  a -11.85 USD | equity:conversion -3 EUR | equity:conversion -3 EUR | equity:conversion -3 EUR | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR | equity:conversion 359 AB | z -359 AB,2001.555 EUR,11.85 USD
2026-01-09  a -11.85 USD [cost] | equity:conversion -3 EUR [conversion] | equity:conversion -3 EUR [conversion] | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | z -359 AB,1998.555 EUR,11.85 USD
2026-01-10  a -7 USD [cost] | equity:conversion -3 EUR | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | c -11.85 X | z -359 AB,1995.555 EUR,7 USD,11.85 X
2026-01-11  a -11.85 USD [cost] | p -11.85 AB @ 0.3 EUR | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB [conversion] | z -359 AB,1992.555 EUR,11.85 USD
2026-01-12  a -11.85 USD [cost] | p -11.85 AB @ 0.3 EUR | equity:conversion -3 EUR [conversion] | equity:trade -1989 EUR [conversion] | equity:conversion 359 AB | z -359 AB,1995.555 EUR,11.85 USD
2026-01-13  c:d 0.00 AB | equity:trading:x 0.00 A B | a0 0.00 A B @ 7 EUR | equity:trade -1882 EUR | equity:trade 0.00 A B | equity:trade 0.002 EUR | b 1881.998 EUR
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/cost-pair-odd-run.journal
2026-01-01  a: USD -11.85 | equity:conversion: EUR -3 | p: EUR -3.555 | equity:trade: EUR -1989 | equity:conversion: AB 359 | z: AB -359, EUR 1995.555, USD 11.85
2026-01-02  a: USD -11.85 | equity:conversion: EUR -3 | p: EUR -2.1 | z: AB -359, EUR 1994.1, USD 11.85
2026-01-03  a: USD -11.85 | equity:conversion: EUR -3 | p: EUR 3.555 | z: AB -359, EUR 1988.445, USD 11.85
2026-01-04  a: USD -11.85 | equity:conversion: EUR -3 | p: EUR -3.5535 | equity:trade: EUR -1989 | equity:conversion: AB 359 | z: AB -359, EUR 1995.5535, USD 11.85
2026-01-05  a: USD -11.85 | equity:conversion: EUR -3 | p: EUR -3.54 | z: AB -359, EUR 1995.54, USD 11.85
2026-01-06  a: USD -11.8 | equity:conversion: EUR -3 | p: EUR -3.555 | equity:trade: EUR -1989 | equity:conversion: AB 359 | z: AB -359, EUR 1995.555, USD 11.8
2026-01-07  a: USD -11.85 | equity:conversion: EUR -3 | c: USD 5 | p: EUR -3.555 | z: AB -359, EUR 1995.555, USD 6.85
2026-01-08  a: USD -11.85 | equity:conversion: EUR -3 | equity:conversion: EUR -3 | equity:conversion: EUR -3 | p: EUR -3.555 | equity:trade: EUR -1989 | equity:conversion: AB 359 | z: AB -359, EUR 2001.555, USD 11.85
2026-01-09  a: USD -11.85 | p: EUR -3.555 | z: AB -359, EUR 1998.555, USD 11.85
2026-01-10  a: USD -7 | equity:conversion: EUR -3 | p: EUR -3.555 | c: X -11.85 | z: AB -359, EUR 1995.555, USD 7, X 11.85
2026-01-11  a: USD -11.85 | p: EUR -3.555 | z: AB -359, EUR 1992.555, USD 11.85
2026-01-12  a: USD -11.85 | p: EUR -3.555 | equity:conversion: AB 359 | z: AB -359, EUR 1995.555, USD 11.85
2026-01-13  c:d: AB 0.00 | equity:trading:x: A B 0.00 | a0: EUR 0 | equity:trade: EUR -1882 | equity:trade: A B 0.00 | equity:trade: EUR 0.002 | b: EUR 1881.998
```

The same with the probes of the conversion note:

```text
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/conversion.journal
2026-01-01  a: EUR 6 | b: EUR -6
2026-01-02  a: EUR 6 | c: Y -6 | d: Z 2 | e: X -2
2026-01-03  a: EUR 6 | b: EUR -6
2026-01-04  a: EUR 2 | equity:trading: EUR 4 | equity:trade: EUR -6
$ spec/hledger/show-amounts.sh --cost spec/hledger/probes/conversion-leading-run.journal
2026-01-01  equity:conversion: X -3 | a: EUR 6 | b: EUR -2
2026-01-02  equity:conversion: X -3 | a: EUR 6 | b: EUR -2
2026-01-03  equity:conversion: X -3 | a: EUR 6 | equity:conversion: EUR 1 | equity:conversion: EUR 1 | b: EUR -8 | c: X 3
2026-01-04  equity:conversion: X -3.0 | a: EUR 6 | equity:conversion: EUR 1 | equity:conversion: EUR 1 | b: EUR -8 | c: X 3 | d: Y 3.05 | d: Y -3.05
2026-01-05  equity:conversion: X -3.0 | a: EUR 6 | b: EUR -2 | d: Y 3.051 | d: Y -3.051
2026-01-06  b: EUR -12 | equity:conversion: X -3 | a: EUR 6 | c: X 6
```

## Gaps

- **Several priced postings** with conversion postings: `conversion.md` refuses the ones it cannot settle
  (`conversion-postings`); the tag rule was only checked with one priced posting. With one posting priced by quantity and
  another by cost, rules 2 and 3 are not ordered by any probe. The engine should refuse `at_cost()` for a transaction with
  conversion postings and several priced postings, as it does for the balancing.
- **A priced posting on a conversion account**, and a priced posting after a pair: refused in `conversion.md`, refused here.
- **Why rule 4 and rule 2 are as they are** is not known (the odd opening run being a silent exception looks like a
  consequence of how hledger matches a priced posting to the conversion postings before it). The rules fit all the
  shapes tried; a shape outside them (balanced virtual postings with an elided amount, three or more commodities
  with several elided postings, declared account types mixed with names) was not tried at scale.
- `--cost` combined with `--value`, `-B`/`-V`/`-X`, and `--cost` on `print` options other than JSON are not covered.
