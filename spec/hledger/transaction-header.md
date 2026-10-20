# Transaction header

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

`DATE [STATUS] [DESCRIPTION] [; COMMENT]`

- The date is reported as `YYYY-MM-DD` (more forms in [dates](dates.md)); years may be 0 or have
  five digits. An impossible date (`2026-02-30`, month 13) is an error; `2024-02-29` is fine.
- Status is `*` (reported `Cleared`) or `!` (`Pending`), alone or followed by a space, otherwise
  none (`Unmarked`). Any number of spaces may separate the parts.
- The description is the rest of the line up to the first `;`, without leading or trailing
  spaces; inner runs of spaces are kept. It may be empty, and may hold `|`, `=`, quotes, accents.
- A `;` anywhere ends the description, with or without a space before it. The text after it is the
  transaction comment.
- A transaction may have no postings at all; `postings` is then empty.

A code `(x)` may follow the status, a second date `DATE=DATE2` the date, and other date forms are
accepted (see [dates](dates.md)). A status needs no space after it (`!x` is Pending, `x`). A status
glued to the date (`2026-01-03!x`) is an error. Neither code nor secondary date is reported.

## Probe

`probes/header-forms.journal`, `probes/header-unusual-accepted-by-hledger.journal`,
`probes/header-status-glued-to-date.journal`, `probes/date-invalid.journal`,
`probes/date-leap.journal`, `probes/date-year-zero-and-five-digits.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/header-forms.journal
spec/hledger/show.sh spec/hledger/probes/header-unusual-accepted-by-hledger.journal
hledger -f spec/hledger/probes/header-status-glued-to-date.journal print -O json
hledger -f spec/hledger/probes/date-invalid.journal print -O json
spec/hledger/show.sh spec/hledger/probes/date-leap.journal
spec/hledger/show.sh spec/hledger/probes/date-year-zero-and-five-digits.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"plain","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Cleared","code":"","description":"cleared","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-03","status":"Pending","code":"","description":"pending","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-04","status":"Cleared","code":"","description":"extra   spaces   inside","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"Coffee","comment":"memo glued\n","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"Pay | note","comment":"comment with trailing blanks\n","line":21,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-07","status":"Unmarked","code":"","description":"","comment":"","line":25,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-08","status":"Cleared","code":"","description":"","comment":"","line":29,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-09","status":"Cleared","code":"","description":"","comment":"only a comment\n","line":33,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-10","status":"Unmarked","code":"","description":"Café € déjà-vu","comment":"","line":37,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-11","status":"Unmarked","code":"","description":"a=b (not a code) and \"quotes\"","comment":"","line":41,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-12","status":"Unmarked","code":"","description":"no postings at all","comment":"","line":45,"postings":[]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"code","description":"description","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"secondary date","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-03","status":"Pending","code":"","description":"glued after a space","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"unpadded","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
hledger: Error: spec/hledger/probes/header-status-glued-to-date.journal:1:11:
  |
1 | 2026-01-03!glued status
  |           ^
(exit status 1)
```
```text
hledger: Error: spec/hledger/probes/date-invalid.journal:1:1:
  |
1 | 2026-02-30 t
  | ^^^^^^^^^^
(exit status 1)
```
```text
{"date":"2024-02-29","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
{"date":"0000-01-01","status":"Unmarked","code":"","description":"zero","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"99999-01-01","status":"Unmarked","code":"","description":"five digits","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
