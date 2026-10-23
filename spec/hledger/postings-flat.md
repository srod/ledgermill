# Flat postings: `print --location -O csv`

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-23.

Status: built as `Ledger::postings()` (hledger); the harness compares it with `print --location -O csv`.

`postings()` for hledger answers what `hledger -f F1 -f F2 ... print --location -O csv` lists: one row
per posting **and commodity**. Each row becomes a `FlatPosting`. This note says what the CSV gives, how
the rows are ordered, what a row's file and header line are, and what hledger does not give (a posting's
own line, which is ours to define).

The rows below are made by `spec/hledger/show-postings.sh`, which runs that command and prints one line per
row: `IDX DATE STATUS | "DESCRIPTION" | FILE:FIRST-LAST | ACCOUNT | AMOUNT COMMODITY`, then
`| pstatus=S`, `| pcomment=TEXT` and `| comment=TEXT` (the transaction comment without its location line)
when they are not empty. `IDX` is the `txnidx` column. The directory `spec/hledger/probes/` is cut from the
file names. Quoted commodities are shown without their quotes (that is what the CSV holds).

## Rule: the columns

The CSV has a header line and the columns `txnidx, date, date2, status, code, description, comment, account,
amount, commodity, credit, debit, posting-status, posting-comment`. The ones `postings()` needs:

| column | holds |
|---|---|
| `txnidx` | the transaction's number, from 1, in **parse order** (files in the order given, top to bottom); a transaction with no posting takes a number and gives no row, so numbers can skip. Every row of a transaction has the same number, so it identifies the transaction in the CSV |
| `date` | the transaction's date (`YYYY-MM-DD`). Not the secondary date (`date2`), not a posting's `date:` tag |
| `status` | the transaction's status: empty, `!` or `*` |
| `description` | everything after the status and the code, up to a `;` comment, as hledger reads it (a `\|` is kept: there is no payee split) |
| `comment` | the transaction's comment (header and indented comment lines, joined by newlines) **followed by one more line, `location: FILE:FIRST-LAST`**; see below |
| `account` | the account with its brackets: `(a)` for a virtual posting, `[a]` for a balanced virtual one, `a` otherwise |
| `amount` | the quantity of **this row's** commodity, as a plain decimal: no commodity, no digit groups |
| `commodity` | the commodity of the row, without quotes; empty for a bare number |
| `posting-status` | the posting's own mark: empty, `!` or `*` |

`credit`, `debit` repeat the amount by sign; ledgermill does not read them. `posting-comment` holds the posting's comment lines.

The CSV is standard: fields with a comma, a quote or a newline are quoted and a quote inside is doubled, so
the `comment` field spans several lines when the transaction has comment lines. Read it with a real CSV reader.

### Probe

`probes/post-basic.journal`, `probes/post-dates.journal`

### Command and output

post-basic (a margin comment line ends a transaction; an indented one after the postings belongs to it; the
`pending` header has a comment of its own that starts with `location:`; the header-only transaction at line 20 gives
no row and takes number 4):

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-basic.journal
2 2026-01-03 | "earlier date, written later" | post-basic.journal:11-13 | a | 1 X
2 2026-01-03 | "earlier date, written later" | post-basic.journal:11-13 | b | -1 X
3 2026-01-03 | "same date, written after" | post-basic.journal:15-17 | a | 2 X
3 2026-01-03 | "same date, written after" | post-basic.journal:15-17 | b | -2 X
5 2026-01-04 ! | "pending" | post-basic.journal:22-24 | a | 4 X | comment=location: this text is the author's, not hledger's
5 2026-01-04 ! | "pending" | post-basic.journal:22-24 | b | -4 X | comment=location: this text is the author's, not hledger's
1 2026-01-05 * | "Shop | bought things" | post-basic.journal:3-8 | expenses:food | 10.50 EUR | comment=note on header\ncomment line inside the transaction
1 2026-01-05 * | "Shop | bought things" | post-basic.journal:3-8 | expenses:misc | 3 USD | pcomment=posting comment | comment=note on header\ncomment line inside the transaction
1 2026-01-05 * | "Shop | bought things" | post-basic.journal:3-8 | assets:cash | -10.50 EUR | pstatus=! | pcomment=trailing indented comment belongs to the transaction | comment=note on header\ncomment line inside the transaction
1 2026-01-05 * | "Shop | bought things" | post-basic.journal:3-8 | assets:cash | -3 USD | pstatus=! | pcomment=trailing indented comment belongs to the transaction | comment=note on header\ncomment line inside the transaction
```

In the `comment` field the location line is always the **last** line. The author's own `location: ...` comment is
kept before it (index 5), so the location is read from the last line, never the first that looks like one.

post-dates (secondary date and posting dates have no effect on `date` nor on the order):

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-dates.journal
2 2026-01-05 | "written second, sorts first" | post-dates.journal:5-7 | a | 1 X
2 2026-01-05 | "written second, sorts first" | post-dates.journal:5-7 | b | -1 X
1 2026-01-10 | "secondary date and posting dates" | post-dates.journal:2-4 | a | 1 X | pcomment=date:2026-01-20
1 2026-01-10 | "secondary date and posting dates" | post-dates.journal:2-4 | b | -1 X | pcomment=date:2026-01-01
```

## Rule: the file and the header line range

`location: FILE:FIRST-LAST` in the `comment` field:

- `FIRST` is the transaction's header line, counted from 1 in its file, blank and comment lines included
  (the same as `tsourcepos[0].sourceLine`, [source-lines](source-lines.md)).
- `LAST` is the last line **of the transaction**: its last posting line or the last **indented** comment line
  after it. Blank lines and margin comment lines after the transaction are not part of it. It is
  `tsourcepos[1].sourceLine - 1` (the JSON end is one past). A transaction with a comment line between its
  postings has that line inside the range. (Post-basic: header on 3, indented comment on 4, postings on 5 to 7, a trailing
  indented comment on 8, then a blank: `3-8`.)
- Parse the location by cutting at the **last** `:` and then splitting the span at `-`: a file name can hold `:`,
  spaces and digits (probed with `sp ace:d-1/a b:3-4.journal`).
- `FILE` is the path hledger resolved, not the name given. A relative name is joined to the working directory
  (with its symlinks resolved, `/tmp` shows as `/private/tmp` on macOS) **without normalizing it**: `./a.journal`
  stays `DIR/./a.journal` and `../x/a.journal` stays `DIR/../x/a.journal`. An absolute name is kept as given. A
  symlink to a file is not followed. So for the file name `N` that ledgermill was handed, the CSV's file is `N`
  itself when `N` is absolute, else ends with `/` + `N`. A harness must match like this; the engine reports `N`.
- The same file given twice is read twice: its transactions come out twice, with different numbers.
- CRLF line endings, a missing final newline and a BOM change none of this.

### Probe

`probes/post-crlf.journal` (CRLF everywhere, no newline at the end of the file), `probes/post-files-a.journal` and
`probes/post-files-b.journal`

### Command and output

post-crlf (the carriage returns are not in any field, line numbers count `\n` only; the last posting is on line 8
without a newline after it):

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-crlf.journal
1 2026-01-01 | "crlf" | post-crlf.journal:1-4 | a | 1 X | pcomment=posting comment\nsecond comment line | comment=header comment
1 2026-01-01 | "crlf" | post-crlf.journal:1-4 | b | -1 X | comment=header comment
2 2026-01-02 | "crlf, no trailing newline" | post-crlf.journal:6-8 | a | 2 X
2 2026-01-02 | "crlf, no trailing newline" | post-crlf.journal:6-8 | b | -2 X
```

Two files (`post-files-b` has a header-only transaction `b2` on line 5, which takes number 4 and gives no row):

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-files-a.journal spec/hledger/probes/post-files-b.journal
2 2026-01-01 | "a2 earlier" | post-files-a.journal:4-6 | p | 2 X
2 2026-01-01 | "a2 earlier" | post-files-a.journal:4-6 | q | -2 X
1 2026-01-02 | "a1" | post-files-a.journal:1-3 | p | 1 X
1 2026-01-02 | "a1" | post-files-a.journal:1-3 | q | -1 X
3 2026-01-02 | "b1 same date as a1" | post-files-b.journal:1-3 | p | 3 X
3 2026-01-02 | "b1 same date as a1" | post-files-b.journal:1-3 | q | -3 X
5 2026-01-03 | "b3" | post-files-b.journal:7-9 | p | 5 X
5 2026-01-03 | "b3" | post-files-b.journal:7-9 | q | -5 X
```

## Rule: the order of rows

1. Transactions are sorted by `date`; transactions of the same date keep parse order (files in the order
   given, then top to bottom). A transaction written late with an early date comes first. This is the order of
   `print` ([ordering](ordering.md)); the CSV adds nothing to it. In the output above `a2` (2026-01-01, written
   second in file a) comes before `a1`; `a1` (file a) comes before `b1` (file b) on the same date, 2026-01-02.
2. Within a transaction, postings come **in the order written**. An elided posting stays where it is written.
3. Within a posting that has several commodities (below), rows are in **commodity-name order**: code point order,
   the empty commodity first (`$`, `E F`, `Z`, `a`, `b`, `É`, `€`).

A file given twice is parsed twice and parse order breaks ties: with `b a b`, the transactions of 2026-01-02 come out `b1` (number 1), `a1` (4), `b1` (6).

### Probe

`probes/post-files-a.journal`, `probes/post-files-b.journal` (read as `b a b` below)

### Command and output

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-files-b.journal spec/hledger/probes/post-files-a.journal spec/hledger/probes/post-files-b.journal
5 2026-01-01 | "a2 earlier" | post-files-a.journal:4-6 | p | 2 X
1 2026-01-02 | "b1 same date as a1" | post-files-b.journal:1-3 | p | 3 X
4 2026-01-02 | "a1" | post-files-a.journal:1-3 | p | 1 X
6 2026-01-02 | "b1 same date as a1" | post-files-b.journal:1-3 | p | 3 X
3 2026-01-03 | "b3" | post-files-b.journal:7-9 | p | 5 X
8 2026-01-03 | "b3" | post-files-b.journal:7-9 | p | 5 X
```

(Only the `p` posting of each transaction is shown.)

## Rule: one row per posting and commodity

A posting written with an amount gives **one row**, the amount's quantity and commodity, whatever cost, lot
or assertion follows the amount (below). A posting without an amount gets its amounts from balancing
([elided-amounts](elided-amounts.md)) and gives **one row per commodity** of what it was given:

- the negated sum of the other postings of its group, one amount per commodity;
- commodities whose sum is zero are **dropped** when another commodity is not zero;
- when **every** amount is zero (or when there are none) the posting gives **one row**, quantity `0`: its commodity is
  the first named commodity in code point order (the bare `""` is skipped when a named commodity exists), else empty
  (`e 0 A` in 2026-01-03, `c 0` in 2026-01-06);
- a bare number is a commodity too: it sorts first and is a row of its own (`c -1` before `c -3 B`).

An assignment (`= 5 X`, `== 0`) is a posting with the amount hledger worked out: `==` over two commodities gives two
rows (negated balance), a difference of zero gives one row `0` with no commodity (`n 0` in 2026-01-09, `n 0` in
2026-01-10 whose partner `m` gets `0` too).
Rows keep the places of the amounts: a sum has the places of its terms ([balancing](balancing.md)); `10 X @ 2.5 EUR`
leaves `-25.0 EUR` for the elided posting (the precise rule is in the next section).

### Probe

`probes/post-elided.journal`, `probes/post-costs.journal`

### Command and output

post-elided (rows 1 to 6, and 11, 12 of the twelve transactions; 7 to 10 are assignments, listed after):

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-elided.journal
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | a | 1 b
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | b | 2 Z
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | c | 3 a
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | d | 4 $
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | e | 5 €
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | f | 6 E F
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | g | 7 É
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -4 $
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -6 E F
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -2 Z
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -3 a
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -1 b
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -7 É
1 2026-01-01 | "three commodities" | post-elided.journal:2-10 | h | -5 €
2 2026-01-02 | "elided posting in the middle" | post-elided.journal:11-15 | a | 4 X
2 2026-01-02 | "elided posting in the middle" | post-elided.journal:11-15 | b | -4 X
2 2026-01-02 | "elided posting in the middle" | post-elided.journal:11-15 | b | -7 Y
2 2026-01-02 | "elided posting in the middle" | post-elided.journal:11-15 | b | 9 Z
2 2026-01-02 | "elided posting in the middle" | post-elided.journal:11-15 | c | -9 Z
2 2026-01-02 | "elided posting in the middle" | post-elided.journal:11-15 | d | 7 Y
3 2026-01-03 | "every commodity nets to zero" | post-elided.journal:17-22 | a | 1 B
3 2026-01-03 | "every commodity nets to zero" | post-elided.journal:17-22 | b | -1 B
3 2026-01-03 | "every commodity nets to zero" | post-elided.journal:17-22 | c | 1 A
3 2026-01-03 | "every commodity nets to zero" | post-elided.journal:17-22 | d | -1 A
3 2026-01-03 | "every commodity nets to zero" | post-elided.journal:17-22 | e | 0 A
4 2026-01-04 | "zeros are left out when another amount is not zero" | post-elided.journal:23-29 | a | 1 B
4 2026-01-04 | "zeros are left out when another amount is not zero" | post-elided.journal:23-29 | b | -1 B
4 2026-01-04 | "zeros are left out when another amount is not zero" | post-elided.journal:23-29 | c | 2 A
4 2026-01-04 | "zeros are left out when another amount is not zero" | post-elided.journal:23-29 | d | 3 C
4 2026-01-04 | "zeros are left out when another amount is not zero" | post-elided.journal:23-29 | e | -3 C
4 2026-01-04 | "zeros are left out when another amount is not zero" | post-elided.journal:23-29 | f | -2 A
5 2026-01-05 | "a commodity-less zero" | post-elided.journal:30-35 | a | 1
5 2026-01-05 | "a commodity-less zero" | post-elided.journal:30-35 | b | -1
5 2026-01-05 | "a commodity-less zero" | post-elided.journal:30-35 | c | 1 Z
5 2026-01-05 | "a commodity-less zero" | post-elided.journal:30-35 | d | -1 Z
5 2026-01-05 | "a commodity-less zero" | post-elided.journal:30-35 | e | 0 Z
6 2026-01-06 | "only the commodity-less amount" | post-elided.journal:36-39 | a | 1
6 2026-01-06 | "only the commodity-less amount" | post-elided.journal:36-39 | b | -1
6 2026-01-06 | "only the commodity-less amount" | post-elided.journal:36-39 | c | 0
11 2026-01-11 | "a bare number and a named commodity" | post-elided.journal:55-58 | a | 1
11 2026-01-11 | "a bare number and a named commodity" | post-elided.journal:55-58 | b | 3 B
11 2026-01-11 | "a bare number and a named commodity" | post-elided.journal:55-58 | c | -1
11 2026-01-11 | "a bare number and a named commodity" | post-elided.journal:55-58 | c | -3 B
12 2026-01-12 | "zeros of a bare number and a named commodity" | post-elided.journal:59-64 | a | 1
12 2026-01-12 | "zeros of a bare number and a named commodity" | post-elided.journal:59-64 | b | -1
12 2026-01-12 | "zeros of a bare number and a named commodity" | post-elided.journal:59-64 | c | 1 B
12 2026-01-12 | "zeros of a bare number and a named commodity" | post-elided.journal:59-64 | d | -1 B
12 2026-01-12 | "zeros of a bare number and a named commodity" | post-elided.journal:59-64 | e | 0 B
```

and the assignments:

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-elided.journal
7 2026-01-07 | "holdings to assign over" | post-elided.journal:41-44 | n | 5 X
7 2026-01-07 | "holdings to assign over" | post-elided.journal:41-44 | n | 7 Y
7 2026-01-07 | "holdings to assign over" | post-elided.journal:41-44 | q | -5 X
7 2026-01-07 | "holdings to assign over" | post-elided.journal:41-44 | q | -7 Y
8 2026-01-08 | "total assignment zeroes two commodities" | post-elided.journal:45-47 | n | -5 X
8 2026-01-08 | "total assignment zeroes two commodities" | post-elided.journal:45-47 | n | -7 Y
8 2026-01-08 | "total assignment zeroes two commodities" | post-elided.journal:45-47 | q | 5 X
8 2026-01-08 | "total assignment zeroes two commodities" | post-elided.journal:45-47 | q | 7 Y
9 2026-01-09 | "a difference of zero leaves an empty amount" | post-elided.journal:48-50 | n | 0
9 2026-01-09 | "a difference of zero leaves an empty amount" | post-elided.journal:48-50 | m | 0 X
10 2026-01-10 | "and so does an elided posting after it" | post-elided.journal:51-53 | n | 0
10 2026-01-10 | "and so does an elided posting after it" | post-elided.journal:51-53 | m | 0
```

## Rule: the places of an amount

The CSV prints the quantity with **at least the display precision** of its amount, not only the places the quantity has in
`print -O json` (`decimalPlaces`). For an amount written with a commodity both are the places written. They differ for an
amount worked out by balancing, and for an amount written without a commodity under a `D` directive.

An amount written **without a commodity** under `D` takes its commodity and is listed with at least the places of `D`'s
sample (`5` under `D 1,000.00 EUR` is `5.00`; `5.123` stays; `5 EUR` is not padded). A price written without a commodity
(`@ 2`) is padded the same way, which shows in the elided amount it makes.

An amount worked out by balancing: the JSON quantity keeps the sum's own places (with the zero quirk of
[elided-amounts](elided-amounts.md)), while the precision is that of the amounts the sum was made of, and the CSV pads to it.
The row shows `max(places of the quantity, precision)` places. A zero is always `0`.

- a sum of plain amounts: the **most** places among the terms, whatever order and zeros there are (`1.25`, `-1.25`, `3`
  leave `-3.00`, though the quantity is `-3`);
- a unit price (`@`): the places of the **first** price of the group of equal prices (`3 Q @ 1.5 R` with `2 Q @ 1.50 R`
  leaves `-7.5 R`; alone, `10 Z @ 2.50 W` leaves `-25.00 W`); the places of the quantity count too;
- a total price (`@@`): the places of the **first** total (`1.50` then `1.5` leave `-3.00`; `1.0` then `1.00` leave `-2.0`);
- the sums of the groups into one commodity: the most places of the groups (a cost of `2` and a total of `1.25` into one
  commodity leave two places);
- the precision is never taken from another transaction (`0.001 R` elsewhere changes nothing);
- an amount worked out by an assignment (`p = X`): the places of the difference, but at least the precision of the asserted
  amount and of the balance (the most places among the amounts that made it): `3 EUR` held and `= 0.00 EUR` give `-3.00`
  though the quantity is `-3`; `0.00 EUR` held and `= 3 EUR` give `3.00`. The probe is
  `probes/assign-diff-places.journal`; the CSV rows of its second transaction (line 5) are
  `p1 -3.00 EUR` and `z 3.00 EUR`.

### Probe

`probes/post-elided-places.journal`, `probes/post-default-commodity.journal`

### Command and output

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-elided-places.journal
1 2026-01-01 | "zero in the middle of the sum" | post-elided-places.journal:3-7 | a | 1.25 C
1 2026-01-01 | "zero in the middle of the sum" | post-elided-places.journal:3-7 | b | -1.25 C
1 2026-01-01 | "zero in the middle of the sum" | post-elided-places.journal:3-7 | c | 3 C
1 2026-01-01 | "zero in the middle of the sum" | post-elided-places.journal:3-7 | d | -3.00 C
2 2026-01-02 | "whole term after" | post-elided-places.journal:9-13 | a | 1.25 C
2 2026-01-02 | "whole term after" | post-elided-places.journal:9-13 | b | 3 C
2 2026-01-02 | "whole term after" | post-elided-places.journal:9-13 | c | -1.25 C
2 2026-01-02 | "whole term after" | post-elided-places.journal:9-13 | d | -3.00 C
3 2026-01-03 | "unit prices" | post-elided-places.journal:15-17 | a | 10 Z
3 2026-01-03 | "unit prices" | post-elided-places.journal:15-17 | b | -25.00 W
4 2026-01-04 | "unit prices grouped, the first price gives the places" | post-elided-places.journal:19-22 | a | 3 Q
4 2026-01-04 | "unit prices grouped, the first price gives the places" | post-elided-places.journal:19-22 | b | 2 Q
4 2026-01-04 | "unit prices grouped, the first price gives the places" | post-elided-places.journal:19-22 | c | -7.5 R
5 2026-01-05 | "totals, the first gives the places" | post-elided-places.journal:24-27 | a | 1 X
5 2026-01-05 | "totals, the first gives the places" | post-elided-places.journal:24-27 | b | 1 X
5 2026-01-05 | "totals, the first gives the places" | post-elided-places.journal:24-27 | c | -3.00 Y
6 2026-01-06 | "another place for R elsewhere changes nothing" | post-elided-places.journal:29-31 | f | 0.001 R
6 2026-01-06 | "another place for R elsewhere changes nothing" | post-elided-places.journal:29-31 | g | -0.001 R
7 2026-01-07 | "an assignment lists the places of the difference" | post-elided-places.journal:33-35 | n1 | 2.00 X
7 2026-01-07 | "an assignment lists the places of the difference" | post-elided-places.journal:33-35 | b | -2.00 X
8 2026-01-08 | "and so does a difference from a balance" | post-elided-places.journal:37-39 | n2 | 7.5 X
8 2026-01-08 | "and so does a difference from a balance" | post-elided-places.journal:37-39 | b | -7.5 X
9 2026-01-09 | "second" | post-elided-places.journal:41-43 | n2 | 1.5 X
9 2026-01-09 | "second" | post-elided-places.journal:41-43 | b | -1.5 X
10 2026-01-10 | "total assignment" | post-elided-places.journal:45-47 | n2 | -9 X
10 2026-01-10 | "total assignment" | post-elided-places.journal:45-47 | b | 9 X
11 2026-01-11 | "totals, the first gives the places (not the most)" | post-elided-places.journal:49-52 | a | 3 X
11 2026-01-11 | "totals, the first gives the places (not the most)" | post-elided-places.journal:49-52 | b | 1 X
11 2026-01-11 | "totals, the first gives the places (not the most)" | post-elided-places.journal:49-52 | c | -2.0 EUR
```

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-default-commodity.journal
1 2026-01-01 | "bare amount" | post-default-commodity.journal:4-6 | a | 5.00 EUR
1 2026-01-01 | "bare amount" | post-default-commodity.journal:4-6 | b | -5.00 EUR
2 2026-01-02 | "bare amount with more places" | post-default-commodity.journal:8-10 | a | 5.123 EUR
2 2026-01-02 | "bare amount with more places" | post-default-commodity.journal:8-10 | b | -5.123 EUR
3 2026-01-03 | "named amount" | post-default-commodity.journal:12-14 | a | 5 EUR
3 2026-01-03 | "named amount" | post-default-commodity.journal:12-14 | b | -5 EUR
4 2026-01-04 | "bare amount, group mark" | post-default-commodity.journal:16-18 | a | 1000.00 EUR
4 2026-01-04 | "bare amount, group mark" | post-default-commodity.journal:16-18 | b | -1000.00 EUR
5 2026-01-05 | "bare amount in a cost" | post-default-commodity.journal:20-22 | a | 3 X
5 2026-01-05 | "bare amount in a cost" | post-default-commodity.journal:20-22 | b | -6.00 EUR
```

## Rule: costs, lots and assertions are not in the row

`amount` and `commodity` are the **units**: `10 X @ 2 EUR`, `10 X @@ 25 EUR`, `10 X {3 EUR}`,
`10 X {{35 EUR}} [2026-01-01] (lab)` and `10 X {3 EUR} @ 4 EUR` all give `10`, `X`. The cost is not in
any column. An elided posting that balances a costed one holds the **cost** commodity (`-25.0 EUR`), as hledger
worked it out, like an elided posting of any group. An inferred price (`10 X` against `-30 EUR`) changes nothing in the rows.
A balance assertion (`= 1 X`) or assignment is not in the row either (the assigned amount is, above).

post-costs:

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-costs.journal
1 2026-01-01 | "unit cost" | post-costs.journal:2-4 | a | 10 X
1 2026-01-01 | "unit cost" | post-costs.journal:2-4 | b | -20 EUR
2 2026-01-02 | "total cost" | post-costs.journal:5-7 | a | 10 X
2 2026-01-02 | "total cost" | post-costs.journal:5-7 | b | -25 EUR
3 2026-01-03 | "lot cost" | post-costs.journal:8-10 | a | 10 X
3 2026-01-03 | "lot cost" | post-costs.journal:8-10 | b | -30 EUR
4 2026-01-04 | "total lot, date and label" | post-costs.journal:11-13 | a | 10 X
4 2026-01-04 | "total lot, date and label" | post-costs.journal:11-13 | b | -35 EUR
5 2026-01-05 | "lot and price" | post-costs.journal:14-16 | a | 10 X
5 2026-01-05 | "lot and price" | post-costs.journal:14-16 | b | -40 EUR
6 2026-01-06 | "the elided posting of a costed one holds the cost commodity" | post-costs.journal:17-19 | a | 10 X
6 2026-01-06 | "the elided posting of a costed one holds the cost commodity" | post-costs.journal:17-19 | b | -25.0 EUR
7 2026-01-07 | "an inferred price" | post-costs.journal:20-22 | a | 10 X
7 2026-01-07 | "an inferred price" | post-costs.journal:20-22 | b | -30 EUR
```

## Rule: virtual postings, status

- `(a)` and `[a]` keep their brackets in the `account` column. The kind of the posting follows from them: both
  brackets, `(` and `)` → virtual, `[` and `]` → balanced virtual, anything else regular. The JSON `paccount` holds the name
  without brackets and `ptype` the kind; the CSV account is `(` + `paccount` + `)` for `VirtualPosting`, `[` + ... + `]` for
  `BalancedVirtualPosting`, and `paccount` otherwise. The kind is read **once the name is cut at two blanks**, as in
  [postings](postings.md).
- A name that starts with a bracket but does not end with one is regular (`(a) b`, `[c] d`), and so is one that
  ends with one but does not start with one (`a (b)`, `x[y]`). `(a) (b)` is virtual, named `a) (b`.
- Nested brackets are stripped from the name, the outer pair gives the kind: `(((g)))` → `(g)`, `([h])` → `(h)`,
  `[(i)]` → `[i]`, `()` → `()` (the name is empty). The exact rule is in the next section.
- Virtual postings are rows like the others, in written order, with the same amount rules (a virtual posting with no amount
  is an elided `0`: `(nothing) 0`, `[bnothing] 0 Y` takes `Y`, the commodity of its group).
- `status` is the transaction's; the posting's own mark is in `posting-status`. They are independent (`! pending`
  transaction, `* c` posting).

### Probe

`probes/post-virtual.journal`

### Command and output

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-virtual.journal
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | a | 1 X
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | b | -1 X
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | (v) | 5 X
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | [bv1] | 2 Y
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | [bv2] | -2 Y
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | c | 3 Z | pstatus=*
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | d | -3 Z | pstatus=!
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | (nothing) | 0
1 2026-01-01 ! | "pending" | post-virtual.journal:1-10 | [bnothing] | 0 Y
2 2026-01-02 * | "cleared, virtual postings only" | post-virtual.journal:11-13 | (v) | 5 X
2 2026-01-02 * | "cleared, virtual postings only" | post-virtual.journal:11-13 | (w) | 0
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | (a) b | 1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | a (b) | 1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | (a) (b) | 1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | [c] d | -1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | (e) | -1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | x[y] | -1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | (f) | -1 X
3 2026-01-03 | "names that look like brackets" | post-virtual.journal:14-22 | () | 1 X
4 2026-01-04 | "nested brackets are all stripped, the outer pair gives the kind" | post-virtual.journal:23-27 | (g) | 1 X
4 2026-01-04 | "nested brackets are all stripped, the outer pair gives the kind" | post-virtual.journal:23-27 | (h) | 1 X
4 2026-01-04 | "nested brackets are all stripped, the outer pair gives the kind" | post-virtual.journal:23-27 | [i] | 2 X
4 2026-01-04 | "nested brackets are all stripped, the outer pair gives the kind" | post-virtual.journal:23-27 | [j] | -2 X
```

## Rule: the name inside the brackets

The kind comes from the outer pair, which must **match**: `(`…`)` or `[`…`]`. The name is what is inside, and then,
**as long as** the name is itself wrapped in a matching pair (`(`…`)` or `[`…`]`, whichever kind), that pair is stripped
too. Only the match of the two ends counts: nothing is looked for inside the name, and a name starting with a blank is
not wrapped (the blank is kept).

- `(a])` → `a]`, `([a)` → `[a`, `((a)` → `(a`, `(a))` → `a)`: one pair stripped, the rest is not a matching pair;
- `((f))`, `(([a]))`, `([(a)])` → `f`, `a`, `a` and `[[(a)]]` → `a` (balanced): every level goes;
- `([a))` → `[a)`, `([(]))` → `[(])`, `(]a[)` → `]a[`: the inside is not a matching pair (`[`…`)`), so it stays;
- `( (a) )` → ` (a) `, `([ a ])` → ` a `: blanks inside are kept and stop the stripping;
- `([])`, `(())` → the empty name; `(()` → `(`; `())` → `)`; `[)(]` → `)(`;
- `[(a])`, `[(a)`, `(a)]` are regular (the ends do not match) and keep their text.

A virtual account's CSV text is `(` + name + `)`, so `(((g)))` shows as `(g)`, as above.

### Probe

`probes/post-virtual-nested.journal` (26 transactions, one case each; `zr` and `[zb]` are elided so they balance)

### Command and output

```text
$ hledger --no-conf -f spec/hledger/probes/post-virtual-nested.journal print -O json | jq -r '.[] | "\(.tpostings[0].ptype) \(.tpostings[0].paccount)"'
```

in the order of the file (`ptype` shortened to V virtual, B balanced virtual, R regular; the name between `|`):

```text
V |a]|   V |[a|   V |(a|   V |a)|   B |a)|   B |[a|   B |a]|   V |a|     V |a|
V |[a)|  V |([a|  V |]a[|  V |[a])x|  R |[(a])|  R |[(a)|  R |(a)]|  B |a|
V |[(])| V | (a) | V | a |  V |() | V ||     V ||     V |(|     V |)|     B |)(|
```

## Rule: the quantity and the style

- `amount` is the exact quantity with the places written (or worked out): `10.50`, `-12.50`, `0.0012` for `1.2e-3`,
  `1000` for `1E3`, `7` for `+7`, `1000` for `1 000`. No digit groups, no plus sign, no exponent. Zero keeps no
  places: `0.00 EUR`, `-0 EUR`, `-0.0` and `0` all give `0`.
- It is **exact**, not rounded to ten places as `print -O json` is: `0.1234567890123456`,
  `0.000000000000000000000001` and `123456789012345678901234567890.5` come out whole, and an elided
  amount is the exact difference (`-0.00000000001`). So read the quantity from the CSV, not from the JSON.
- The decimal mark is the **commodity's style mark**: a comma for a commodity declared `commodity 1.000,00 EUR`
  (`1234,5678 EUR`) or after `decimal-mark ,` (every commodity, and bare numbers), a point otherwise. The style is that of
  the commodity's directive, else of its first amount in the files ([directives-numbers](directives-numbers.md)), so a
  second amount written in the other style is printed in the first's: `2,25 X` after `1.5 X` is `2.25`. The engine must
  take the mark of the style when it writes the number, or parse it back. The places are not changed by the style (`commodity 1.000,00 EUR` does not pad `1234,5678`).
- A bare number has an empty `commodity`.

### Probe

`probes/post-zero-and-bare.journal`, `probes/post-styles.journal`, `probes/post-decimal-mark.journal`

### Command and output

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-zero-and-bare.journal
1 2026-01-01 | "zeros" | post-zero-and-bare.journal:2-8 | a | 0 EUR
1 2026-01-01 | "zeros" | post-zero-and-bare.journal:2-8 | b | 0 EUR
1 2026-01-01 | "zeros" | post-zero-and-bare.journal:2-8 | c | 0 EUR
1 2026-01-01 | "zeros" | post-zero-and-bare.journal:2-8 | d | 0
1 2026-01-01 | "zeros" | post-zero-and-bare.journal:2-8 | e | 0
1 2026-01-01 | "zeros" | post-zero-and-bare.journal:2-8 | f | 0 EUR
2 2026-01-02 | "no commodity" | post-zero-and-bare.journal:9-11 | a | 5
2 2026-01-02 | "no commodity" | post-zero-and-bare.journal:9-11 | b | -5
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (a) | 1000 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (b) | 1234567.89 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (c) | 0.0012 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (d) | 123456789012345678901234567890.5 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (e) | 0.000000000000000000000001 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (f) | -12.50 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (g) | 7 X
3 2026-01-03 | "number forms" | post-zero-and-bare.journal:12-20 | (h) | 1000 X
4 2026-01-04 | "places of an elided posting" | post-zero-and-bare.journal:21-24 | a | 1.5 X
4 2026-01-04 | "places of an elided posting" | post-zero-and-bare.journal:21-24 | b | -1.25 X
4 2026-01-04 | "places of an elided posting" | post-zero-and-bare.journal:21-24 | c | -0.25 X
5 2026-01-05 | "an elided posting with more than ten places" | post-zero-and-bare.journal:25-27 | a | 0.00000000001 X
5 2026-01-05 | "an elided posting with more than ten places" | post-zero-and-bare.journal:25-27 | b | -0.00000000001 X
```

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-styles.journal
1 2026-01-01 | "declared styles" | post-styles.journal:4-10 | (a) | 1234,5678 EUR
1 2026-01-01 | "declared styles" | post-styles.journal:4-10 | (b) | 1234.5678 USD
1 2026-01-01 | "declared styles" | post-styles.journal:4-10 | (c) | 1234.5678 CHF
1 2026-01-01 | "declared styles" | post-styles.journal:4-10 | (d) | 12345678.1 CHF
1 2026-01-01 | "declared styles" | post-styles.journal:4-10 | (e) | 3,5 EUR
1 2026-01-01 | "declared styles" | post-styles.journal:4-10 | (f) | 1000000 USD
2 2026-01-02 | "the style of a commodity without a directive comes from its first amount" | post-styles.journal:11-16 | (a) | 1.5 X
2 2026-01-02 | "the style of a commodity without a directive comes from its first amount" | post-styles.journal:11-16 | (b) | 2.25 X
2 2026-01-02 | "the style of a commodity without a directive comes from its first amount" | post-styles.journal:11-16 | (c) | 1234.50 X
2 2026-01-02 | "the style of a commodity without a directive comes from its first amount" | post-styles.journal:11-16 | (d) | 1,5 GBP
2 2026-01-02 | "the style of a commodity without a directive comes from its first amount" | post-styles.journal:11-16 | (e) | 2,5 GBP
```

```text
$ spec/hledger/show-postings.sh spec/hledger/probes/post-decimal-mark.journal
1 2026-01-01 | "comma everywhere" | post-decimal-mark.journal:2-6 | (a) | 1234,5 X
1 2026-01-01 | "comma everywhere" | post-decimal-mark.journal:2-6 | (b) | 2,5
1 2026-01-01 | "comma everywhere" | post-decimal-mark.journal:2-6 | (c) | 3,25 EUR
1 2026-01-01 | "comma everywhere" | post-decimal-mark.journal:2-6 | (d) | 4
```

## Rule: the posting's own line is ours

hledger gives **no source position for a posting** anywhere: the CSV has only the transaction's range, and in
`print -O json` the posting object has no position field (its keys are `paccount, pamount, pbalanceassertion,
pcomment, pdate, pdate2, poriginal, pstatus, ptags, ptransaction_, ptype`; `tsourcepos` is the transaction's header
and its end). The one exception is a posting with a balance assertion or assignment: `pbalanceassertion.baposition`
gives the line (and the column of the `=`) of the assertion, which is the line of the posting.

So `FlatPosting.line` is defined by ledgermill: **the line, counted from 1 in the file, on which the posting is
written** (the line of its account, or of its status mark before it). Comment lines and blank lines inside a transaction are not
postings and have no line. All the rows of a posting with several commodities have the same line. The line is
always inside `FIRST..=LAST` of the transaction, and a transaction's first posting is after its header line.

What the harness can check against hledger: the posting belongs to the transaction whose header is at
`FIRST` in `FILE` (rows grouped by `txnidx`), is the n-th posting of it, and its line lies in `FIRST+1..=LAST`; for a posting with an assertion,
the line equals `baposition.sourceLine`.

### Probe

`probes/post-assertion-line.journal`

### Command and output

The postings are on lines 3, 5 and 6 (`b`'s line, 5, follows an indented comment on 4 and has no assertion);
header on 2, the transaction ends at 6 (`tsourcepos` end is 7):

```text
$ hledger --no-conf -f spec/hledger/probes/post-assertion-line.journal print -O json | jq -c '.[0] | [.tsourcepos[0].sourceLine, .tsourcepos[1].sourceLine, (.tpostings[] | [.paccount, .pbalanceassertion.baposition.sourceLine])]'
[2,7,["a",3],["b",null],["c",6]]
```

## How a row becomes a `FlatPosting`

| `FlatPosting` | from the CSV |
|---|---|
| `date` | `date` |
| `flag` | `status` (`!`, `*`; empty for an unmarked transaction) |
| `payee` | none (hledger has no payee) |
| `narration` | `description` |
| `account` | `account` without the brackets; the kind (regular, virtual, balanced virtual) from them |
| `number`, `currency` | `amount`, `commodity` |
| `file` | the file name as given to `load` (the CSV's resolved path matches it, above) |
| `line` | the posting's own line (ours) |

## Gaps

- The posting's line cannot be read from hledger except through an assertion; the check is by range and
  position, as above.
- The CSV's `amount` for numbers of more than 255 places or exponents beyond a few hundred digits was not probed
  (the exact value comes out for 30 places and 30 integer digits).
- `flag` for an unmarked transaction is the empty string, as in the CSV (owner's decision); `!` and `*` otherwise.
- Included files (`include`) are refused by ledgermill; the CSV's file name would be the included file's.
- The `comment` field (`description` with a `;` in it, tags) and `posting-comment`, `date2`, `code` are not part of
  `FlatPosting` and were looked at only in passing.
