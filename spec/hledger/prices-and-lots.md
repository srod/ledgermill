# Prices and lots

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

- `Q C @ P D` gives the amount a unit price, reported as written (`3.50` keeps its places).
- `Q C @@ T D` gives it a total price. The total takes the sign of the quantity (a zero quantity
  counts as positive) and loses its trailing zeros (`@@ 100.00` is 100).
- `{C}`, `{=C}`: a lot cost per unit. `{{T}}`: a total lot cost, reported per unit as T / Q, rounded
  half to even to ten places, without trailing zeros. `[DATE]`: a lot date (a date without a year
  takes the transaction's). `(LABEL)`: a lot label, trimmed. `{}`, `{{}}`, `{=}`: a lot with no
  cost. The parts come in any order, glued or spaced, each at most once.
- A lot alone does not convert the amount; a price does (the lot is kept as well).
- A price or lot amount without a commodity takes the `D` commodity, like any amount.

## Probe

`probes/prices.journal`, `probes/lots.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/prices.journal
spec/hledger/show.sh spec/hledger/probes/lots.journal
```

## Output

prices:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a unit price is reported as written","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",350,2]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-7,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"a total takes the sign of the quantity and loses trailing zeros","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",-2,0,["@@","EUR",-100,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",100,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"glued","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@@","EUR",3,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-3,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"a price without commodity","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","",3,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-6,0]]}]}
```

lots:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a lot cost, date and label","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,{"cost":["EUR",3,0],"date":"2026-01-02","label":"label"}]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"a total lot cost is reported per unit","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,{"cost":["EUR",33333333333,10],"date":null,"label":null}]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"per unit, signed by the quantity","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",-4,0,{"cost":["EUR",-25,1],"date":null,"label":null}]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",4,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"a lot alone does not convert: the elided amount keeps the lot","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,{"cost":["EUR",1,0],"date":null,"label":null}]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",-1,0,{"cost":["EUR",1,0],"date":null,"label":null}]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"a lot and a price: the price converts","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@","EUR",3,0],{"cost":["EUR",2,0],"date":null,"label":null}]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-3,0]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"empty and fixed lots","comment":"","line":21,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,{"cost":null,"date":null,"label":null}]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",1,0,{"cost":["EUR",2,0],"date":null,"label":null}]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]}]}
```
