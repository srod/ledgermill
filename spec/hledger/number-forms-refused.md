# Number forms hledger accepts that ledgermill refuses

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

hledger accepts a leading plus sign (`+3`), a number with no integer digits (`.5`), a trailing
point (`5.`), leading zeros (`007.50`, read as 7.50) and a space between the minus sign and the
number (`- 3 EUR`). They give ordinary quantities, but they are rare and they interact with
display styles, so ledgermill answers `unsupported-syntax` (construct `amount`) instead of
accepting them in this slice.

Also refused: digit grouping, `,` as decimal mark, exponents (`1e2`), quoted commodities,
prices, balance assertions and anything after the amount other than a comment.

## Probe

`probes/number-forms-accepted-by-hledger.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/number-forms-accepted-by-hledger.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"plus sign","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-3,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"no leading digit","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",5,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-5,1]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"trailing point","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",5,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-5,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"leading zeros","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",750,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-75,1]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"space after the minus sign","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",-3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]}]}
```
