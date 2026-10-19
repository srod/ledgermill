# Balancing

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

- In one commodity, a transaction balances when the exact sum of its amounts is zero, whatever the
  places (`0.30 + 0.7 - 1`; `1.50 - 1.5 + 0`). `1.004 - 1.00` does not balance: hledger prints
  an error and **no transactions at all**.
- A transaction with one posting balances only if that amount is zero. A transaction with no
  postings is accepted.
- Postings in different commodities are not an error when they cancel: hledger infers a price.
  That also happens between a commodity and no commodity (`3 EUR` and `-3`) and between `EUR` and
  `eur`. ledgermill refuses all of them (`multiple-commodities`): each such transaction would
  also report a price that the typed value has no place for.

ledgermill reports `unbalanced-transaction` on the header line where hledger fails, and still
returns the transaction.

## Probe

`probes/balance-ok-mixed-places.journal`, `probes/balance-unbalanced.journal`,
`probes/balance-single-posting.journal`, `probes/balance-single-zero-posting.journal`,
`probes/balance-two-commodities.journal`, `probes/balance-commodity-less-and-symbol.journal`,
`probes/balance-case-differs.journal`, and the first transaction of `probes/decimal-places.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/<probe>.journal   # for each probe above
```

## Output

balance-ok-mixed-places:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",150,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-15,1]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",0,0]]}]}
```
balance-single-zero-posting:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",0,0]]}]}
```
balance-two-commodities:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["USD",350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-350,2]]}]}
```
balance-commodity-less-and-symbol:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-3,0]]}]}
```
balance-case-differs:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["eur",-3,0]]}]}
```
balance-unbalanced:

```text
hledger: Error: spec/hledger/probes/balance-unbalanced.journal:1-3:
1 | 2026-01-01 t
  |     a       1.004 EUR
  |     b       -1.00 EUR
(exit status 1)
```
balance-single-posting:

```text
hledger: Error: spec/hledger/probes/balance-single-posting.journal:1-2:
1 | 2026-01-01 t
  |     a           1 EUR

(exit status 1)
```
