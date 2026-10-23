# Holdings: `balance --layout=tidy --real` with `--cost` and `--value=end`

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-23 (the day of the probes: "today" below).

Status: spec only; the engine call is not built.

Plainledger reads its holdings from four `hledger balance` runs. The engine call to design for is
`holdings({ end?: D, commodity: V })`. For every account and every commodity C it answers four
exact positions, over the whole ledger and **real postings only** (`(a)` and `[a]` postings are
ignored everywhere):

| field | the run it replaces (queries `-- P1 ... Pn cur:C`, every run with `--real` and `-e D` when D is given) |
|---|---|
| `units` | `balance --layout=tidy -O csv` (with no `cur:`: every commodity at once) |
| `cost` | `... --cost` |
| `value` | `... --value=end,V` |
| `costValue` | `... --cost --value=end,V` |

Prefix filtering (`P1 ... Pn`), symbol aliases (`€` for `EUR`) and the rule for a missing cost stay in
Plainledger. This note says what hledger puts in those rows, so the engine can return the same
numbers and the harness can ask hledger the same questions.

The lines below are made by `spec/hledger/show-holdings.sh`:

- default: the **exact** amounts from `balance --layout=tidy -O json`, one line per account,
  `ACCOUNT: COMMODITY NUMBER | COMMODITY NUMBER`, the amounts of one commodity added up whatever their costs;
- `--csv`: the rows of `balance --layout=tidy -O csv` as Plainledger reads them,
  `ACCOUNT  COMMODITY  VALUE  START..END`.

Options go before the files; the end date is written `--end=D` (it is `-e D`); the query follows a `--`.
Amounts are ledgermill's [numbers](numbers.md): exact decimals; [cost](cost.md) says what `--cost` does to an
amount, [value-prices](value-prices.md) which price and path a value takes, [value-arithmetic](value-arithmetic.md) the
products and the rounding to ten places. This note does not repeat them.

## Rule: which postings count (`-e D`)

A posting counts when its own date is **strictly before D**. Its own date is its `date:` tag when
it has one, else its transaction's date. The transaction's date when the tag is on another posting
does not matter, and the secondary date (`2026-01-20=2026-05-25`) is never used. Without `-e`, every
posting counts, including those dated after today. With `-e D` and no posting before D the run has no row.

This applies to all four runs.

### Probe

`probes/hold-e-boundary.journal`, `probes/hold-e-tags.journal`

### Command and output

hold-e-boundary (X is 1, 2, 3, 4 EUR from 01-01, 02-01, 03-01, 04-01; 10 X on 01-10 and 10 X on 02-10; the
CSV run, so the period shows the date `--value=end` uses, see the next rule). The lot of 01-10 is not counted
by `-e 2026-01-10`, it is by `-e 2026-01-11`; same for 02-10:

```text
$ spec/hledger/show-holdings.sh --csv --end=2026-01-10 --value=end,EUR spec/hledger/probes/hold-e-boundary.journal -- a
$ spec/hledger/show-holdings.sh --csv --end=2026-01-11 --value=end,EUR spec/hledger/probes/hold-e-boundary.journal -- a
a  EUR  10  2026-01-10
$ spec/hledger/show-holdings.sh --csv --end=2026-02-10 --value=end,EUR spec/hledger/probes/hold-e-boundary.journal -- a
a  EUR  20  2026-01-10..2026-02-09
$ spec/hledger/show-holdings.sh --csv --end=2026-02-11 --value=end,EUR spec/hledger/probes/hold-e-boundary.journal -- a
a  EUR  40  2026-01-10..2026-02-10
$ spec/hledger/show-holdings.sh --csv --end=2026-02-11 spec/hledger/probes/hold-e-boundary.journal -- a
a  X  20  2026-01-10..2026-02-10
```

hold-e-tags (`a 10 X ; date:2026-01-05` and `b -10 X ; date:2026-05-05` in a transaction dated 03-01; a
second transaction dated 01-20 with the secondary date 05-25). `a` counts from 01-05, `b` only from 05-06,
`c` and `d` from 01-21 whatever the secondary date:

```text
$ spec/hledger/show-holdings.sh --csv --end=2026-01-06 spec/hledger/probes/hold-e-tags.journal
a  X  10  2026-01-05
$ spec/hledger/show-holdings.sh --csv --end=2026-03-02 spec/hledger/probes/hold-e-tags.journal
a  X  10  2026-01-05..2026-03-01
c  X  1  2026-01-05..2026-03-01
d  X  -1  2026-01-05..2026-03-01
$ spec/hledger/show-holdings.sh --csv --end=2026-05-06 spec/hledger/probes/hold-e-tags.journal
a  X  10  2026-01-05..2026-05-05
b  X  -10  2026-01-05..2026-05-05
c  X  1  2026-01-05..2026-05-05
d  X  -1  2026-01-05..2026-05-05
```

(How `date:` tags are read is in [comment-dates](comment-dates.md).)

## Rule: the date `--value=end` values at

The valuation date E is a date, and the prices used are those of E
([value-prices](value-prices.md): the `P` lines dated on or before E). The engine can call
`Ledger::value(E, V)` arithmetic with it. E is:

1. **with `-e D`: the day before D**, whatever the journal holds (`-e 2026-05-01` values at `2026-04-30` although
   the last price is dated 04-01). It is the last day of the period hledger reports. The row
   exists only when a posting counts (previous rule).
2. **without `-e`: the latest of**
   - the date of every transaction of the journal,
   - the own date of every posting of the journal (a `date:` tag),
   - the date of every `P` line **that is not after today**.

   Nothing else counts: not the secondary dates, not `P` lines dated after today. A transaction dated after today
   does count (E may be in the future). The latest of these dates is E, so a price dated after the last
   transaction moves E (up to today), and E is never earlier than the last transaction.

The set of dates is that of the **whole journal**, not of the rows asked for: virtual postings count (their
transaction date and their `date:` tags, even with `--real`), and so do transactions and `P` lines of other
accounts and other commodities than the query's.

Today is hledger's clock (2026-10-23 here). It enters only through the `P` lines dated after today, so the engine needs a
`today` (a `YYYY-MM-DD` input, defaulted to the local date) next to `end`. Probes use 2099 for "after today".

The units run (no `--value`) shows a period that ends at the latest transaction or posting date alone (secondary dates and
`P` lines play no part there); the value run's period ends at E.

### Probe

`probes/hold-end-p-after.journal`, `probes/hold-end-p-before.journal`, `probes/hold-end-p-future.journal`,
`probes/hold-end-p-mixed.journal`, `probes/hold-end-txn-future.journal`, `probes/hold-end-unrelated.journal`,
`probes/hold-end-tag-earlier.journal`, `probes/hold-end-virtual.journal`, `probes/hold-end-only-p.journal`,
`probes/hold-e-boundary.journal`, `probes/hold-e-tags.journal`

### Command and output

The last `P` line (06-01, X = 5) is after the last transaction (03-10): E is 06-01, and 20 X are 100 EUR. With the
last transaction after the last `P` line (02-01, X = 2): E is 03-10, 20 X are 40 EUR. With `-e 2026-03-11` on the first
journal E is 03-10 and the price of 06-01 is not known yet:

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-p-after.journal
a  EUR  100  2026-01-10..2026-06-01
b  EUR  -100  2026-01-10..2026-06-01
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-p-before.journal
a  EUR  40  2026-01-10..2026-03-10
b  EUR  -40  2026-01-10..2026-03-10
$ spec/hledger/show-holdings.sh --csv --value=end,EUR --end=2026-03-11 spec/hledger/probes/hold-end-p-after.journal
a  EUR  40  2026-01-10..2026-03-10
b  EUR  -40  2026-01-10..2026-03-10
```

A `P` line after today does not move E (the 2099 price of 9 is ignored; the only transaction is 2026-03-10: E is that
date and 10 X are 10 EUR). With another `P` line between (06-01) the one before today moves E and the one of 2099 is ignored.
A transaction after today moves E (2099-03-10); the `P` line before it (2099-02-01, X = 5) counts, the one after it (2099-06-01) does not:

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-p-future.journal
a  EUR  10  2026-03-10
b  EUR  -10  2026-03-10
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-p-mixed.journal -- a
a  EUR  50  2026-03-10..2026-06-01
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-txn-future.journal
a  EUR  100  2026-01-10..2099-03-10
b  EUR  -100  2026-01-10..2099-03-10
```

The whole journal decides, not the query: the last `P` line is for Y (08-01), the last transaction is in account `c` (05-10). Querying `a` alone
values at 08-01, though the units run of `a` ends 05-10. A `P` line alone gives no row:

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-unrelated.journal -- a
a  EUR  20  2026-01-10..2026-08-01
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-end-unrelated.journal -- a
a  X  10  2026-01-10..2026-05-10
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-only-p.journal
```

Postings dated earlier than their transaction (01-05, 02-05 in a transaction of 03-10): E is the transaction's 03-10, and the
start of the period is 01-05. A virtual posting `(v) 1 X ; date:2026-06-01` moves E to 06-01 although `--real` drops it; so does the
date of a transaction that has only a virtual posting (02-10, here earlier than the others):

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-tag-earlier.journal
a  EUR  30  2026-01-05..2026-03-10
b  EUR  -30  2026-01-05..2026-03-10
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-end-virtual.journal
a  EUR  40  2026-01-10..2026-06-01
b  EUR  -40  2026-01-10..2026-06-01
```

With a `date:` tag a posting may be dated later than its transaction; hold-e-tags run without `-e` ends at the latest
`P` line (06-01), while the units run of the same journal ends at the posting dated 05-05:

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-e-tags.journal
a  EUR  60  2026-01-05..2026-06-01
b  EUR  -60  2026-01-05..2026-06-01
c  EUR  6  2026-01-05..2026-06-01
d  EUR  -6  2026-01-05..2026-06-01
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-e-tags.journal
a  X  10  2026-01-05..2026-05-05
b  X  -10  2026-01-05..2026-05-05
c  X  1  2026-01-05..2026-05-05
d  X  -1  2026-01-05..2026-05-05
```

Check: 150 random journals (1 to 4 transactions, `date:` tags, virtual postings, 1 to 4 `P` lines, dates in 2026 and 2099, `-e` or
not) gave exactly the date, the rows and the value this rule predicts (a throwaway script, not kept).

## Rule: how a position is made

Per account and per commodity C, the position is the **sum of the amounts**, exact, of the postings that count:

- **units** (`balance`): the sum of the quantities of C. The costs and lots of the amounts play no part (10 X @ 2 EUR and 5 X are 15 X;
  hledger's JSON lists them apart, its CSV adds them up).
- **value** (`--value=end,V`): the units, valued at E in V by [value-arithmetic](value-arithmetic.md). A cost is not a price.
  C = V stays as it is (units, summed). No price path from C to V: the amount stays in C.
- **cost** (`--cost`): every posting is converted first ([cost](cost.md)), then summed per commodity. The result can hold several commodities: the costs' commodities,
  and C itself for the postings with no cost. The conversion postings are dropped as `print --cost` drops them (below).
- **costValue** (`--cost --value=end,V`): the **cost first, then the value**: each commodity of the cost is valued like a unit amount, so a cost in V stays,
  a cost in another commodity with a price path to V is converted at E, the amounts with no cost are valued as in `value`, and what has no path stays where it is. When C = V
  the postings with a cost in another commodity leave V and come back at the price of E.

Valuing the sum or summing the values is the same to ten places: three postings of 1 EUR valued in Y (P Y = 3 EUR) give 1.0000000000 Y, as
the sum 3 EUR times the 255-place inverse does, not 3 times 0.3333333333 (0.9999999999). The rounding to ten places is done once, on the sum.
A quantity of a position is a [number](numbers.md) with up to 255 places inside and at most ten places once valued (not shortened if it was rounded).

### Probe

`probes/hold-sum.journal`, `probes/hold-cost.journal`, `probes/hold-cost-self.journal`, `probes/hold-cost-conversion.journal`, `probes/hold-noprice.journal`,
`probes/hold-real.journal`

### Command and output

hold-sum (three postings of 1 EUR; the CSV rounds, see below, so the JSON is the line to follow):

```text
$ spec/hledger/show-holdings.sh --value=end,Y spec/hledger/probes/hold-sum.journal
a: Y 1.0000000000
b: Y -1.0000000000
```

hold-cost (account `a`: 10 X at 2 EUR each, 5 X with no cost, 2 X for 5 USD; X = 3 EUR; EUR = 1.1 USD), query `a cur:X`. `--cost`
gives EUR, USD and X itself; `--value` values the 17 X at 3 (51); `--cost --value` takes 20 EUR, 5 USD / 1.1 and 5 X times 3
(20 + 4.5454545455 + 15). In USD (X to EUR to USD): 22 + 5 + 16.5:

```text
$ spec/hledger/show-holdings.sh spec/hledger/probes/hold-cost.journal -- a cur:X
a: X 17
$ spec/hledger/show-holdings.sh --cost spec/hledger/probes/hold-cost.journal -- a cur:X
a: EUR 20 | USD 5 | X 5
$ spec/hledger/show-holdings.sh --value=end,EUR spec/hledger/probes/hold-cost.journal -- a cur:X
a: EUR 51
$ spec/hledger/show-holdings.sh --cost --value=end,EUR spec/hledger/probes/hold-cost.journal -- a cur:X
a: EUR 39.5454545455
$ spec/hledger/show-holdings.sh --cost --value=end,USD spec/hledger/probes/hold-cost.journal -- a cur:X
a: USD 43.5
$ spec/hledger/show-holdings.sh --value=end,X spec/hledger/probes/hold-cost.journal -- a cur:X
a: X 17
```

hold-cost-self (C = V = EUR, 10 EUR bought at 1.2 USD each, USD = 0.9 EUR): the value leaves the EUR as it is, the cost turns it into USD, and
the cost value brings the 12 USD back as 10.8 EUR:

```text
$ spec/hledger/show-holdings.sh --value=end,EUR spec/hledger/probes/hold-cost-self.journal -- a cur:EUR
a: EUR 10
$ spec/hledger/show-holdings.sh --cost spec/hledger/probes/hold-cost-self.journal -- a cur:EUR
a: USD 12
$ spec/hledger/show-holdings.sh --cost --value=end,EUR spec/hledger/probes/hold-cost-self.journal -- a cur:EUR
a: EUR 10.8
```

hold-noprice (X = 3 EUR, nothing for W; a holds 10 X @ 2 USD, 2 X, 4 W): the USD cost has no path to EUR and W has none to anything.
They stay where they are:

```text
$ spec/hledger/show-holdings.sh --cost --value=end,EUR spec/hledger/probes/hold-noprice.journal -- a cur:X
a: EUR 6 | USD 20
$ spec/hledger/show-holdings.sh --cost --value=end,EUR spec/hledger/probes/hold-noprice.journal -- a cur:W
a: W 4
```

Conversion postings. A transaction through `equity:conversion` (a pair of postings on it) is dropped from the **cost** and the **cost value**
runs, as `print --cost` does ([cost](cost.md), "the postings that are dropped"); the units run and the value run keep them. So the cost of
`equity:conversion` is nothing and the account has no row, while its units are `EUR 100 | USD -110`:

```text
$ spec/hledger/show-holdings.sh spec/hledger/probes/hold-cost-conversion.journal
assets:eur: EUR -100
assets:usd: USD 110
equity:conversion: EUR 100 | USD -110
$ spec/hledger/show-holdings.sh --cost spec/hledger/probes/hold-cost-conversion.journal
assets:eur: EUR -100
assets:usd: USD 110
$ spec/hledger/show-holdings.sh --value=end,EUR spec/hledger/probes/hold-cost-conversion.journal
assets:eur: EUR -100
assets:usd: EUR 99
equity:conversion: EUR 1
$ spec/hledger/show-holdings.sh --cost --value=end,EUR spec/hledger/probes/hold-cost-conversion.journal
assets:eur: EUR -100
assets:usd: EUR 99
```

`--real` removes `(v)`, `[bv]` and `[bw]` postings from every run (only the real postings' accounts have a row; without `--real`,
`v`, `bv` and `bw` would have rows):

```text
$ spec/hledger/show-holdings.sh --cost --value=end,EUR spec/hledger/probes/hold-real.journal
a: EUR 20
b: EUR -20
```

## Rule: `cur:C`

`cur:C` keeps **the amounts** whose commodity matches, not whole postings: a posting with several amounts (an elided
posting that balances a transaction with several commodities) contributes only the matching ones. It is applied to the amounts **before**
`--cost`: with `cur:X` and `--cost`, the postings `10 X @ 2 EUR` are converted to EUR and appear in the answer, and the EUR
postings that were never X do not.

C is a **regular expression**, not a name:

- matched against the whole commodity, case-insensitively: the pattern is put between `^` and `$` *without a group*, so `X|x` is
  `^X|x$` (it matches `XY` and `2X`);
- `X` matches `X` and `x` (two different commodities; both come back, in two rows); `E F` matches the commodity written `"E F"`,
  while `"E F"` with the quotes matches nothing;
- the regex metacharacters work: `A+B` matches `AB`, not `A+B` (`A\+B` does); `A.B` matches `AXB`, `A-B`, `A.B`, `A+B`; `.` matches any one-character
  commodity; `$` matches only the empty commodity (the pattern is `^$$`), `\$` the dollar sign; `cur:A(B` is an error (exit status 1);
- `cur:` and `cur:^$` select the amounts without a commodity (the empty commodity); their rows have an empty `commodity` column.

Plainledger passes C raw, so those quirks are Plainledger's. The engine returns exact commodities: it matches C
by equality and Plainledger (or the harness) decides which commodities a `cur:` pattern would have selected.

### Probe

`probes/hold-cur-regex.journal`, `probes/hold-cur-bare.journal`

### Command and output

hold-cur-regex has one account per commodity (`a:x` is `1 X`, `a:lx` is `3 x`, `a:xy`, `a:usd` is `4 $`, `a:ef` is `5 "E F"`,
...) and an elided account `z` holding the negation of all of them. `cur:PATTERN`, accounts and commodities of the rows (`z` left out):

```text
cur:X => a:lx,x  a:x,X
cur:E F => a:ef,E F
cur:"E F" =>
cur:A+B => a:ab,AB
cur:A\+B => a:plus,A+B
cur:A.B => a:axb,AXB  a:dash,A-B  a:dot,A.B  a:plus,A+B
cur:X|x => a:digit,2X  a:lx,x  a:x,X  a:xy,XY
cur:. => a:euro,€  a:lx,x  a:usd,$  a:x,X
cur:$ =>
cur:\$ => a:usd,$
cur:€ => a:euro,€
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-cur-regex.journal -- z cur:X
z  X  -1  2026-01-01
z  x  -3  2026-01-01
```

(The first lines were made with `hledger --no-conf -f spec/hledger/probes/hold-cur-regex.journal balance --layout=tidy -O csv --real cur:PATTERN`, keeping the account
and commodity columns.) hold-cur-bare (`a 5`, `b 3 X`, `c -3 X`, `d -5`): the empty commodity is selected by `cur:` and `cur:^$`; `cur:X` leaves `b`:

```text
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-cur-bare.journal -- a b d cur:
a    5  2026-01-01
d    -5  2026-01-01
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-cur-bare.journal -- a b d cur:X
b  X  3  2026-01-01
```

## Rule: which rows the tidy layout gives

- **One row per account and commodity.** The report is flat: no totals row, no tree. An account with its own postings and children
  (`parent`, `parent:child`) gives separate rows, each its own postings only. Only accounts with a posting that counts
  can have a row.
- **Zero rows are dropped.** A commodity whose sum is exactly zero has no row; an account whose sum is zero in every commodity has
  no row. This is judged on the position of the run: `twocosts` (10 X at 2 EUR and -10 X at 3 EUR) has no units row and a cost row of
  -10 EUR; `costzero` (10 X at 2 EUR and -20 EUR) has units rows and no cost row; an account that cancels only once valued
  (3 EUR and -1.5 X at 2 EUR) has no value row; a price of zero gives no value row (`5 Z` at 0 EUR). The engine can return zero
  positions; whoever compares drops them, or treats a missing row as zero.
- **A sum that rounds to zero keeps a row.** When the sum is not zero but the value is (5e-11 EUR rounded to ten places is
  `0.0000000000`; or the display rounding of the CSV below makes `0.005` zero), the row exists with an **empty commodity**
  and the value `0`. In the exact JSON it is `EUR 0.0000000000`.
- **Order:** accounts in the order of their `:`-separated names, each name compared in code point order (so `a:B`, `a:b` come before
  `a b`, `a-b`, `a_b`, `alpha`; digits before capitals before lower case; `Émile` before `é`), then, within an account,
  commodities in code point order with the empty commodity first (`""`, `$`, `10X`, `2X`, `9X`, `A`, `A B`, `A-B`, `A.B`, `A_B`, `E F`, `EUR`, `X`, `Z`, `a`, `x`, `é`, `€`).
  Nothing depends on this for the engine's numbers; it is the order the harness has to sort to.

### Probe

`probes/hold-tidy-accounts.journal`, `probes/hold-tidy-commodities.journal`, `probes/hold-tidy-zero.journal`, `probes/hold-tidy-tiny.journal`

### Command and output

Sorting: `Zeta`, `alpha`, `Alpha`, `a:b`, `a-b`, `a_b`, `a:B`, `é`, `Émile`, `10:x`, `9:x`, `a b`, `parent`, `parent:child`, `zero`
(5 X and -5 X, no row) and the balancing `zz`:

```text
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-tidy-accounts.journal
10:x  X  1  2026-01-01
9:x  X  1  2026-01-01
Alpha  X  1  2026-01-01
Zeta  X  1  2026-01-01
a:B  X  1  2026-01-01
a:b  X  1  2026-01-01
a b  X  1  2026-01-01
a-b  X  1  2026-01-01
a_b  X  1  2026-01-01
alpha  X  1  2026-01-01
parent  X  1  2026-01-01
parent:child  X  2  2026-01-01
zz  X  -15  2026-01-01
Émile  X  1  2026-01-01
é  X  1  2026-01-01
```

hold-tidy-zero (X = 2 EUR, Z = 0 EUR), the CSV rows without the balancing `sink*` accounts. `costhalf` is 10 X at 2 EUR, -20 EUR and 5 EUR; `twocosts` is
10 X at 2 EUR and -10 X at 3 EUR; `valcancel` is 3 EUR and -1.5 X; `zeromix` is 5 X, -5 X and 3 Y (the zero X has no row; the JSON keeps `X 0` and, valued, `EUR 0`):

```text
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-tidy-zero.journal
costhalf  EUR  -15  2026-01-01..2026-01-07
costhalf  X  10.0  2026-01-01..2026-01-07
costzero  EUR  -20  2026-01-01..2026-01-07
costzero  X  10.0  2026-01-01..2026-01-07
valcancel  EUR  3  2026-01-01..2026-01-07
valcancel  X  -1.5  2026-01-01..2026-01-07
zeromix  Y  3  2026-01-01..2026-01-07
zeroprice  Z  5  2026-01-01..2026-01-07
$ spec/hledger/show-holdings.sh --csv --cost spec/hledger/probes/hold-tidy-zero.journal
costhalf  EUR  5  2026-01-01..2026-01-07
twocosts  EUR  -10  2026-01-01..2026-01-07
valcancel  EUR  3  2026-01-01..2026-01-07
valcancel  X  -1.5  2026-01-01..2026-01-07
zeromix  Y  3  2026-01-01..2026-01-07
zeroprice  Z  5  2026-01-01..2026-01-07
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-tidy-zero.journal
costhalf  EUR  5  2026-01-01..2026-01-07
zeromix  Y  3  2026-01-01..2026-01-07
$ spec/hledger/show-holdings.sh --value=end,EUR spec/hledger/probes/hold-tidy-zero.journal
costhalf: EUR 5
zeromix: EUR 0 | Y 3
```

(`sink*` rows are left out of the lists above.) hold-tidy-tiny (X = 0.5 EUR; `tiny` holds 0.0000000001 X, which is 5e-11 EUR):

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-tidy-tiny.journal
big  EUR  500.0  2026-01-01
sink  EUR  -500.0  2026-01-01
tiny    0  2026-01-01
$ spec/hledger/show-holdings.sh --value=end,EUR spec/hledger/probes/hold-tidy-tiny.journal
big: EUR 500
sink: EUR -500.0000000000
tiny: EUR 0.0000000000
```

## Rule: the CSV is rounded; the exact value is in the JSON

The tidy CSV writes each value in the **display style of its commodity**, so it is not the exact position:

- rounded **half to even** to the commodity's display precision: the most places any amount of that commodity has in the journal,
  including the amounts of `P` lines (`P X 0.5 EUR` makes the precision of `EUR` at least 1) and of costs; a commodity that has no style
  at all (it appears only as the target of a valuation, or only in `P` lines as the priced commodity)
  is written with 8 places;
- the decimal mark of that style (`7900,00` for a commodity declared `1.000,00 EUR`; the dot for the others), and **no digit groups**;
- a value that rounds to zero loses its commodity (the row above: empty commodity, `0`).

`balance -O json` with the same flags gives the exact numbers: mantissa and places, ten places at most for a valued amount, and the **same rows and the
same values** once rounded to the CSV's places (checked on every probe of this note under seven option sets: 189 runs, 1,087 CSV rows, no
difference, zero rows aside). JSON lists amounts that have costs apart (`X 5`, `X 10 @ 2 EUR`, `X 2 @@ 5 USD`) and keeps zero amounts
(`X 0`); added up per commodity it is the CSV without the zero rows.

**The engine matches the exact value (the JSON)**, not the CSV text. The harness reads the JSON, or rounds the engine's value
half to even to the places of the CSV row (`show-holdings.sh`'s default is the JSON).

### Probe

`probes/hold-csv-round.journal`, `probes/hold-csv-marks.journal`, `probes/hold-sum.journal`

### Command and output

hold-csv-round (EUR has two places because of `anchor 1.00 EUR`; X = 0.5 EUR; the amounts are 0.25, 0.27, 0.29, 0.23, -0.25, 0.01 X). The CSV
gives 0.12, 0.14, 0.14, 0.12, -0.12 (half to even of 0.125, 0.135, 0.145, 0.115, -0.125) and a zero row for 0.005:

```text
$ spec/hledger/show-holdings.sh --csv --value=end,EUR spec/hledger/probes/hold-csv-round.journal
a1  EUR  0.12  2026-01-01
a2  EUR  0.14  2026-01-01
a3  EUR  0.14  2026-01-01
a4  EUR  0.12  2026-01-01
a5  EUR  -0.12  2026-01-01
a6    0  2026-01-01
anchor  EUR  1.00  2026-01-01
sink  EUR  -1.40  2026-01-01
$ spec/hledger/show-holdings.sh --value=end,EUR spec/hledger/probes/hold-csv-round.journal
a1: EUR 0.125
a2: EUR 0.135
a3: EUR 0.145
a4: EUR 0.115
a5: EUR -0.125
a6: EUR 0.005
anchor: EUR 1.00
sink: EUR -1.40
```

hold-csv-marks (`commodity 1.000,00 EUR`, `commodity $1,000.00`): the CSV writes `7900,00`, `1234567,89` for EUR, `1234567.89` for `$`, no groups.
The JSON numbers are plain; hold-sum (Y has no style) is written with 8 places in the CSV and ten in the JSON:

```text
$ spec/hledger/show-holdings.sh --csv spec/hledger/probes/hold-csv-marks.journal
a  EUR  7900,00  2026-01-01
b  EUR  1234567,89  2026-01-01
c  X  5  2026-01-01
d  $  1234567.89  2026-01-01
sink  $  -1234567.89  2026-01-01
sink  EUR  -1242467,89  2026-01-01
sink  X  -5  2026-01-01
$ spec/hledger/show-holdings.sh --csv --value=end,Y spec/hledger/probes/hold-sum.journal
a  Y  1.00000000  2026-01-10..2026-01-12
b  Y  -1.00000000  2026-01-10..2026-01-12
```

## Rule: account queries and the `--` before them

Plainledger writes `-- P1 ... Pn cur:C`. The terms that are not prefixed (`P1 ... Pn`) are account patterns: **regular expressions, case-insensitive, not
anchored, matched against the whole account name**, and several are **or**-ed (`assets` matches `Assets:Cash`, `liabilities:assets` and
`expenses:my-assets`; `^assets` only the first and `assets:bank`). `cur:C` is **and**-ed with them.

`--` is not hidden: hledger turns the `--` Plainledger passes after its own `--` into a **query term**, an account pattern `--`
that matches no account (unless an account name contains `--`). It is or-ed with `P1 ... Pn`, so with n >= 1 it changes nothing, but with **n = 0 and `cur:C` alone the whole
run gives no row** (and `-- not:s` finds only an account containing `--`). The harness mimics Plainledger by passing the same `--`;
the engine ignores all of this (it does no filtering).

### Probe

`probes/hold-query.journal`, `probes/hold-query-dashes.journal`

### Command and output

hold-query (accounts `Assets:Cash`, `assets:bank`, `liabilities:assets`, `equity:Opening`, `expenses:my-assets`; rows of each query, the balancing
account left out):

```text
-- assets => Assets:Cash assets:bank expenses:my-assets liabilities:assets
-- Assets => Assets:Cash assets:bank expenses:my-assets liabilities:assets
-- ^assets => Assets:Cash assets:bank
-- assets$ => expenses:my-assets liabilities:assets
-- ASSETS cash => Assets:Cash assets:bank expenses:my-assets liabilities:assets
-- assets:bank Cash => Assets:Cash assets:bank
-- expenses => expenses:my-assets
-- not:assets =>
-- cur:X assets => Assets:Cash assets:bank expenses:my-assets liabilities:assets
no "--": cur:X => 6 rows
"--" first: -- cur:X => 0 rows
```

(Made with `hledger --no-conf -f spec/hledger/probes/hold-query.journal balance --layout=tidy -O csv --real -- QUERY` and the account column.) hold-query-dashes (account `q--r`
and `s`): the account whose name has `--` is found by any query that is only account patterns, or only `cur:`; `not:s` finds it as well:

```text
-- cur:X on dashes => "q--r","X","1"
-- zzz on dashes => "q--r","X","1"
zzz (no --) on dashes => 0 rows
-- not:s on dashes => "q--r"
```

## What `holdings()` must do

Per (account, commodity C), over all the real postings of the ledger that count:

1. **Postings:** real only (`(a)` and `[a]` out). A posting counts if its own date (`date:` tag, else the transaction's date) is before `end`, or always without `end`. The secondary date is not used.
2. **`units`:** exact sum of the quantities of C, costs and lots ignored. A zero sum may be reported as zero; hledger omits it.
3. **`cost`:** `at_cost()` of each posting (conversion postings dropped, as for `print --cost`), summed per commodity, for the postings that held C.
   The result has any number of commodities, C itself when a posting had no cost.
4. **`value`:** the units valued by `value(E, V)`, with C = V unchanged and "no path" unchanged.
5. **`costValue`:** `cost` first, then each commodity of it valued by `value(E, V)`.
6. **E (valuation date):** `end - 1 day` when `end` is given; else the latest of all transaction dates, all posting dates (including virtual postings and `date:` tags) and all `P` dates that are not after `today`. This uses the whole journal, not the rows asked for, and `today` is an input.
7. **Rounding:** only once, on the final amount of a (account, C, V) position: half to even to ten places, ten places kept ([value-arithmetic](value-arithmetic.md)). No rounding per posting.
8. **No rows when nothing counts:** with `end` before every posting, `holdings()` returns nothing.
9. **Exactness:** compare with JSON, not the CSV; the CSV rounds half to even to the commodity's display precision (8 places with no style) and writes the decimal mark of the style.

## Gaps

- `today` was only observed on one day (2026-10-23): a `P` line dated today counts, one dated tomorrow does not. The rule is probed with the system clock; the 2099 probes keep their meaning.
- `-e D` forms other than `YYYY-MM-DD` (Plainledger's), `-b`, periods (`-p`), `--date2`, `-E`, `--depth`, `--tree` are not probed.
- Aliases and `apply account` rename accounts before any of this (see [aliases-and-parents](aliases-and-parents.md)); `include` is refused by ledgermill. Not re-probed here.
- Several files: the order of `P` lines on the same date is that of [value-prices](value-prices.md); the dates E takes are from all files.
- Lots (`{...}`) in the value and cost runs follow [value-arithmetic](value-arithmetic.md) and [cost](cost.md); only `cost-lots`'s cases are covered, none through `balance`.
- The harness (`compare.ts`) reads every probe of this directory. The `hold-*` journals use only forms the engine reads, but it was not rebuilt (no `pkg/`), so none was run through it.
- The CSV's decimal mark for a commodity whose journal mixes `7900,00` and `1 234,50` was not worked out (the CSV wrote a dot for EUR and a comma for X in one probe); it never matters for the exact value.
- Commodity names that hledger cannot quote (a name containing `"` or a newline) were not probed.
