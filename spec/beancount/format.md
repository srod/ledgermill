# Format: what whitespace Beancount ignores, and what bean-format aligns

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`;
`bean-format --version`: `Beancount 3.2.3`), 2026-10-24.

Status: **spec only**. `format()` returns a Beancount file unchanged until PLA-360 is built.

`format(file)` rewrites the whitespace of posting lines (and of `balance` and `price` lines) and nothing
else. The acceptance is that its output equals `bean-format`'s wherever `bean-format` only moves
whitespace and keeps the ledger. So this note records two things: what Beancount reads the same (the room
the formatter has), and what `bean-format` does, found by diffing its output (the target). `bean-format`
works on text with regular expressions and never parses the ledger, which is why most of its rules are
about the shape of a line.

## Rule: what Beancount reads the same

`bean-check` and `bean-query` give the same transactions, postings and prices when only these change:

- **Indent**: a posting indented by 1 space, 2, 4 or a tab is the same posting. Postings of one
  transaction may even have different indents. A posting must be indented: an account at column 0 is a
  syntax error (`probes/refused/format-transaction-ends.beancount`).
- **Separator between account and number**: two spaces, many spaces, a tab, two tabs, a space and a tab,
  and **a single space** all read the same. One blank is enough, because an account name has no blank
  in it.
- **Between number and currency**: one blank, several, a tab, or nothing at all (`12.50EUR`) read the
  same.
- **Trailing blanks** (spaces and tabs) after an amount, a comment, a price, a bare account or a header
  read the same.
- **CRLF** line endings read the same as LF. A missing final newline reads the same.
- **Gap between a posting flag and its account**: `! Account`, `!Account` and `!` or `*` followed by several
  blanks or tabs all give the same posting with the same flag; so does `*Account`
  (`probes/refused/format-flag-gap-star.beancount`, kept apart because ledgermill refuses a posting that
  starts with `*` straight before the account as a `number-expression` today).
- **Metadata lines** (`key: value`) belong to the entry above them, whatever their indent: deeper than the
  posting, the same as the posting, shallower, or tab-indented. A metadata line under a posting
  belongs to that posting; one before the first posting belongs to the transaction.
- **Comment lines** inside a transaction, indented by 1 space, 4 spaces, a tab or deeper than the postings,
  change nothing and do not end the transaction.
- **`balance` and `price` lines**: blanks (spaces, tabs, runs of them) between the date, the keyword, the
  account, the number, the tolerance and the currency all read the same; so does a missing blank between a
  `price`'s number and its currency.

Not the same, so never touched by a formatter:

- A **blank line, a line of only blanks, or a comment at column 0** ends the transaction. The indented
  posting after it is a syntax error (`probes/refused/format-transaction-ends.beancount`). Such lines
  are kept as written.
- A **byte order mark** at the start of the file is part of the first token: `bean-check` reports
  `Invalid token` on the first directive (`probes/refused/format-bom.beancount`). A formatter keeps
  the BOM and does not try to repair it.

## Rule: what bean-format does

`bean-format FILE` prints the file with some lines rewritten. It keeps the number of lines, so line N of
the output is line N of the input. It looks at each line alone (a pattern never reaches into the next
line), in three shapes. "Blank" below is a space or a tab; a character is a Unicode code point
(a tab counts 1).

**Shape 1, a posting line with an amount** (aligned and re-indented):

1. a run of blanks (the indent, at least one);
2. an account with no flag before it: a first component starting with an uppercase letter, then one or
   more `:`-separated components, each starting with an uppercase letter or a digit; every component
   continues with letters, digits and `-` (Unicode letters and digits count: `Édition`, `Ag2024`; `_`,
   `.`, `+` do not, a trailing `:` does not);
3. a run of blanks (one is enough);
4. a number: `+` or `-` optionally, then optional blanks, then one or more digits and commas, then
   optionally a `.` and digits (`5.` counts, `.5` does not); **or** one pair of parentheses holding two such
   numbers with one `+ - * /` between them and optional blanks around the operator
   (`(4 + 2.5)`, `(1*2)`, `( 1+2)`; not `(5)`, not `(1+2+3)`, not `-(2)`, not `4+2`, not `(1+2)*3`; a
   blank after `(` is allowed, one before `)` is not);
5. a run of blanks;
6. a currency: a capital letter followed by capitals, digits and `' . _ -`, or the `/…` form
   (`/NQ`), followed by something that is not a letter, a digit or `_` (so `EURx` is not a currency);
7. anything, kept as it is, including the spaces in it and trailing blanks.

**Shape 2, a dated line with an amount** (aligned): the first character is a digit; then any text without
`"` and without `;`, up to the first place where a run of blanks is followed by a number (as above), a run
of blanks and a currency. In a valid ledger this is a `balance` line (the number aligned is the tolerance
when there is a `~`, so `400.00 ~` belongs to the text before it) and a `price` line. A
`custom "x" 5 EUR` or `note … "5 EUR"` line has a `"` before its number, so it is not this shape; an
`open Assets:Bank EUR` has no number.

**Shape 3, an account line** (re-indented only): the indent, then an account start (an uppercase
letter, letters/digits/`-`, a `:`, then an uppercase letter or a digit), then anything. This covers
elided postings (`  Expenses:Food`), postings with only a comment, and every shape-1 line.

Not in any shape, so copied exactly (and not counted in any width): flagged postings (`! Account  5 EUR`,
`* …`), postings whose number is an expression other than one parenthesised `(a op b)`, a number with no
blank before the currency (`12.50EUR`), `.5`, comment lines, metadata lines (they have a lower-case key),
headers, `open`, `close`, `commodity`, `pad`, `note`, `event`,
`custom`, `document`, `query`, `option`, `plugin`, `include`, blank lines.

**The column.** Over all shape-1 and shape-2 lines of the **whole file** (not per transaction):

- `P` is the longest *prefix*, in characters: for a posting, the indent as written plus the account; for a
  dated line, the text before the blanks that precede the number.
- `N` is the widest number, in characters, as written (its sign and any blanks inside it count).

Each such line is rewritten as: the prefix padded with spaces to `P` characters, **2 spaces**, the number
right-aligned in `N` characters, **1 space**, then the currency and the rest of the line unchanged. So
the last character of every number is in column `P + 2 + N − 1` (counting from 0), and every currency
starts at `P + 2 + N + 1`. The blanks between prefix and number and between number and currency are
the only ones that change (`-w`, `-W` and `-c` replace `P`, `N` and the currency column; checked in
`probes/format-align-width-narrow.beancount`).

**The indent.** Every shape-1 and shape-3 line is re-indented to the same number of spaces: the indent
that occurs on most of those lines, a tab counting as 1 character, and **the larger indent when two
are tied**. Metadata lines and comment lines are not counted and keep their indent. A file indented with
tabs therefore comes out indented by one space; a file with more 4-space than 2-space lines comes out at 4.
The column is worked out with each line's **original** indent but the lines are written with the new one,
so a line whose original indent differs from the usual one ends up shifted by the difference.

**Everything else.** Whitespace inside the text after the currency, trailing blanks, blank lines,
whitespace-only lines and tabs elsewhere are kept. A byte order mark is kept (a first line that starts
with it is in no shape). Line endings are not kept: a CR or CRLF becomes LF, and a missing final newline
is added.

**Where bean-format falls short** (so ledgermill must not copy it):

- It **changes the ledger** when a line inside a multi-line string has one of the shapes: the text of the
  string (the narration) changes (`probes/format-align-string.beancount`).
- It **stops with an error** (an assertion of its own, no output) when an amount line holds a blank other
  than a space or a tab (a no-break space, a form feed, an ideographic space) next to its number
  (`probes/refused/format-nbsp-separator.beancount`). Beancount rejects such a line too (`Invalid token`).
  It also stops on a file that is not UTF-8.
- It is **not idempotent**. Running it on its own output can move the column: the first run works the
  column out from the original indents and writes the usual one, the second works it out from the written
  ones (`probes/format-align-idempotent.beancount`; 23 of the 400 random files below).
- A `balance` line with a tolerance is wide (`2026-02-10 balance Assets:Bank 400.00 ~` is 39 characters
  before the number), and as `P` is a maximum over the file it pushes every amount in the file to the
  right (`probes/format-align-kinds.beancount`).

**The model.** `format-model.py` is our implementation of the rules above in Python. Run on random journals
made of postings, flagged postings, `balance`, `price`, tolerances, metadata, comments, expressions,
tabs, Unicode names and random one-character edits, its output equals `bean-format`'s on every
file. Its variant for ledgermill (the column worked out from the new indent) is idempotent on every file,
and equals `bean-format` on every file whose lines all have one indent. Results are in Output.

## Probe

What Beancount reads the same: `probes/format-indent.beancount`, `probes/format-separator.beancount`,
`probes/format-trailing.beancount`, `probes/format-flag-gap.beancount`, `probes/format-metadata-indent.beancount`,
`probes/format-comment-indent.beancount`, `probes/format-directives.beancount`, `probes/format-crlf.beancount`,
`probes/format-no-final-newline.beancount`, `probes/refused/format-flag-gap-star.beancount`,
`probes/refused/format-transaction-ends.beancount`, `probes/refused/format-bom.beancount`.

What bean-format does: `probes/format-align-width.beancount`, `probes/format-align-width-narrow.beancount`,
`probes/format-align-width-lots.beancount`, `probes/format-align-kinds.beancount`,
`probes/format-align-indent.beancount`, `probes/format-align-indent-tie.beancount`,
`probes/format-align-indent-ignored.beancount`, `probes/format-align-unaligned.beancount`,
`probes/format-align-unaligned-width.beancount`, `probes/format-align-idempotent.beancount`,
`probes/format-align-string.beancount`, `probes/refused/format-align-expressions.beancount`,
`probes/refused/format-nbsp-separator.beancount`, and the probes of "what Beancount reads the same" again
(`format-separator`, `format-trailing`, `format-flag-gap`, `format-indent`, `format-metadata-indent`,
`format-comment-indent`, `format-directives`, `format-crlf`, `format-no-final-newline`, `refused/format-bom`).

## Command

What Beancount reads (the postings of each file: date, narration, account, number, currency, line):

```sh
spec/beancount/show-postings.sh spec/beancount/probes/format-indent.beancount | cut -d, -f1,4-7,9
# and the same for format-separator, format-trailing, format-comment-indent, format-crlf,
# format-no-final-newline
spec/beancount/show.sh spec/beancount/probes/format-flag-gap.beancount        # posting flags: 3rd column of the postings
# and the same for refused/format-flag-gap-star
spec/beancount/show-prices.sh spec/beancount/probes/format-directives.beancount
bean-check spec/beancount/probes/format-directives.beancount                  # the balance lines: no output
bean-query -f csv spec/beancount/probes/format-metadata-indent.beancount \
  "SELECT lineno, account, meta FROM postings" | sed -E "s#'filename': '[^']*', ##"
bean-query -f csv spec/beancount/probes/format-metadata-indent.beancount \
  "SELECT lineno, meta FROM entries WHERE type = 'transaction'" | grep -o "^[0-9]*\|'id': '[a-z0-9]*'" | paste -sd' ' -
bean-check spec/beancount/probes/refused/format-transaction-ends.beancount
spec/beancount/show-postings.sh spec/beancount/probes/refused/format-bom.beancount
```

What bean-format does (`show-format.sh` runs `bean-format`, prints a ruler, then the lines it changed with
their line numbers; `$` ends a line, `^I` is a tab):

```sh
spec/beancount/show-format.sh spec/beancount/probes/format-align-width.beancount   # and -narrow, -lots, -kinds,
# -indent, -indent-tie, -indent-ignored, -unaligned, -unaligned-width, -idempotent, -string,
# refused/format-align-expressions, format-separator, format-trailing, format-flag-gap, format-indent,
# format-metadata-indent, format-comment-indent, format-directives
BEAN_FORMAT_ARGS='-w 30 -W 12' spec/beancount/show-format.sh spec/beancount/probes/format-align-width-narrow.beancount
BEAN_FORMAT_ARGS='-c 40' spec/beancount/show-format.sh spec/beancount/probes/format-align-width-narrow.beancount
bean-format spec/beancount/probes/format-align-idempotent.beancount > /tmp/once && bean-format /tmp/once | diff /tmp/once -
grep -c "$(printf '\r')" spec/beancount/probes/format-crlf.beancount
bean-format spec/beancount/probes/format-crlf.beancount | grep -c "$(printf '\r')"
tail -c 6 spec/beancount/probes/format-no-final-newline.beancount | od -c
bean-format spec/beancount/probes/format-no-final-newline.beancount | tail -c 6 | od -c
bean-format spec/beancount/probes/refused/format-bom.beancount | head -c 6 | od -c
```

Whether it only moves whitespace and keeps the ledger (`format-same.sh` runs `bean-format` on a copy, compares
the text without whitespace, then `show-postings.sh` and `show-prices.sh` before and after):

```sh
spec/beancount/format-same.sh spec/beancount/probes/format-align-string.beancount spec/beancount/probes/format-align-kinds.beancount
spec/beancount/format-same.sh harness/journals/app/*.beancount spec/beancount/probes/*.beancount spec/beancount/probes/refused/*.beancount
```

The model against `bean-format` (needs `bean-format` on the PATH; 3000 random files take about 10 minutes):

```sh
spec/beancount/format-model.py 400 5
spec/beancount/format-model.py 3000 7
```

## Output

### What Beancount reads the same

The rows are `date,narration,account,number,currency,line`. The `-- bean-check` line has nothing after it: no
error.

format-indent: one space (line 5), four spaces (9), a tab (13), a tab and spaces mixed (17): the same
postings.

```text
2026-03-01,one space,Expenses:Food,12.50,EUR,5
2026-03-01,one space,Assets:Bank,-12.50,EUR,6
2026-03-02,four spaces,Expenses:Food,12.50,EUR,9
2026-03-02,four spaces,Assets:Bank,-12.50,EUR,10
2026-03-03,tab,Expenses:Food,12.50,EUR,13
2026-03-03,tab,Assets:Bank,-12.50,EUR,14
2026-03-04,mixed,Expenses:Food,12.50,EUR,17
2026-03-04,mixed,Assets:Bank,-12.50,EUR,18
```

format-separator: two spaces, many spaces, two tabs, one tab, **one space**, a space and a tab, blanks
between number and currency, **no blank between number and currency**: every pair gives 12.50 and -12.50 EUR.

```text
2026-03-01,two spaces,Expenses:Food,12.50,EUR,5
2026-03-02,many spaces,Expenses:Food,12.50,EUR,9
2026-03-03,two tabs,Expenses:Food,12.50,EUR,13
2026-03-04,one tab,Expenses:Food,12.50,EUR,17
2026-03-05,one space,Expenses:Food,12.50,EUR,21
2026-03-06,space and tab,Expenses:Food,12.50,EUR,25
2026-03-07,blanks between number and currency,Expenses:Food,12.50,EUR,29
2026-03-08,no blank between number and currency,Expenses:Food,12.50,EUR,33
```

format-trailing: blanks after an amount, a tab after one, blanks after a comment, after a header, after a bare
account, after a price.

```text
2026-03-01,blanks after the amount,Expenses:Food,12.50,EUR,5
2026-03-01,blanks after the amount,Assets:Bank,-12.50,EUR,6
2026-03-02,blanks after a comment and a bare account,Expenses:Food,12.50,EUR,9
2026-03-02,blanks after a comment and a bare account,Assets:Bank,-12.50,EUR,10
2026-03-03,blanks after a posting with a price,Expenses:Food,10,USD,13
2026-03-03,blanks after a posting with a price,Assets:Bank,-12.50,EUR,14
```

format-flag-gap (`show.sh`, the id shortened): the flag is the third column.

```text
id,Expenses:Food,!      <- "! Expenses:Food"
id,Assets:Bank,*        <- "* Assets:Bank"
id,Expenses:Food,!      <- "!Expenses:Food"
id,Assets:Bank,!        <- "!Assets:Bank"
id,Expenses:Food,!      <- "!    Expenses:Food"
id,Assets:Bank,*        <- "*<TAB><TAB>Assets:Bank"
```

refused/format-flag-gap-star: `*Expenses:Food` and `*Assets:Bank` give the flag `*` on both.

```text
id,Expenses:Food,*
id,Assets:Bank,*
```

format-metadata-indent: a metadata line belongs to the posting above it whether it is deeper (line 7), shallower (9) or at
the same indent as the posting (14), or tab-indented (20). The transaction's `id` (lines 5, 12, 18: 4 spaces, the
same as the postings, a tab) is read in all three: the last query lists the transactions' `id`. The row
numbers are those of the postings; `filename` is left out.

```text
6,Expenses:Food,"{'lineno': 6, 'note': 'deeper'}"
8,Assets:Bank,"{'lineno': 8, 'tag': 'shallower'}"
13,Expenses:Food,"{'lineno': 13, 'note': 'same indent'}"
15,Assets:Bank,"{'lineno': 15}"
19,Expenses:Food,"{'lineno': 19, 'note': 'tabs'}"
21,Assets:Bank,"{'lineno': 21}"
4 'id': 'a1'
11 'id': 'a2'
17 'id': 'a3'
```

format-comment-indent: comments at 1 space, 4 spaces, a tab and deeper, before the first posting, between and after
the postings: two postings, no error.

```text
2026-03-01,indented comments between and around the postings,Expenses:Food,12.50,EUR,6
2026-03-01,indented comments between and around the postings,Assets:Bank,-12.50,EUR,10
```

format-directives: the four `price` lines (spaces, runs of spaces, tabs, no blank before `EUR`) read as
0.9, 0.91, 0.92 and 0.93 EUR; the five `balance` lines (single spaces, runs, tabs, a tolerance with runs) are
accepted, `bean-check` prints nothing.

```text
-- prices: date,currency,number(amount),currency(amount)
2026-02-01,USD,0.9,EUR
2026-02-02,USD,0.91,EUR
2026-02-03,USD,0.92,EUR
2026-02-04,USD,0.93,EUR
```

format-crlf and format-no-final-newline: the same rows (the lines 5, 6, 9 and 10) as the LF file.

```text
2026-03-01,crlf,Expenses:Food,12.50,EUR,5
2026-03-01,crlf,Assets:Bank,-12.50,EUR,6
2026-03-02,narrower,Expenses:Food,3,EUR,9
2026-03-02,narrower,Assets:Bank,-3,EUR,10
```

refused/format-transaction-ends: a blank line (line 6), a line of blanks (11) and a comment at column 0 (16) each end
the transaction before the second posting; the posting after is a syntax error (7, 12, 17) and the transaction no
longer balances. An account at column 0 (20) is a syntax error too.

```text
format-transaction-ends.beancount:7: syntax error, unexpected INDENT
format-transaction-ends.beancount:12: syntax error, unexpected INDENT
format-transaction-ends.beancount:17: syntax error, unexpected INDENT
format-transaction-ends.beancount:20: syntax error, unexpected ACCOUNT
format-transaction-ends.beancount:4: Transaction does not balance: (12.50 EUR)
format-transaction-ends.beancount:9: Transaction does not balance: (12.50 EUR)
format-transaction-ends.beancount:14: Transaction does not balance: (12.50 EUR)
```

refused/format-bom: the byte order mark makes the first token invalid; the transactions still read.

```text
format-bom.beancount:1: Invalid token: '<U+FEFF>2026-01-01'
```

### What bean-format does

format-align-width: the longest account (`  Expenses:Food:Groceries`, 25 characters) is in one transaction, the widest number
(`-12,345.67`, 10) in the other. `P = 25`, `N = 10`: numbers end in column `25 + 2 + 10 - 1 = 36`
(from 0), currencies start in 38. Both transactions use it.

```text
    |0         1         2         3         4         5         6
    |0123456789012345678901234567890123456789012345678901234567890123
   6|  Expenses:Food:Groceries        4.50 EUR$
   7|  Assets:Bank                   -4.50 EUR$
  10|  Assets:Bank               12,345.67 EUR$
  11|  Income:Pay               -12,345.67 EUR$
```

format-align-width-narrow: `P = 14` (`  Expenses:Tea`), `N = 6` (`100.00`): numbers end in column 21, so `-99.5` gets one extra
space; line 6 was already there and is not listed. Right-aligned by the last character, not by the point.

```text
   7|  Assets:Bank    -99.5 EUR$
   8|  Assets:Cash     -0.5 EUR$
  11|  Expenses:Tea       7 EUR$
  12|  Assets:Bank       -7 EUR$
```

Same file with `-w 30 -W 12` (the numbers end in column `30 + 2 + 12 - 1 = 43`) and with `-c 40` (the currency in column 40
counting from 1, so index 39):

```text
   6|  Expenses:Tea                        100.00 EUR$      <- -w 30 -W 12
   7|  Assets:Bank                          -99.5 EUR$
   6|  Expenses:Tea                  100.00 EUR$            <- -c 40
   7|  Assets:Bank                    -99.5 EUR$
```

format-align-width-lots: the widest prefix is not a posting: `2026-03-03 balance Assets:Bank` is 30 characters, `N = 9`
(`-1,005.00`): numbers end in column 40. Units, costs, prices and the `balance` line all follow it; what comes after
the currency (`{100.5 EUR} @ 101.25 EUR`) is untouched.

```text
   6|  Assets:Broker                        10 VT {100.5 EUR}$
   7|  Assets:Bank                   -1,005.00 EUR$
  10|  Assets:Broker                       -10 VT {100.5 EUR} @ 101.25 EUR$
  11|  Assets:Bank                    1,012.50 EUR$
  12|  Income:Gains                      -7.50 EUR$
  14|2026-03-03 balance Assets:Bank       7.50 EUR$
```

format-align-kinds: every kind of line in one file (the tolerance line makes `P = 39`, `N = 9`). Changed: `price` lines
(also the one with a comment after), postings with plain, cost (`{…}`, `{{…}}`) and price (`@`) amounts, with a comment
after, with a `+`; both `balance` lines, the tolerance as the number on the second. Not listed, so kept: `option`,
`open`, `commodity` and its metadata line, comment lines, headers, the metadata line under a posting, the flagged
postings (23, 24), the elided postings, `pad`, `note`, `event`, `custom`, `close`.

```text
  13|2026-02-01 price VT                          100.5 EUR$
  14|2026-02-02 price  VT                        101.25 EUR  ; a comment after a price$
  18|  Assets:Bank                             1,500.00 EUR$
  20|  Income:Pay                             -1,500.00 EUR ; a comment after an amount$
  26|  Assets:Broker                                 10 VT {100.5 EUR}$
  27|  Assets:Broker                                  2 VT {{201 EUR}}$
  28|  Assets:Bank                               -1,206 EUR$
  30|  Assets:Broker                                 -1 VT {100.5 EUR} @ 101.25 EUR$
  31|  Assets:Bank                               101.25 EUR$
  34|  Expenses:Food                                6.5 EUR$
  35|  Expenses:Food                                 +3 EUR$
  38|2026-02-09 balance Assets:Bank              400.00 EUR$
  39|2026-02-10 balance Assets:Bank 400.00 ~       0.01 EUR$
```

format-align-indent: three account lines at 4 spaces (the elided posting, line 9, is one), two at 2 spaces, two with a
tab (1 character): 4 is the commonest, so every account line comes out at 4 spaces, the elided posting and
the tab lines included (17 is `Assets:Cash`, which had no amount).

```text
   8|    Expenses:Tea     3.5 EUR$
  12|    Expenses:Food      1 EUR$
  13|    Assets:Bank       -1 EUR$
  16|    Expenses:Tea       2 EUR$
  17|    Assets:Cash$
```

format-align-indent-tie: two lines at 2 spaces, two at 4: the larger wins.

```text
   7|    Expenses:Food   1 EUR$
   8|    Assets:Bank   -1 EUR$
  11|    Expenses:Tea   2 EUR$
  12|    Assets:Cash   -2 EUR$
```

format-align-indent-ignored: the metadata and comment lines at 6 spaces (two metadata, three comment lines) do not count
against the two postings at 2; the postings keep 2 and the others keep 6.

```text
   8|  Expenses:Food   1 EUR$
  13|  Assets:Bank    -1 EUR$
```

format-separator: every variant of the separator comes out as the same two spaces and right alignment;
`12.50EUR` (lines 33, 34) is not listed: it was not touched. format-trailing: the blanks after the
amount, the tab, the blanks after the comment and the price stay (`$` shows them). format-flag-gap: nothing is listed,
every line is as written. format-indent: the lines come out with one space (the tab counts as one
character and 1 is the commonest). format-metadata-indent and format-comment-indent: only posting lines are listed.

```text
format-separator            5|  Expenses:Food   12.50 EUR$      (the same for 6, 9, 10, 13, 14, 17, 18, 21, 22, 25, 26, 29, 30)
format-trailing             5|  Expenses:Food   12.50 EUR   $
                            6|  Assets:Bank    -12.50 EUR^I$
                            9|  Expenses:Food   12.50 EUR  ; note   $
                           13|  Expenses:Food      10 USD @ 1.25 EUR  $
format-indent               5| Expenses:Food      12.50 EUR$
                            6| Assets:Bank       -12.50 EUR$
format-metadata-indent      6|  Expenses:Food   12.50 EUR$
                            8|  Assets:Bank    -12.50 EUR$
```

format-directives: prefixes are kept as written, tabs and doubled blanks included, and measured in characters. With a
tolerance in the file, every amount ends in column 50.

```text
   7|2026-02-01 price USD                            0.9 EUR$
   9|2026-02-03^Iprice^IUSD                           0.92 EUR$
  13|  Expenses:Food                               12.50 EUR$
  18|2026-03-04  balance  Assets:Bank             -12.50 EUR$
  19|2026-03-05 balance Assets:Bank -12.50 ~        0.01 EUR$
  20|2026-03-06 balance Assets:Bank   -12.50   ~    0.01 EUR$
```

format-align-unaligned: the flagged postings (lines 7, 8) and the two `3.50EUR` lines (11, 12) are not listed, and
neither counts toward `P` or `N` (`12.50` and `-12.50` are wider than `-1`): the last transaction is aligned to its
own widths. format-align-unaligned-width: the long flagged account sets no column, nothing is listed.

```text
  15|  Expenses:Tea   1 EUR$
  16|  Assets:Cash   -1 EUR$
```

refused/format-align-expressions: after a wide plain number (`1,000,000.00`) that makes every aligned line end in
column 29. Aligned: `(4 + 2.5)` (line 9) and `- 5` (line 33), and the plain numbers next to the others. Not
aligned and not counted: `(4 + 2 + 0.5)` (13), `4+2` (17), `-(2)` (21), `(5)` (25), `(1 + 2) * 3` (29): those lines are as written (they
are not listed).

```text
   5|  Expenses:Food   1,000,000.00 EUR$
   6|  Assets:Bank    -1,000,000.00 EUR$
   9|  Expenses:Food      (4 + 2.5) EUR$
  10|  Assets:Bank             -6.5 EUR$
  14|  Assets:Bank             -6.5 EUR$
  18|  Assets:Bank               -6 EUR$
  22|  Assets:Bank                2 EUR$
  26|  Assets:Bank               -5 EUR$
  30|  Assets:Bank               -9 EUR$
  33|  Expenses:Food            - 5 EUR$
  34|  Assets:Bank                5 EUR$
```

format-align-idempotent: the second run moves the column (`diff` of the first output against the second):

```text
8,9c8,9
<   Expenses:Tea                2 EUR
<   Assets:Cash                -2 EUR
---
>   Expenses:Tea              2 EUR
>   Assets:Cash              -2 EUR
12,13c12,13
<   Expenses:Food:Groceries     3 EUR
<   Assets:Cash                -3 EUR
---
>   Expenses:Food:Groceries   3 EUR
>   Assets:Cash              -3 EUR
```

Line endings, final newline, BOM:

```text
CRs in the input: 10          (format-crlf)
CRs in the output: 0
0000000    -   3       E   U   R                       <- end of format-no-final-newline: no "\n"
0000000    3       E   U   R  \n                       <- bean-format adds one
0000000  357 273 277   2   0   2                       <- the BOM is kept (refused/format-bom)
```

Ledger and text kept (`format-same.sh`) over the 12 `harness/journals/app` journals and all probes
(196 files): 152 are changed, the rest are already aligned; none changes more than whitespace; the ledger
(postings, prices and the errors `bean-check` prints) is the same in all but one; `bean-format` fails on one.

```text
probes/format-align-kinds.beancount | whitespace only: yes | same ledger: yes | lines changed: 13
probes/format-align-string.beancount | whitespace only: yes | same ledger: NO | lines changed: 4
probes/refused/format-nbsp-separator.beancount | bean-format fails
```

The string probe: the narration has become

```text
"first line
  Assets:Bank        5 EUR
1                    2 EUR last line"
```

(it was `"first line⏎  Assets:Bank  5 EUR⏎1   2 EUR last line"`).

The model (`format-model.py N SEED`, mutated random journals; the first line of each run is the number of files, the
last the number of files on which `bean-format` is not idempotent):

```text
400 files (0 skipped: bean-format fails)
  bean_format() differs from bean-format: 0
  proposed() not idempotent: 0
  one-indent files: 187, proposed() differs from bean-format on 0
  bean-format not idempotent on 23
```

Our reader (the wasm, `bun harness/real.ts spec/beancount/probes/format-*.beancount`, one run per file) gives the same
transactions as the reference on all 20 `probes/format-*.beancount`, so they can stay in the corpus.

The same with 3000 files (`format-model.py 3000 7`; half of them with one indent everywhere):

```text
3000 files (0 skipped: bean-format fails)
  bean_format() differs from bean-format: 0
  proposed() not idempotent: 0
  one-indent files: 1405, proposed() differs from bean-format on 0
  bean-format not idempotent on 165
```

## The formatter's rules (ledgermill's own)

These make `format()` equal to `bean-format` wherever `bean-format` only moves whitespace and keeps the ledger
(and every file with one indent), and keep it whitespace-only, idempotent and ledger-preserving. Where the
choice was not obvious it is marked **Decision**; the owner settled all four on 2026-10-24 by taking the default
stated, the one that satisfies the acceptance (equality with `bean-format`).

1. **Which lines change.** A line is rewritten only if it has one of bean-format's three shapes (above) **and**
   ledgermill reads it as part of what it parsed: a posting of a transaction, a `balance` directive or a `price`
   directive. Everything else is kept byte for byte: headers, other directives and their metadata lines, comment
   lines (indented or not), blank and whitespace-only lines, `option`/`plugin`/`include` lines, flagged postings
   (shape 1 requires no flag), a posting whose number is an expression other than one `(a op b)` (ledgermill refuses
   number expressions today, `refused.md`, so such a file is not formatted at all, rule 8), and any line inside a
   multi-line string. (Reading the line as a posting is what keeps the lines inside a string out, which `bean-format` does not;
   rule 9 is the safety net.)
2. **Blanks** are spaces and tabs only. A line with another blank (no-break space, form feed, ideographic
   space) where a rule needs a blank is not in any shape and is kept (`bean-format` stops on such a file; Beancount
   rejects the line).
3. **The indent** `I` of the account lines (shape 1 and 3): the indent, in characters with a tab counting 1, that most
   of them have; **a tie goes to the larger**. Every account line is written with `I` spaces, then the account
   as written. Metadata and comment lines keep their indent.
   **Decision O1: the indent (settled: (a)).** (a) *`bean-format`'s* (above): keeps whatever the file mostly uses, equals
   `bean-format` on every file, never rewrites a consistent file's style; but a tab-indented file comes out with one
   space, and "postings indented 2 spaces" of the ticket holds only when the file mostly uses 2. (b) *Always 2 spaces*
   (as the hledger formatter always uses 4): matches the ticket's wording, no tie rule, tabs become 2; but it differs
   from `bean-format` on every file not mostly indented by 2 (it rewrites a 4-space file to 2), so the equality
   acceptance would then hold only on 2-indented files. Recommendation: (a), which is what the acceptance asks;
   (b) if the owner prefers the ticket's wording to the equality on other indents.
4. **The column.** Over the whole file (not per transaction), for the aligned lines (rule 1): `P` is the longest
   prefix in characters, **using the new indent `I`** for postings (`I` plus the account) and the text before the number for
   `balance` and `price` lines as written; `N` is the widest number in characters, as written (sign and inner blanks
   included). Width is the number of characters (a tab is 1; wide characters such as CJK or emoji will look
   misaligned, as in the hledger formatter and in `bean-format`). Using the new indent, not the original one, is the
   one deliberate difference from `bean-format` in the column: it makes the result idempotent, and gives
   `bean-format`'s text whenever all the account lines have one indent.
5. **An aligned line** is written as: the prefix padded with spaces to `P` characters, 2 spaces, the number
   right-aligned in `N` characters, 1 space, then the currency and the rest of the line exactly as written (inner
   spaces, comment, trailing blanks). The prefix of a posting is `I` spaces plus the account; the prefix of a `balance`
   or `price` line is its text as written up to the blanks before the number (blanks, tabs and doubled spaces
   in it kept). The number is the text as written (`1,500.00`, `+3`, `- 5`); notation is never touched.
   **Decision O2: the tolerance (settled: follow `bean-format`).** In `balance DATE ACCOUNT NUMBER ~ TOL CUR`, `bean-format` aligns `TOL`, so
   `NUMBER ~` belongs to the prefix and one such line makes `P` about 20 columns wider for every amount in
   the file (in `format-align-kinds`, 39 instead of 30). Following it keeps equality with `bean-format` (the default
   here). The alternative is to align the first number of a `balance` line like the others and keep `~ TOL CUR`
   as the tail: tidier files, but unequal to `bean-format` in any file with a tolerance. Recommendation: follow
   `bean-format`; tolerance lines are rare, and the harness can compare against `bean-format` only if we agree with it.
6. **Account-only lines** (no amount: elided postings, a posting with only a comment after the account) are re-indented to `I` and otherwise kept as written, trailing blanks included; they do not count in `P`.
   **Decision O3: flagged postings (settled: kept as written).** `bean-format` neither re-indents nor aligns `! Account  5 EUR` and
   `* Account  5 EUR`, and does not count them in `P` or `N`. Following it (the default here) gives equality, but a
   transaction with a flagged posting keeps that posting's own spacing and indent. The alternative is to treat a flagged
   posting like any other (indent `I`, flag and account as written, aligned and counted): consistent files, as in
   the hledger formatter, but unequal to `bean-format` on every file that has a flagged posting, and `P` and `N` of
   the whole file change.
7. **Line endings, BOM, final newline** are kept exactly: a line that ended in CR LF still does (the CR is not
   part of the line for any rule), a BOM stays (a first line that starts with it is in no shape, as in `bean-format`),
   a missing final newline stays missing. This is a deliberate difference from `bean-format`, which turns CR and CRLF
   into LF and adds a final newline; that is not alignment, so the equality with `bean-format` is checked after
   turning CRLF into LF and adding the final newline to our output. Lines never move or merge.
8. **Files ledgermill refuses.** If loading the file reports anything ledgermill refuses (`unsupported-syntax`: an
   `include`, a `plugin`, an `option` that changes names or tolerances, a number expression, a cost to merge, …) the
   file is returned unchanged: ledgermill then reads no transaction of the file, so the safety check (rule 9)
   compares two empty ledgers and proves nothing.
   **Decision O4: refused files (settled: returned unchanged).** The default is the safe one. The alternative is to format them anyway with
   the same text rules (as `bean-format` does): it equals `bean-format` on those files too, but nothing checks that
   the ledger is kept.
9. **Safety check.** The original and the formatted text are loaded under the same name. If their `transactions()`,
   `postings()`, `prices()` or `check()` differ (a finding's column and message are ignored, as a line's column moves),
   the original is returned. (It catches the string case if rule 1 misses it.) This is the same check as the hledger
   formatter's.
10. **Idempotent by construction.** `I`, `P` and `N` do not depend on the spacing the lines have: `I` is stable
    because all account lines are written with it, `P` and `N` are worked out from the account, prefix and number
    texts alone. Formatting the output gives the output (`bean-format` does not: 23 of the 400 random files).
11. **Whitespace only.** Only the blanks (a) before an account, (b) between an account and its number, (c) between
    a number and its currency, and (d) between a `balance`/`price` prefix and its number change. The text with all
    whitespace removed is the same before and after.

What stays open for later (not decided here): wide characters (display width); aligning `open`/`pad`/`custom` lines or
metadata (`bean-format` does not).
