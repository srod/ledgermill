# Format: what whitespace hledger ignores in a posting line

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-24.

`format(file)` rewrites the whitespace of posting lines and nothing else. It may do so only where the
reference tool reads the same transactions before and after. This note records what the tool ignores,
then states the rules the formatter follows.

## Rule: what hledger reads the same

- **Indent**: a posting indented by 1 space, 4 spaces or a tab is the same posting.
- **Separator between account and amount**: two spaces, many spaces, two tabs, or a space and a tab
  all end the account and give the same amount. **A lone tab is not a separator**: it is part of the
  account name (`expenses:food<TAB>12.50 EUR` is an account of that name, and the amount is
  elided). The formatter therefore never turns a tab into a separator or the other way round: it
  works from where ledgermill (which follows this) ends the account.
- **Trailing blanks** (spaces and tabs) at the end of a posting line, after an amount, after a comment
  or after a bare account, change nothing (a comment is reported trimmed).
- **CRLF** line endings read the same as LF, whatever the columns.
- **Status and brackets**: a `*` or `!` before the account, and `()` or `[]` around it, are read
  (`spec/hledger/postings.md`). The formatter keeps all of them, and any spaces between them, as
  written, so the gap between a status and its account needs no rule of its own (it reads the same
  with 0, 1 or 4 spaces: `probes/format-status-gap.journal`).

## Probe

`probes/format-indent.journal`, `probes/format-separator.journal`,
`probes/format-separator-lone-tab.journal`, `probes/format-trailing.journal`,
`probes/format-crlf.journal`, `probes/format-status.journal`, `probes/format-status-gap.journal`

Command (the postings of each transaction; its line is the first number):

```sh
spec/hledger/show.sh spec/hledger/probes/format-indent.journal | jq -c '[.line, .postings]'
# and the same for format-separator, format-separator-lone-tab, format-trailing, format-crlf
spec/hledger/show.sh spec/hledger/probes/format-status-gap.journal
```

## Output

The `"type":"RegularPosting",` field is left out below to shorten the lines.

format-indent: one space (line 1), four spaces (5), a tab (9): the same postings.

```text
[1,[{"account":"expenses:food","comment":"","amounts":[["",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["",-1250,2]]}]]
[5,[{"account":"expenses:food","comment":"","amounts":[["",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["",-1250,2]]}]]
[9,[{"account":"expenses:food","comment":"","amounts":[["",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["",-1250,2]]}]]
```

format-separator: two spaces (1), many spaces (5), two tabs (9), a space and a tab (13): the same postings.

```text
[1,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
[5,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
[9,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
[13,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
```

format-separator-lone-tab: a single tab is part of the name; the posting got its amount by elision.

```text
[1,[{"account":"expenses:food 12.50 EUR","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
```

format-trailing: blanks after the amount, after a comment, after a bare account, a trailing tab.

```text
[1,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"expenses:tea","comment":"with a comment\n","amounts":[["EUR",3,0]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1550,2]]}]]
[6,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
```

format-crlf: the same postings with CRLF endings and different columns (comment `note`).

```text
[1,[{"account":"expenses:food","comment":"","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
[5,[{"account":"expenses:food","comment":"note\n","amounts":[["EUR",1250,2]]},{"account":"assets:cash","comment":"","amounts":[["EUR",-1250,2]]}]]
```

format-status-gap (full `show.sh` lines, shortened here to the amounts): `*    expenses:food`,
`* expenses:tea` (line 1) and `*expenses:food`, `*expenses:tea` (line 6) all give accounts
`expenses:food` and `expenses:tea` with the amounts 12.50 EUR and 3 EUR, and `assets:cash` -15.50 EUR.

```text
{"date":"2026-03-06","status":"Unmarked",...,"line":1,"postings":[{"account":"expenses:food",...,"amounts":[["EUR",1250,2]]},{"account":"expenses:tea",...,"amounts":[["EUR",3,0]]},{"account":"assets:cash",...,"amounts":[["EUR",-1550,2]]}]}
{"date":"2026-03-06","status":"Unmarked",...,"line":6,"postings":[{"account":"expenses:food",...,"amounts":[["EUR",1250,2]]},{"account":"expenses:tea",...,"amounts":[["EUR",3,0]]},{"account":"assets:cash",...,"amounts":[["EUR",-1550,2]]}]}
```

## The formatter's rules (ledgermill's own)

1. **Which lines change.** Only the posting lines of the transactions ledgermill reads from the file.
   Everything else is kept byte for byte: transaction headers, directives and their indented lines,
   comment lines (indented or not), `comment … end comment` blocks, `=` and `~` rule blocks, blank
   lines, `P` lines, and any transaction ledgermill refuses or drops. (A Beancount file is formatted by
   its own rules, `spec/beancount/format.md`.)
2. **A rewritten posting line** is 4 spaces, then the posting as written from its first non-blank
   character to the end of its account (status, brackets and the spaces between them as written);
   then, if it has an amount, padding of at least 2 spaces and the rest of the line from the first
   character of the amount, as written, without trailing spaces and tabs. A posting with no amount
   only gets the new indent, its rest kept as written without trailing spaces and tabs.
3. **The amount** is the quantity with its commodity as written (`-12,50 €`, `EUR 1 003,11`,
   `"AB 1" 3`), up to where ledgermill's amount reader stops: a cost (`@`, `@@`, `(@)`, `(@@)`), a lot
   (`{…}`, `[DATE]`, `(LABEL)`), a valuation expression, a balance assertion or assignment, or a
   comment comes after it and is kept as written. Number notation is never touched.
4. **An assignment-only posting** (`a  = 5`) has no amount. It is not aligned: it gets the new indent
   and the rest of its line stays as written. (Aligning the `=` would be possible, but then
   `a  = 5` and `a  1 = 5` would differ in the column they are put at, for no reader's benefit.)
5. **The column of a transaction** is the largest `4 + width(status and account) + 2 + width(amount)`
   over its postings that have an amount. Every such posting is padded so that the end of its amount is
   on that column, so the amounts end in one column. Width is the number of characters (a shortcut:
   wide characters such as CJK or emoji will look misaligned).
6. **Line endings, BOM, final newline** are kept exactly as they are: a posting line that ended in CR
   LF still does, a BOM stays, a missing final newline stays missing. Lines never move.
7. **Safety check.** The original and the formatted text are loaded under the same name. If their
   `transactions()`, `postings()`, `prices()` or `check()` differ (a finding's column and message
   are ignored, as a posting line's column moves), the original is returned.
8. **Idempotent by construction.** The widths used for the column do not depend on the original
   spacing, so formatting the output gives the output.
