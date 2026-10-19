# Big numbers

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

When a mantissa is very large, hledger's JSON writes `decimalMantissa` in floating notation
(`1.2345E+39`), which cannot be read back exactly. Our first user's decoder also needs mantissas
below 2^53. ledgermill therefore refuses amounts with more than 15 digits in total
(`number-too-large`), so every quantity it reports is one a JSON consumer can check exactly.

## Probe

`probes/big-numbers.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/big-numbers.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1.23456789012345678901234567890123456789E+39,10]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-1.23456789012345678901234567890123456789E+39,10]]}]}
```
