# Balance assignments: balancing, elided amounts, order, and what is refused

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

### Balancing

- The amount an assignment gets ([assignments](assignments.md)) is an ordinary amount of its
  posting: it is summed with the others in its group (regular postings; balanced virtual `[...]`
  postings; a `(virtual)` posting is not balanced), and a zero difference means no amount.
- **An assignment is not an elided amount.** A group can hold assignments and one posting with no
  amount at all, and that posting gets the negated sum (`a = 5 EUR`, `b = -4 EUR`, `c` gives `c`
  -1 EUR). Two real postings with no amount and no assertion are still the two-elided error, with
  assignments beside them or not. An elided `(virtual)` posting gets a `0` with no commodity; with
  `[b] = 3 EUR` the elided `[o]` gets -3 EUR; with a zero difference in the group, the elided
  posting gets no amount unless other postings give it one.
- **Places.** The places of the assigned amount count for the tolerance like those of an amount
  written in the transaction ([balancing](balancing.md)): with `a` at 1.234, `a = 2 EUR` assigns
  `0.766`, and `b -0.77 EUR` leaves 0.004 and fails, while `b -0.766 EUR` balances. The places
  of the amount in a *plain* assertion count for nothing: `1 X @ 0.44 EUR` with `-0.4 EUR =
  -0.4000 EUR` balances (precision 1), `1 X @ 0.44 EUR` with `= -0.4000 EUR` does not (an assignment
  of `-0.4000`), and does with `= -0.4 EUR`.
- **Inferred prices** are given to assigned amounts like to written ones
  ([inferred-prices](inferred-prices.md)): `s = 5 USD` with `t -3 EUR` gives `s` the amount
  `5 USD @@ 3 EUR`; with `u -3 EUR` first and `v = 5 USD` after it, the first posting of the two
  commodities, `u`, gets the price (`-3 EUR @@ -5 USD`); a commodity in several postings gets unit
  prices. A posting with several assigned amounts (a `==` that also zeroes other commodities) is
  never given a price: the transaction is then unbalanced.

### Two ways of processing a transaction

hledger treats a transaction in one of two ways. **A transaction with an assignment** is one
with at least one posting that has an assertion and no amount, a `(virtual)` or `[balanced]` one
included, and one whose difference turned out zero. All the others are **plain**. For each file
(see [assertion-failures](assertion-failures.md) for several files):

1. The file is parsed (every parse error).
2. Every plain transaction is balanced, in file order: elided amounts are worked out, the group
   checks are made. The first problem stops everything.
3. Then all postings are walked in the order of [assertion-order](assertion-order.md), with running
   balances:
   - a plain posting, at its own date (its posting date if it has one): its amount is added to the
     balance of its account, then its assertion, if any, is checked;
   - a transaction with an assignment is taken as a **block, at the date of the transaction**, at
     the place where the transaction is written among the others of that date. Its postings are
     looked at in the order written, whatever their posting dates: the one dated March is in the
     balance on the transaction's 1 January, unlike in a plain transaction. For each posting: an
     assignment is worked out against the balance now, and the balance is set as in
     [assignments](assignments.md); a posting with an amount is added to the balance and its
     assertion checked; a posting with no amount (the one that will be elided) is passed over.
     After the last posting the transaction is balanced like a plain one (the elided amount is
     worked out, the group checks made, a price inferred) and the elided amounts are added to
     the balances of their accounts then.

**Posting dates inside the block.** A posting of such a transaction dated *after* the transaction is
not in the balance for the postings before it: it waits for the end of the block, is added then (its
assertion, if any, is checked then, before the transaction is balanced), and is in the balance for
everything after. A posting dated on or before the transaction is in at once. (Probe
`assign-later-posting-date`: `x 5 EUR ; date:2026-03-01`, then `x = 7 EUR` gives `7`, and the `x 0 EUR =
7 EUR` after it holds; the next day `x` holds 12. `assign-fail-later-posting-date`: `x 0 EUR = 5 EUR`
after the assignment fails at its line, calculated 0 EUR. A posting dated 2026-01-01 and one dated
2026-01-04 in a transaction of 4 January are both in at once.) This is how the 'dated March is in the
balance' of the list above is to be read: in for everything after the block.

**Prices of postings in the block.** A posting with a price (`@`, `@@`) of a transaction with an
assignment is added to the balance *with* its price, as an entry of its own price group; in a plain
transaction the price is dropped ([assignment-lots-and-costs](assignment-lots-and-costs.md)). The
entries stay in the balance for later transactions. A later assignment subtracts the entries as
they are, so `==` zeroes a priced entry with that price, and a priced and a plain entry in one commodity
are two amounts (costless first). Probe `assign-entry-prices`:

```text
 1: x [1 X @ 2 EUR]  w [-]  y [-2 EUR]              (w = 0 EUR makes the transaction an assignment one)
 6: x [-1 X @ 2 EUR] z [2 EUR]                       x == 0 EUR the next day: the price comes along
10: p [1 X @ 2 EUR]  y [-2 EUR]                      plain transaction
14: p [-1 X]         z [1 X]                         the price was dropped: none on the zeroing amount
18: q [3 X @@ 6 EUR, -3 X @@ -6 EUR]  y [0 EUR]      same transaction: a total price is subtracted as a total
23: r [1 X @ 2 EUR]  r [2 X]  r [-2 X, -1 X @ 2 EUR]  y [0 EUR, 0 X]   a priced and a plain entry: two amounts
```

So an assertion or assignment inside a transaction with an assignment does not see the elided
posting of its own transaction, before or after the elided posting in the order written; one in
a later transaction does. In a plain transaction an assertion on the elided account sees its
amount. With `b` left out, then `a = 5`, then `b 0 EUR = -5 EUR`, the assertion fails with `b` at
0, while with `a 5` instead of `a = 5` it holds. An assignment of `b` itself, `b = -7`, is
worked out on the balance without the elided amount; the elided 2 is then added, and the next day
`b` holds -5 and not -7.

### What is refused

- An assignment posting with a posting date: any `date:` tag or `[DATE]`-style date in its own
  comment (also one equal to the transaction's date, also under `-I`):
  `Balance assignments and custom posting dates may not be combined.` The message shows the
  transaction and names no file or line. `date2:` is fine. Other postings of the transaction
  may have a date.
- Two real postings with no amount (see above).
- Amounts that do not sum to zero per commodity: the assigned amounts are in the sum (`a = 5 EUR`,
  `b = -4 EUR` leaves 1 EUR; `a = 5 EUR` alone is a posting of 5 EUR with nothing against it; a `==`
  that also zeroes a commodity another posting cannot take).
- Syntax errors of the assertion ([assertion-syntax](assertion-syntax.md)).

Accepted: several assignments on one account in a transaction (each works from the balance the
previous one left), a plain posting and an assignment of the same account in any order, assignments
on `(virtual)` and `[balanced]` postings, `date2:`, an assignment in a transaction with a second
date, an assignment under `-I` (the flag only skips assertions: the amounts are filled in).

Which error comes first when there are several is in [assertion-failures](assertion-failures.md).

## Probe

`probes/assign-balancing.journal`, `probes/assign-entry-prices.journal`,
`probes/assign-later-posting-date.journal`, `probes/assign-fail-later-posting-date.journal`,
`probes/assign-precision.journal`,
`probes/assign-fail-precision.journal`, `probes/assign-fail-precision-assertion.journal`,
`probes/assign-processing.journal`, `probes/assign-fail-elided-not-yet.journal`,
`probes/assign-fail-elided-not-yet-virtual.journal`,
`probes/assign-fail-elided-not-yet-zero.journal`,
`probes/assign-fail-posting-dates-in-transaction.journal`, `probes/assign-order.journal`,
`probes/assign-posting-date2.journal`, `probes/assign-fail-posting-date.journal`,
`probes/assign-fail-posting-date-same.journal`, `probes/assign-fail-posting-date-bracket.journal`,
`probes/assign-fail-two-elided.journal`, `probes/assign-fail-unbalanced.journal`,
`probes/assign-fail-single.journal`, `probes/assign-fail-total-unbalanced.journal`,
`probes/assign-file-b.journal` (with `probes/assert-file-a.journal`)

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/assign-balancing.journal
spec/hledger/show.sh spec/hledger/probes/assign-precision.journal
spec/hledger/show.sh spec/hledger/probes/assign-processing.journal
spec/hledger/show.sh spec/hledger/probes/assign-order.journal
spec/hledger/show.sh spec/hledger/probes/assign-entry-prices.journal
spec/hledger/show.sh spec/hledger/probes/assign-later-posting-date.journal
spec/hledger/show.sh spec/hledger/probes/assert-file-a.journal spec/hledger/probes/assign-file-b.journal
spec/hledger/assertion-verdicts.sh spec/hledger/probes/assign-fail-*.journal \
  spec/hledger/probes/assign-posting-date2.journal
hledger --no-conf -f spec/hledger/probes/assign-fail-posting-date.journal print
```

## Output

Below each transaction is `line`, then `account amounts` (`[commodity, mantissa, places]`).
assign-balancing:

```text
1  a [EUR 5]  b [EUR -4]  c [EUR -1]            (a = 5, b = -4, c left out)
6  d [EUR 5]  e [EUR -5]
10 f [EUR 5]  f [EUR 2]   g [EUR -7]            (f = 5, then f = 7)
15 h [EUR 1]  h [EUR 6]  h [EUR 2]  h [EUR 2]  g [EUR -11]
22 i [EUR 5]  j [EUR 1]  k [EUR -1]             ((i) = 5 is not balanced)
27 l [EUR 5]  v [["",0,0]]  m [EUR -5]          (an elided virtual posting)
32 l []  n [EUR 3]  o [EUR -3]  m []            (l = 5 again: no amount; [n] = 3, [o] left out)
38 p []  q [EUR 5]  r [EUR -5]
43 s [USD 5 @@ EUR 3]  t [EUR -3]               (inferred price on the assigned amount)
47 u [EUR -3 @@ USD -5]  v [USD 5]
51 w [USD 1 @ EUR 0.8]  w [USD 4 @ EUR 0.8]  t [EUR -3]  t [EUR -1]
```

(Each EUR/USD amount above stands for `["EUR",5,0]` and so on; prices are `["@@","EUR",3,0]`,
`["@","EUR",8,1]`.) assign-precision, the two accepted transactions with their assigned
`["EUR",766,3]` and the one with `0.4`:

```text
5  a [["EUR",766,3]]  b [["EUR",-766,3]]
9  c [["X",1,0,["@","EUR",44,2]]]  d [["EUR",-4,1]]       (the assertion -0.4000 does not count)
13 e [["X",1,0,["@","EUR",44,2]]]  f [["EUR",-4,1]]
```

assign-processing (lines 1-20: plain, then a transaction with an assignment; 24 a virtual assignment
and a zero difference; 30-35 plain: the dated posting is left out on 8 January; 39-44 the block):

```text
1  a [EUR 5]  b [EUR -5]  b [EUR 0]               plain: the assertion after the elided b sees -5
6  c [EUR -5]  d [EUR 5]  c [EUR 0]               assignment: c = 0 at that point, -5 added at the end
11 c [EUR 0]  y [EUR 0]                           the next day c holds -5
15 m [EUR 2]  f [EUR 5]  m [EUR -7]               m = -7 on a balance of 0, the elided 2 added after
20 m [EUR 0]  y [EUR 0]                           m holds -5
24 g [EUR 5]  h [EUR -5]  v []  h [EUR 0]         (v) = 0 EUR, a zero difference, is an assignment
30 k [EUR 5]  k [EUR 1]  z [EUR -6]               the second k is dated 1 March
35 k [EUR 0]  y [EUR 0]                           k = 5 on 8 January
39 l [EUR 5]  l [EUR 1]  z [EUR -6]               l = 5, then a posting dated 1 March
44 l [EUR 0]  y [EUR 0]                           l = 6 on 10 January: the dated posting is already in
```

assign-order (printed in date order; `a`: assignment of 1 March written first, 3 on 1 January
written second; `b`: two assignments on one day; `c`: a plain posting dated 5 January written before
the assignment of the 5th; `e`: the other way round):

```text
5  a [EUR 3]  z [EUR -3]
9  b [EUR 5]  z [EUR -5]
13 b [EUR 3]  z [EUR -3]
17 c [EUR 1]  z [EUR -1]
29 e [EUR 1]  z [EUR -1]
21 c [EUR 2]  z [EUR -2]                          3 - 1
25 e [EUR 3]  z [EUR -3]                          the assignment first: 3
33 c [EUR 0]  e [EUR 0]  y [EUR 0]                c holds 3, e holds 4
1  a [EUR 2]  z [EUR -2]                          5 - 3, last in date order
```

The same file pair, `assert-file-a` then `assign-file-b` (one file does not see the other):

```text
{"date":"2026-01-02","status":"Unmarked","code":"","description":"an assignment sees only its own file: 20, not 10","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",20,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-20,0]]}]}
```

The failing journals:

```text
assign-fail-elided-not-yet | rejects | assign-fail-elided-not-yet.journal:4:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted -5 EUR; calculated 0 EUR
assign-fail-elided-not-yet-virtual | rejects | assign-fail-elided-not-yet-virtual.journal:5:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted -5 EUR; calculated 0 EUR
assign-fail-elided-not-yet-zero | rejects | assign-fail-elided-not-yet-zero.journal:5:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted -5 EUR; calculated 0 EUR
assign-fail-posting-dates-in-transaction | rejects | assign-fail-posting-dates-in-transaction.journal:7:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 5 EUR; calculated 6 EUR
assign-fail-posting-date | rejects | - | Balance assignments and custom posting dates may not be combined.
assign-fail-posting-date-same | rejects | - | Balance assignments and custom posting dates may not be combined.
assign-fail-posting-date-bracket | rejects | - | Balance assignments and custom posting dates may not be combined.
assign-fail-precision | rejects | assign-fail-precision.journal:5 | unbalanced
assign-fail-precision-assertion | rejects | assign-fail-precision-assertion.journal:1 | unbalanced
assign-fail-single | rejects | assign-fail-single.journal:1 | unbalanced
assign-fail-total-unbalanced | rejects | assign-fail-total-unbalanced.journal:5 | unbalanced
assign-fail-two-elided | rejects | assign-fail-two-elided.journal:1 | two-elided
assign-fail-unbalanced | rejects | assign-fail-unbalanced.journal:1 | unbalanced
assign-posting-date2 | accepts |
```

The posting-date error in full (no file, no line):

```text
hledger: Error: Balance assignments and custom posting dates may not be combined.

2026-01-01 t
    a                 = 5 EUR  ; date:2026-02-01
    a           1 EUR
    z

Balance assignments may not be used on postings with a custom posting date
(it makes balancing the journal impossible).
Please write the posting amount explicitly (or remove the posting date).
```

(In the failing `assign-fail-*` journals the position is the transaction's first line for the
balancing errors, `FILE:N`; for an assertion that fails inside a transaction, the posting's line and
column, as everywhere.)
