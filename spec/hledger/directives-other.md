# Other directives

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

Read, and changing nothing ledgermill reports:

- `account NAME` (the name ends at two blanks; a `type:` tag is read, see
  [conversion](conversion.md)), `payee`, `tag`, `commodity SYMBOL` with indented `note` or other
  lines, `N`, `C`, `A`, `bucket`, `apply tag`, `apply fixed` and their `end` lines, `--`, `assert`,
  `capture`, `check`, `define`, `expr`, `eval`, `python`, `value`.
- `P DATE [HH:MM[:SS]] COMMODITY AMOUNT`: market prices. The line is fully checked (a bad date,
  time, symbol or amount is an error; `P` may be glued to the date) but changes nothing in raw
  output.
- `comment` ... `end comment`: everything between is ignored.

Directives that take no indented lines (`D`, `decimal-mark`, `P`, `Y`, `alias`) fail on one; the
others accept indented lines, and comments are always fine. An unknown word in column 0 is an
error. `include` is refused by ledgermill (its caller hands it the files). Periodic (`~`) and
automated (`=`) rules are not read yet: refused as `directive`.

## Probe

`probes/directive-ignored.journal`, `probes/directive-comment-block.journal`, `probes/directive-price-invalid.journal`, `probes/directive-price-no-amount.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/directive-ignored.journal
spec/hledger/show.sh spec/hledger/probes/directive-comment-block.journal
hledger --no-conf -f spec/hledger/probes/directive-price-invalid.journal print -O json
hledger --no-conf -f spec/hledger/probes/directive-price-no-amount.journal print -O json
```

## Output

directive-ignored:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":11,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",-1,0]]}]}
```

directive-comment-block:

```text
{"date":"2026-01-02","status":"Unmarked","code":"","description":"read","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

directive-price-invalid:

```text
hledger: Error: spec/hledger/probes/directive-price-invalid.journal:1:13:
  |
1 | P 2026-01-01X 2 EUR
  |             ^
unexpected 'X'
expecting digit
(exit status 1)
```

directive-price-no-amount:

```text
hledger: Error: spec/hledger/probes/directive-price-no-amount.journal:1:15:
  |
1 | P 2026-01-01 X
  |               ^
unexpected newline
(exit status 1)
```
