# Dates inside posting comments

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

hledger reads dates in posting comments (never in transaction comments) and rejects the journal
when one is invalid:

- `date:` and `date2:` tags (found as described in [conversion](conversion.md), `type:` tags):
  the value must start with a date; text after it is fine. A date without a year takes the
  transaction's year.
- Brackets holding only date characters (digits, `-`, `/`, `.`, `=`) and a separator:
  `[DATE]`, `[DATE=DATE]`, `[=DATE]`. Other brackets (`[x]`, `[2026]`) are text.

Valid dates change nothing that ledgermill reports.

## Probe

`probes/comment-dates.journal`, `probes/comment-date-invalid.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/comment-dates.journal
hledger --no-conf -f spec/hledger/probes/comment-date-invalid.journal print -O json
```

## Output

comment-dates:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"date:2026-02-01, [2026-02-03] and [x]\n[=02-05]\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"date:02-01\n","amounts":[["",-1,0]]}]}
```

comment-date-invalid:

```text
hledger: Error: spec/hledger/probes/comment-date-invalid.journal:2:19:
  |
2 |   a  1  ; x, date:2026-02-30
  |                   ^^^^^^^^^^
This is not a valid date, please fix it.
(exit status 1)
```
