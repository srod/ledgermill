# Balancing

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

Each group must sum to zero: the regular postings, and separately the balanced virtual `[...]`
postings. Virtual `(...)` postings are never balanced. A transaction with no postings, or whose
only posting is zero, is fine.

What a posting contributes: its amount, or for a priced amount its cost (unit price times
quantity without trailing zeros, or the total). Lots never convert.

A group balances when, for every commodity, its exact sum rounded half to even to the commodity's
precision is zero. The precision is the most decimal places among the amounts written in that
commodity anywhere in the transaction (virtual postings included); if no amount is written in
it, the most places among the prices written in it. It is capped at ten.

`1.004 EUR` and `-1.00 EUR` do not balance (precision 3); `1 X @ 0.44 EUR` and `-0.4 EUR` do
(precision 1: 0.04 rounds to 0.0); `0.45` too (half to even); `0.46` does not.

ledgermill reports `unbalanced-transaction` on the header line, with the group and the residual
per commodity, and still returns the transaction.

## Probe

`probes/balance-ok-mixed-places.journal`, `probes/balance-single-zero-posting.journal`, `probes/tolerance.journal`, `probes/balance-unbalanced.journal`, `probes/balance-single-posting.journal`, `probes/tolerance-fails.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/balance-ok-mixed-places.journal
spec/hledger/show.sh spec/hledger/probes/balance-single-zero-posting.journal
spec/hledger/show.sh spec/hledger/probes/tolerance.journal
hledger --no-conf -f spec/hledger/probes/balance-unbalanced.journal print -O json
hledger --no-conf -f spec/hledger/probes/balance-single-posting.journal print -O json
hledger --no-conf -f spec/hledger/probes/tolerance-fails.journal print -O json
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

tolerance:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"checked to the places written: 0.44 rounds to 0.4","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@","EUR",44,2]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-4,1]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"half to even: 0.45 rounds to 0.4","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@","EUR",45,2]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-4,1]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"places of a virtual posting count too","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",334,3]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-100,2]]},{"account":"v","type":"VirtualPosting","comment":"","amounts":[["EUR",10,1]]}]}
```

balance-unbalanced:

```text
hledger: Error: spec/hledger/probes/balance-unbalanced.journal:1-3:
1 | 2026-01-01 t
  |     a       1.004 EUR
  |     b       -1.00 EUR
This transaction is unbalanced.
The real postings' sum should be 0 but is: 0.004 EUR
(exit status 1)
```

balance-single-posting:

```text
hledger: Error: spec/hledger/probes/balance-single-posting.journal:1-2:
1 | 2026-01-01 t
  |     a           1 EUR
This transaction is unbalanced.
The real postings' sum should be 0 but is: 1 EUR
(exit status 1)
```

tolerance-fails:

```text
hledger: Error: spec/hledger/probes/tolerance-fails.journal:1-3:
1 | 2026-01-01 0.46 rounds to 0.5
  |     a    1 X @ 0.46 EUR
  |     b          -0.4 EUR
This multi-commodity transaction is unbalanced.
The real postings' sum should be 0 but is: 0.06 EUR
Consider adjusting this entry's amounts, adding missing postings,
or recording conversion price(s) with @, @@ or equity postings.
(exit status 1)
```
