# Balance assertions: the syntax

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

A posting may end with a balance assertion. After the account and, if there is one, the amount:

| Written | Kind | JSON flags (`pbalanceassertion`) |
|---|---|---|
| `= AMOUNT` | one commodity, the account alone | `batotal` false, `bainclusive` false |
| `== AMOUNT` | all commodities ("total"), the account alone | `batotal` true |
| `=* AMOUNT` | one commodity, the account and its sub-accounts | `bainclusive` true |
| `==* AMOUNT` | all commodities, with sub-accounts | both true |

What the assertion means is in [assertions](assertions.md). A posting with an assertion and no
amount is an assignment ([assignments](assignments.md)).

- **Spacing.** Blanks around the `=` signs are free (none, one, many, tabs), and nothing is
  needed between the amount and the `=`. The `*` must touch the equals signs (`=*`, `==*`);
  `= * 1 EUR` is a parse error. After the assertion, blanks and a `;` comment (with or without a
  space) are fine.
- **Before the `=`**, the posting-line rule still holds: the account ends at two blanks in a
  row. `b = 1 EUR` (one space) is an account named `b = 1 EUR` with no amount, not an assertion.
  A tab followed by a space is two blanks and does separate.
- **The asserted amount** is read like any amount ([amount-forms](amount-forms.md),
  [numbers](numbers.md)): commodity on either side, quoted commodity, sign before or after the
  commodity, `+`, exponents, digit groups, the file's `decimal-mark` / `commodity` / `D` rules for
  a lone `,` or `.`. A number with no commodity takes the `D` commodity if the file has one.
  It may carry a price, `@ P` or `@@ T`; the price is kept and never used by the check
  ([assertions](assertions.md)). It may not carry a lot (`{...}`, `{{...}}`, `[date]`, `(label)`):
  parse error.
- **Which postings.** Regular, `(virtual)` and `[balanced virtual]` postings, with or without a
  status mark, may carry one assertion each.
- **Parse errors** are reported on the posting's line: nothing after the `=`s; a `*` apart from
  the equals signs; three `=`; a second `=` or any text after the amount (other than a comment);
  a lot in the amount.
- `print -O json` keeps the assertion on the posting as `pbalanceassertion`: `baamount` (the
  amount with its price), `batotal`, `bainclusive`, and `baposition` with the line and the column
  of the first `=` (the column counts a tab as moving to the next multiple of 8). The harness and
  ledgermill's `Transaction` do not carry it.

## Probe

`probes/assert-forms.journal`, `probes/assert-single-space.journal`,
`probes/assert-syntax-star-space.journal`, `probes/assert-syntax-empty.journal`,
`probes/assert-syntax-lot.journal`, `probes/assert-syntax-two.journal`,
`probes/assert-syntax-junk.journal`, `probes/assert-syntax-three-equals.journal`

Command:

```sh
spec/hledger/show-assertions.sh spec/hledger/probes/assert-forms.journal
spec/hledger/show.sh spec/hledger/probes/assert-single-space.journal
spec/hledger/assertion-verdicts.sh spec/hledger/probes/assert-syntax-*.journal
hledger --no-conf -f spec/hledger/probes/assert-syntax-star-space.journal print
```

## Output

assert-forms (excerpt: lines 7-10 spacing, 12 tab, 17-22 the four kinds with and without a blank
after the `*`, 33-35 the amounts, 39-40 `*`/`!` marks):

```text
{"at":"7:11","account":"p","asserted":["EUR",1,0],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"8:12","account":"p","asserted":["EUR",1,0],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"9:11","account":"p","asserted":["EUR",1,0],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"17:12","account":"p","asserted":["EUR",1,0],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"18:12","account":"p","asserted":["EUR",1,0],"total":true,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"19:12","account":"p","asserted":["EUR",3,0],"total":false,"inclusive":true,"amounts":[["EUR",0,0]]}
{"at":"20:12","account":"p","asserted":["EUR",3,0],"total":true,"inclusive":true,"amounts":[["EUR",0,0]]}
{"at":"21:12","account":"p","asserted":["EUR",3,0],"total":false,"inclusive":true,"amounts":[["EUR",0,0]]}
{"at":"22:12","account":"p","asserted":["EUR",3,0],"total":true,"inclusive":true,"amounts":[["EUR",0,0]]}
{"at":"33:12","account":"p","asserted":["EUR",10,1],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"34:12","account":"p","asserted":["EUR",1,0,["@","USD",5,1]],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
{"at":"35:12","account":"p","asserted":["EUR",1,0,["@@","USD",7,0]],"total":false,"inclusive":false,"amounts":[["EUR",0,0]]}
```

assert-single-space (the second posting is an account named `b = 1 EUR`, which got the balancing
amount):

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b = 1 EUR","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]}]}
```

The syntax errors (`FILE:LINE:COLUMN`, the first error):

```text
assert-syntax-empty | rejects | assert-syntax-empty.journal:3:14 | parse
assert-syntax-junk | rejects | assert-syntax-junk.journal:2:20 | parse
assert-syntax-lot | rejects | assert-syntax-lot.journal:4:16 | parse
assert-syntax-star-space | rejects | assert-syntax-star-space.journal:3:15 | parse
assert-syntax-three-equals | rejects | assert-syntax-three-equals.journal:2:14 | parse
assert-syntax-two | rejects | assert-syntax-two.journal:2:20 | parse
```

```text
hledger: Error: …/assert-syntax-star-space.journal:3:15:
  |
3 |   b  -1 EUR = * 1 EUR
  |               ^
unexpected '*'
expecting amount (for a balance assertion or assignment)
```
