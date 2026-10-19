# Decimal places

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

The quantity keeps exactly the decimal places written: `12.50` is mantissa 1250 with 2 places,
`12.5` is 125 with 1. Trailing zeros are not dropped and places are not aligned between postings.
`-0.00` is a plain zero with 2 places (no negative zero), `-0 EUR` is `0`.

Postings of one transaction may use different numbers of places and still balance when the exact
sum is zero (see [balancing](balancing.md)).

More than ten decimal places are rounded to ten (a second probe, below). ledgermill refuses numbers
with more than ten places, and numbers with more than 15 digits in total (see
[big-numbers](big-numbers.md)).

## Probe

`probes/decimal-places.journal`, `probes/decimal-places-rounded.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/decimal-places.journal
spec/hledger/show.sh spec/hledger/probes/decimal-places-rounded.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"places differ between postings","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",30,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",7,1]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"negative zero is zero, places are kept","comment":"","line":6,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",0,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",0,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"zeros in a commodity","comment":"","line":10,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",0,3]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",0,0]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"many digits","comment":"","line":14,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",12345678901234,7]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-12345678901234,7]]}]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"more than ten places are rounded to ten","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",11234567890,10]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-11234567890,10]]}]}
```
