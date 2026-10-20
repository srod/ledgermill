# Elided amounts

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

At most one posting per group (regular, balanced virtual) may leave out its amount; two is an
error. It gets, per commodity, the negated sum of the other postings of its group, one amount per
commodity, sorted by commodity in code point order (`AA`, `ZZ`, `bb`). An elided virtual posting
gets a zero with no commodity. Alone in its transaction it gets no amount at all.

How the sum is made matters for its decimal places:

1. The amounts are first summed per group of equal-looking amounts: same commodity, same kind of
   price, same price commodity and, for unit prices, the same price value (`@ 1` and `@ 1.00`
   go together).
2. Each group is then converted: costless groups stay as they are, a unit-price group becomes
   quantity times price without trailing zeros, a total-price group becomes the sum of its
   totals.
3. The results are summed per commodity, costless groups first, then by the source commodity,
   totals before unit prices.

In both sums the first term starts the sum and a zero term gives back the other term with its
places: `1.25`, `3`, `-1.25` give `-3.00` (`1.25 + 3` is `4.25`, minus `1.25` is `3.00`), while
`1.25`, `-1.25`, `3` give `-3` (the running sum is zero when `3` comes). Two zeros give the later
one in step 1 (`0.0`, `0.00` give `0.00`) and the earlier one in step 3.

A zero sum is kept as a zero amount when the posting has other amounts. A lot survives on the
elided amount only when exactly one costless amount contributed to that commodity.

## Probe

`probes/elided.journal`, `probes/elided-grouping.journal`, `probes/elided-two.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/elided.journal
spec/hledger/show.sh spec/hledger/probes/elided-grouping.journal
hledger --no-conf -f spec/hledger/probes/elided-two.journal print -O json
```

## Output

elided:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"one commodity","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",150,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-15,1]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",0,2]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"several commodities, one amount each, sorted","comment":"","line":6,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["bb",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["ZZ",1,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["AA",1,0]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["AA",-1,0],["ZZ",-1,0],["bb",-1,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a zero sum is kept when the posting has other amounts","comment":"","line":12,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["USD",10,1]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["EUR",0,0],["USD",-10,1]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"summing onto a zero keeps the other term places","comment":"","line":18,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["C",125,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["C",3,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["C",-125,2]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["C",-300,2]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"alone: no amount","comment":"","line":24,"postings":[{"account":"e","type":"RegularPosting","comment":"","amounts":[]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"an elided virtual posting gets a zero","comment":"","line":27,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]},{"account":"v","type":"VirtualPosting","comment":"","amounts":[["",0,0]]}]}
```

elided-grouping:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"equal unit prices are grouped by value before converting","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["AB",15,1,["@","X",1,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["AB",-15,1,["@","X",100,2]]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["AB",1,0,["@@","X",1,0]]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["X",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"totals are summed after dropping their trailing zeros","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@@","Y",15,1]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",1,0,["@@","Y",15,1]]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["Y",-30,1]]}]}
```

elided-two:

```text
hledger: Error: spec/hledger/probes/elided-two.journal:1-4:
1 | 2026-01-01 t
  |     a               1
  |     b
  |     c
This multi-commodity transaction is unbalanced.
There can't be more than one real posting with no amount.
(Remember to put two or more spaces between account and amount.)
(exit status 1)
```
