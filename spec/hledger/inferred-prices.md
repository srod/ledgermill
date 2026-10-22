# Inferred prices

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

In a group with no written price and no elided amount, when exactly two commodities do not sum to
zero and their sums have opposite signs, one of them gets a price in the other, so the group
balances:

- The commodity of the first amount, in posting order, among the two gets the price.
- If exactly one posting holds it, that posting gets a total price: the negated sum of the other
  commodity, places as summed.
- If several do (zero amounts count), each gets the same unit price: the negated other sum
  divided by this sum, kept to 255 places; reported rounded half to even to ten places, without
  trailing zeros when the exact quotient fits in ten places.
- The sums follow the same zero rule as elided amounts: a zero term gives back the other term.
- A posting that holds more than one amount, a third unbalanced commodity, or sums of the same
  sign: no inference, and the group is unbalanced. The posting that holds more than one amount
  is one that would get the price: the holder of the *other* commodity may hold several (an
  assignment `a == 3 USD` also gives `a` a `-1 X`; with `d -6 Y` written before it the price is
  inferred, `d -6 Y @@ -3 USD`, and with `a` first it is not: `pi72-infer`, `pi71-infer`).

## Probe

`probes/inferred-prices.journal`, `probes/inferred-prices-same-sign.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/inferred-prices.journal
hledger --no-conf -f spec/hledger/probes/inferred-prices-same-sign.journal print -O json
```

## Output

inferred-prices:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"one posting in the first commodity: a total","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@@","Y",250,2]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",-250,2]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"several: a unit price, rounded to ten places","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@","Y",3333333333,10]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","Y",3333333333,10]]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["Y",-1,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"exact unit prices lose trailing zeros","comment":"","line":10,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@","Y",15,1]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@","Y",15,1]]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["Y",-300,2]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"the first posting holding one of the two commodities gets the price","comment":"","line":15,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["Z",0,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",-2,0,["@@","X",-1,0]]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",1,0]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"a commodity that sums to zero is left out","comment":"","line":20,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@@","Y",2,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",-2,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["Z",1,0]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["Z",-1,0]]}]}
```

inferred-prices-same-sign:

```text
hledger: Error: spec/hledger/probes/inferred-prices-same-sign.journal:1-3:
1 | 2026-01-01 t
  |     a             1 X
  |     b             2 Y
This multi-commodity transaction is unbalanced.
The real postings all have the same sign. Consider negating some of them.
Consider adjusting this entry's amounts, adding missing postings,
or recording conversion price(s) with @, @@ or equity postings.
(exit status 1)
```
