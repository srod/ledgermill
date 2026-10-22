# Valuation: prices, paths and inverse rates

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

This note covers `hledger print -O json --value=DATE,COMM` up to the choice of the rate:
which `P` lines count on DATE, which path of commodities is followed, and what an inverse rate is.
[value-arithmetic](value-arithmetic.md) covers the products, the rounding and the amounts
themselves; [cost](cost.md) covers `--cost`. The engine call is `Ledger::value(date, commodity)`.

The output lines below are made by `spec/hledger/show-amounts.sh`: `DATE  ACCOUNT: AMOUNT | ...`,
where an amount is a commodity and the number with exactly its decimal places. Every probe
uses virtual postings (`(a)`), which need not balance, so one transaction can hold any amounts.
The date of a transaction plays no part in valuation: only DATE does.

## Rule: the command

`--value=DATE,COMM` values every amount of every posting, at the prices known on DATE, in the
commodity COMM. COMM is the text after the first comma, taken as it is: `E F` names the commodity
written `"E F"` in the journal, and a quote on the command line is part of the name. DATE is read as
`YYYY-MM-DD` (hledger also takes `YYYY/MM/DD` and `YYYYMMDD`, and refuses `2026-1-9` and impossible
dates). A call with no commodity (`--value=DATE` or `--value=DATE,`) and the words `now`, `end`,
`then` are other modes; they are not covered here (see Gaps).

## Rule: which `P` lines exist on DATE

Prices are global: every `P` line of every file counts, wherever it is and whichever transaction
it sits near. For each ordered pair (from-commodity, to-commodity), `P DATE X PRICE EUR` being the pair
(X, EUR):

1. keep the lines dated on or before DATE (a line dated DATE itself counts, a later one is ignored);
2. among those, the one with the latest date wins, whatever its place in the file;
3. on the same date, the line that comes later in parse order wins. Parse order is the files in the
   order given, each from top to bottom;
4. a time of day (`10:00`, `09:00:30`) is read and ignored, so it plays no part in 2 or 3.

A pair has no price when every line is dated after DATE; the amount is then not converted by that edge.
The two directions are separate pairs: `P X 2 EUR` and `P EUR 0.25 X` do not replace one another (below).
A price of zero is a price (its inverse is 0); a negative price is a price. `P X 2 X` (a commodity in itself)
changes nothing. A price keeps all its decimal places (up to 255); only a result is rounded.
A price without a commodity is in the commodity of the `D` directive, or in the empty commodity
`""` when there is none; the empty commodity is a commodity like another, and an amount without
commodity is an amount of `""`.

### Probe

`probes/value-p-latest.journal`, `probes/value-p-time.journal`, `probes/value-p-files-a.journal`
and `probes/value-p-files-b.journal`, `probes/value-p-pair-direction.journal`,
`probes/value-p-zero-negative-self.journal`, `probes/value-p-no-commodity.journal`,
`probes/value-p-default-commodity.journal`, `probes/value-p-precision.journal`,
`probes/value-p-quoted.journal`

### Command and output

value-p-latest (prices: X 2 on 01-01, 3 then 4 on 01-05, 5 on 01-09, and 7 on 01-03 written last;
before 01-01 there is none):

```text
$ spec/hledger/show-amounts.sh --value=2025-12-31,EUR spec/hledger/probes/value-p-latest.journal
2026-02-01  a: X 1
$ spec/hledger/show-amounts.sh --value=2026-01-01,EUR spec/hledger/probes/value-p-latest.journal
2026-02-01  a: EUR 2
$ spec/hledger/show-amounts.sh --value=2026-01-04,EUR spec/hledger/probes/value-p-latest.journal
2026-02-01  a: EUR 7
$ spec/hledger/show-amounts.sh --value=2026-01-05,EUR spec/hledger/probes/value-p-latest.journal
2026-02-01  a: EUR 4
$ spec/hledger/show-amounts.sh --value=2026-01-08,EUR spec/hledger/probes/value-p-latest.journal
2026-02-01  a: EUR 4
$ spec/hledger/show-amounts.sh --value=2026-01-09,EUR spec/hledger/probes/value-p-latest.journal
2026-02-01  a: EUR 5
```

On 01-04 the line dated 01-03 (written last) wins over the lines dated 01-01; on 01-05 the second
line of that date wins; the line dated 01-09 is ignored before 01-09 and used on it.

value-p-time (01-05: `10:00` 3, then `09:00:30` 4; 01-06: `08:00` 6, then no time 7):

```text
$ spec/hledger/show-amounts.sh --value=2026-01-05,EUR spec/hledger/probes/value-p-time.journal
2026-01-01  a: EUR 4
$ spec/hledger/show-amounts.sh --value=2026-01-06,EUR spec/hledger/probes/value-p-time.journal
2026-01-01  a: EUR 7
```

Several files (the first file has the transaction and X 3 on 01-05, X 1 on 01-03; the second only
`P` lines: X 4 on 01-05, X 8 on 01-04):

```text
$ spec/hledger/show-amounts.sh --value=2026-01-04,EUR spec/hledger/probes/value-p-files-a.journal spec/hledger/probes/value-p-files-b.journal
2026-01-02  a: EUR 8
$ spec/hledger/show-amounts.sh --value=2026-01-04,EUR spec/hledger/probes/value-p-files-b.journal spec/hledger/probes/value-p-files-a.journal
2026-01-02  a: EUR 8
$ spec/hledger/show-amounts.sh --value=2026-01-05,EUR spec/hledger/probes/value-p-files-a.journal spec/hledger/probes/value-p-files-b.journal
2026-01-02  a: EUR 4
$ spec/hledger/show-amounts.sh --value=2026-01-05,EUR spec/hledger/probes/value-p-files-b.journal spec/hledger/probes/value-p-files-a.journal
2026-01-02  a: EUR 3
```

A file with no transaction still gives its prices; the date decides first (8 on 01-04 in both orders)
and the file order only breaks a tie (4 or 3 on 01-05).

Directions are separate (X to EUR 2 on 01-01, EUR to X 0.25 on 01-02, X to EUR 3 on 01-03). On 01-01 only
the first exists, so EUR is valued in X by its inverse 0.5; from 01-02 the forward edge EUR to X is used
in preference to the inverse of the other direction:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-01,X spec/hledger/probes/value-p-pair-direction.journal
2026-01-04  a: X 1 | b: X 0.5
$ spec/hledger/show-amounts.sh --value=2026-01-02,X spec/hledger/probes/value-p-pair-direction.journal
2026-01-04  a: X 1 | b: X 0.25
$ spec/hledger/show-amounts.sh --value=2026-01-03,X spec/hledger/probes/value-p-pair-direction.journal
2026-01-04  a: X 1 | b: X 0.25
$ spec/hledger/show-amounts.sh --value=2026-01-03,EUR spec/hledger/probes/value-p-pair-direction.journal
2026-01-04  a: EUR 3 | b: EUR 1
```

Zero, negative and self prices (X is 0 EUR, Y is -2 EUR, `P Z 2 Z`, V is 4 EUR; one amount of 5 in each
of X, Y, Z, EUR, V). Valued in EUR, X gives 0, Y gives -10 and Z is not converted. Valued in X, EUR gives 0 (the
inverse of 0 is 0, and so do V and Y behind it); valued in Z nothing is converted, as no path leads to Z:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-p-zero-negative-self.journal
2026-01-02  a: EUR 0 | a: EUR -10 | a: Z 5 | a: EUR 5 | a: EUR 20
$ spec/hledger/show-amounts.sh --value=2026-01-02,X spec/hledger/probes/value-p-zero-negative-self.journal
2026-01-02  a: X 5 | a: X 0 | a: Z 5 | a: X 0 | a: X 0
$ spec/hledger/show-amounts.sh --value=2026-01-02,Y spec/hledger/probes/value-p-zero-negative-self.journal
2026-01-02  a: Y 0 | a: Y 5 | a: Z 5 | a: Y -2.5 | a: Y -10
$ spec/hledger/show-amounts.sh --value=2026-01-02,Z spec/hledger/probes/value-p-zero-negative-self.journal
2026-01-02  a: X 5 | a: Y 5 | a: Z 5 | a: EUR 5 | a: V 5
```

An unpriced commodity (`P X 2`, no `D`): the price is in `""`. A bare 3 valued in X gives 1.5 X, the inverse;
an amount already in X stays. With `D 1000.00 EUR` the price is in EUR:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,X spec/hledger/probes/value-p-no-commodity.journal
2026-01-02  a: X 1.5 | b: X 3
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-p-default-commodity.journal
2026-01-02  a: EUR 6 | b: EUR 1
```

A long price is kept whole (0.123456789012345678901234567890 EUR per X); the result is rounded to ten places,
and a non-zero amount that rounds to zero keeps its ten places (`0.0000000000`):

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,EUR spec/hledger/probes/value-p-precision.journal
2026-01-02  a: EUR 0.1234567890 | b: EUR 123456.7890123457 | c: EUR 0.0000000000
```

A quoted commodity (`"E F"`) is named without quotes. Valued in `E F`, X gives 2; valued in `"E F"`, with
the quotes, nothing is found; valued in `€`, the price of `E F` in `€` is used:

```text
$ spec/hledger/show-amounts.sh '--value=2026-01-02,E F' spec/hledger/probes/value-p-quoted.journal
2026-01-02  a: E F 2 | b: E F 1
$ spec/hledger/show-amounts.sh '--value=2026-01-02,€' spec/hledger/probes/value-p-quoted.journal
2026-01-02  a: € 6 | b: € 3
```

## Rule: the choice of the path

An amount in commodity C is valued in COMM by a path of prices from C to COMM, then multiplied by the
product of the rates along it (see [value-arithmetic](value-arithmetic.md)). If C is COMM, nothing is done.
The prices on DATE form a graph: each pair (from, to) with a price is a **forward edge** from `from` to `to`,
with rate the price, and also an **inverse edge** from `to` to `from`, with rate 1/price
(see below). A price of a commodity in itself is no edge.

1. First look for a path of **forward edges only**. The shortest one wins. Among shortest paths, the one
   whose commodity names are smallest wins, compared from the start, name by name, in code point order
   (so `Z` before `a`, `B` before `b`; never the locale's order). Any forward path, however long, beats
   every path that uses an inverse edge, even a direct one, even one made of newer prices.
2. Only if there is no forward path: look for a path of forward and inverse edges. The shortest wins.
   Equal lengths are decided by a breadth-first search: from each commodity, its **forward neighbours,
   sorted by name, are tried first, then its inverse neighbours, sorted by name**; the first path found
   to COMM is the one (so the forward step wins over an inverse step, whatever the names, and then the
   smaller name wins). When the same neighbour is both forward and inverse, the forward edge is the one used.
3. Cycles do not matter: a commodity is visited once.
4. No path: the amount is not converted. It keeps its price and lot.

A cost (`@`, `@@`) and a lot are never prices: they play no part in the graph.
The path is the shortest in number of edges; the rates play no part in the choice.

This ordering was checked against hledger on 360 random journals of 3 to 6 commodities
and 2 to 9 prices (every one of four dates and every target commodity, one amount of each commodity): the rule
above gave the expected amount in every query. Two variants of step 2, sorting the forward and inverse
neighbours together by name and trying the inverse ones first, were wrong in 43 and 67 of the queries of
60 of those journals. (The checker was a throwaway script, not kept.)

### Probe

`probes/value-path-forward-first.journal`, `probes/value-path-forward-shortest.journal`,
`probes/value-path-inverse-shortest.journal`, `probes/value-path-mixed.journal`,
`probes/value-path-mixed-names.journal`, `probes/value-path-mixed-ties.journal`,
`probes/value-path-mixed-both-directions.journal`,
`probes/value-path-cycle-and-none.journal`

### Command and output

Forward first: A to B 2, B to C 3, C to D 5, and a newer D to A 7. A is valued in D along the three
forward edges (30), not by the direct inverse of D to A (1/7). In the other direction (D valued in A), the
only forward path is the direct edge D to A, 7:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-03,D spec/hledger/probes/value-path-forward-first.journal
2026-01-03  a: D 30 | b: D 1
$ spec/hledger/show-amounts.sh --value=2026-01-03,A spec/hledger/probes/value-path-forward-first.journal
2026-01-03  a: A 1 | b: A 7
```

Forward shortest: from A, the direct edge to E (5) beats A, B, E (6); to G, A to C to G (77) and A to D to G
(221) have the same length and C sorts before D; to H, through `Z` (437) or through `a` (899): `Z` sorts before `a`.

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,E spec/hledger/probes/value-path-forward-shortest.journal
2026-01-02  x: E 5
$ spec/hledger/show-amounts.sh --value=2026-01-02,G spec/hledger/probes/value-path-forward-shortest.journal
2026-01-02  x: G 77
$ spec/hledger/show-amounts.sh --value=2026-01-02,H spec/hledger/probes/value-path-forward-shortest.journal
2026-01-02  x: H 437
```

Inverse shortest (no forward path from A): to D the direct inverse (1/7) beats the three-edge path
through B and C; to F two inverse edges (1/11 then 1/13); to C two inverse edges (1/2, 1/3).

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,D spec/hledger/probes/value-path-inverse-shortest.journal
2026-01-02  x: D 0.1428571429
$ spec/hledger/show-amounts.sh --value=2026-01-02,F spec/hledger/probes/value-path-inverse-shortest.journal
2026-01-02  x: F 0.0069930070
$ spec/hledger/show-amounts.sh --value=2026-01-02,C spec/hledger/probes/value-path-inverse-shortest.journal
2026-01-02  x: C 0.1666666667
```

Mixed: A to B is a forward edge, C to A an inverse one (from A); both reach D in two edges. The forward
first step wins (2 times 1/5 is 0.4):

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,D spec/hledger/probes/value-path-mixed.journal
2026-01-02  x: D 0.4
```

Mixed, names against direction: the first step to Z (forward, then an inverse edge to M: 2 times 1/3) wins
over the step to B (inverse, then forward: 1/5 times 7) although B sorts first. Two inverse first steps:
the smaller name, B (2.5), not C:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,M spec/hledger/probes/value-path-mixed-names.journal
2026-01-02  x: M 0.6666666667
$ spec/hledger/show-amounts.sh --value=2026-01-02,D spec/hledger/probes/value-path-mixed-ties.journal
2026-01-02  x: D 2.5
```

A and B with a price each way (A is 2 B, B is 5 A) and C is 3 B: no forward path from A to C, and the
path through B uses the forward edge A to B (2 times 1/3 = 0.6666666667), not the inverse of B to A:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,C spec/hledger/probes/value-path-mixed-both-directions.journal
2026-01-02  x: C 0.6666666667
```

A cycle (A to B to C to A) and unreachable commodities. Valued in C, A takes the two forward edges (6) rather than the
inverse of C to A (0.2); C is already C; `K` has no path to C and keeps its price and lot, `Q` as well. Valued in L,
K takes the edge K to L (2): the price `@ 3 L` plays no part, and the lot is dropped:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,C spec/hledger/probes/value-path-cycle-and-none.journal
2026-01-02  x: C 6 | x: C 1 | x: K 4 @ L 3 | x: K 2 {L 4} | x: Q 1
$ spec/hledger/show-amounts.sh --value=2026-01-02,L spec/hledger/probes/value-path-cycle-and-none.journal
2026-01-02  x: A 1 | x: C 1 | x: L 8 | x: L 4 | x: Q 1
```

## Rule: inverse rates

The inverse of a price p is 1/p, computed in decimal and **rounded half to even to 255 decimal places**.
It is not a fraction: `1/3` is 255 threes, and three of them make 255 nines. The inverse of a zero price is 0
(no error). `1/8` is 0.125 exactly, as it fits.

The consequences, from the arithmetic of [value-arithmetic](value-arithmetic.md): 3 EUR valued in X (P X 3 EUR) is
`3 × 0.333…3 = 0.999…9`, which is rounded to ten places as `1.0000000000` and is **not** shortened to `1`
(rounding to ten places leaves the places it made). 9 EUR gives `3.0000000000`. 1 EUR gives `0.3333333333`. But when the
product of the 255 places rounds to a short number exactly, it is shortened as any exact result: 0.9 EUR gives
`0.3` (0.9 × 0.333…3 has 256 places and rounds up to 255 places `0.3`), 0.9 A over two thirds gives `0.1`.

### Probe

`probes/value-inverse.journal`, `probes/value-inverse-two-hops.journal`

### Command and output

X is 3 EUR, Y is 8 EUR, W is 0 EUR, V is 0.207 EUR, U is 7 EUR; EUR amounts 1, 3, 9, 2, 0.9, 1000000:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,X spec/hledger/probes/value-inverse.journal
2026-01-02  a: X 0.3333333333 | b: X 1.0000000000 | c: X 3.0000000000 | d: X 0.6666666667 | e: X 0.3 | f: X 333333.3333333333
$ spec/hledger/show-amounts.sh --value=2026-01-02,Y spec/hledger/probes/value-inverse.journal
2026-01-02  a: Y 0.125 | b: Y 0.375 | c: Y 1.125 | d: Y 0.25 | e: Y 0.1125 | f: Y 125000
$ spec/hledger/show-amounts.sh --value=2026-01-02,W spec/hledger/probes/value-inverse.journal
2026-01-02  a: W 0 | b: W 0 | c: W 0 | d: W 0 | e: W 0 | f: W 0
$ spec/hledger/show-amounts.sh --value=2026-01-02,V spec/hledger/probes/value-inverse.journal
2026-01-02  a: V 4.8309178744 | b: V 14.4927536232 | c: V 43.4782608696 | d: V 9.6618357488 | e: V 4.3478260870 | f: V 4830917.8743961353
$ spec/hledger/show-amounts.sh --value=2026-01-02,U spec/hledger/probes/value-inverse.journal
2026-01-02  a: U 0.1428571429 | b: U 0.4285714286 | c: U 1.2857142857 | d: U 0.2857142857 | e: U 0.1285714286 | f: U 142857.1428571429
```

A path of two inverse edges (X is 3 A, Y is 3 X): the rate is the product of two 255-place numbers, rounded to 255
places again; 1 A gives 0.1111111111, 0.9 A gives exactly 0.1, 9 A gives 1.0000000000, 27 A gives 3.0000000000:

```text
$ spec/hledger/show-amounts.sh --value=2026-01-02,Y spec/hledger/probes/value-inverse-two-hops.journal
2026-01-02  a: Y 0.1111111111 | b: Y 0.1 | c: Y 1.0000000000 | d: Y 3.0000000000
```

## Gaps

- `--value=DATE` with no commodity (or an empty one), `now`, `end`, `then`, `--value` with `--cost`,
  `-V`, `-X` are not probed: ledgermill's call always names a date and a commodity. With
  `--value=2026-01-09,` hledger converts X of `value-p-latest.journal` into EUR, which looks like
  "the commodity the amount's price leads to"; the rule was not worked out.
- Prices dated with a date that `include`d files give in another order: `include` is refused by
  ledgermill, which is handed the files.
