# Conversion postings

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

A conversion account is, while the file declares none: `equity:conversion`, `equity:trade`,
`equity:trades`, `equity:trading` or a sub-account of one of them, in any case
(`equity:conversions`, `equity:tradings` are not). Once a file declares an account of type `V`
or `Conversion` (an `account` directive with a `type:` tag, on its line or an indented comment
line), only declared accounts and their sub-accounts are conversion accounts in that file.

The `type:` tag: in each comma-separated part of the comment, a tag is the word right before a
`:`, preceded by a blank or the start; the first `type:` tag counts. Its value, up to the next
comma, must be one of `A L E R X C V G U` or `Asset Liability Equity Revenue Expense Cash
Conversion Gain UnrealisedGain`, in any case, or the file is rejected.

Conversion postings are paired left to right: two adjacent conversion postings form a pair, the
next pair starts after them (virtual postings count). A pair must not carry a price.

With one priced posting, look at the conversion postings right before it:

- an odd number of them that opens the transaction (and the transaction has a pair): the priced
  posting balances by its quantity, whatever the pairs hold. Unless another posting without a
  price (conversion postings aside, virtual ones included, any commodity) has a quantity within
  half a unit of the priced quantity at the priced quantity's places (`3.05` and `2.95` for
  `3.0`): then it balances by its cost.
- an odd number that does not open the transaction: by its cost.
- an even number (or none): by its quantity when a pair holds its negated quantity and its cost,
  in either order, by number only (the commodities play no part); each pair amount is compared
  at its own places, within half a unit (`8` holds a cost of `7.5`, `7.0` does not). A pair with
  an elided amount holds nothing.

With several priced postings, each pair goes with the one priced posting (counted as above:
even run) it fits; a pair that fits two goes with neither, so two priced postings with the same
numbers both balance by cost.

An elided amount next to a posting balanced by its quantity is rejected by hledger and reported
as unbalanced.

Not pinned down, refused by ledgermill (`conversion-postings`): several priced postings where
one follows an odd run of conversion postings preceded only by conversion and priced postings;
a priced posting on a conversion account.

The leading-run rule has no reason we can see; it was found by enumerating about 70,000 small
transactions (one or two priced postings among conversion and plain postings, with equal and
nearly equal numbers) and reading which postings hledger balanced from the residual it reports
when a further posting keeps the transaction unbalanced.

## Probe

`probes/conversion.journal`, `probes/conversion-declared.journal`, `probes/conversion-type-tags.journal`, `probes/conversion-names.journal`, `probes/conversion-with-cost.journal`, `probes/conversion-type-invalid.journal`, `probes/conversion-elided-with-pair.journal`, `probes/conversion-leading-run.journal`, `probes/conversion-pair-places.journal`, `probes/conversion-pair-places-off.journal`, `probes/conversion-several.journal`, `probes/conversion-several-same-numbers.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/conversion.journal
spec/hledger/show.sh spec/hledger/probes/conversion-declared.journal
spec/hledger/show.sh spec/hledger/probes/conversion-type-tags.journal
hledger --no-conf -f spec/hledger/probes/conversion-names.journal print -O json
hledger --no-conf -f spec/hledger/probes/conversion-with-cost.journal print -O json
hledger --no-conf -f spec/hledger/probes/conversion-type-invalid.journal print -O json
hledger --no-conf -f spec/hledger/probes/conversion-elided-with-pair.journal print -O json
spec/hledger/show.sh spec/hledger/probes/conversion-leading-run.journal
spec/hledger/show.sh spec/hledger/probes/conversion-pair-places.journal
hledger --no-conf -f spec/hledger/probes/conversion-pair-places-off.journal print -O json
spec/hledger/show.sh spec/hledger/probes/conversion-several.journal
hledger --no-conf -f spec/hledger/probes/conversion-several-same-numbers.journal print -O json
```

## Output

conversion:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a matching pair: the priced posting balances by its quantity","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",3,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-6,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"the pair matches on amounts only, in either order","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",3,0]]]},{"account":"Equity:Trades:x","type":"RegularPosting","comment":"","amounts":[["Y",6,0]]},{"account":"equity:trading","type":"RegularPosting","comment":"","amounts":[["Z",-2,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["Y",-6,0]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["Z",2,0]]},{"account":"e","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a pair that does not match changes nothing","comment":"","line":15,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",3,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-6,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"a pair with an elided amount never matches","comment":"","line":21,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",1,0]]]},{"account":"equity:trading","type":"RegularPosting","comment":"","amounts":[["EUR",4,0]]},{"account":"equity:trade","type":"RegularPosting","comment":"","amounts":[["EUR",-6,0]]}]}
```

conversion-declared:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a declared conversion account turns the names off","comment":"","line":3,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",3,0]]]},{"account":"x:sub","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]},{"account":"x","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-6,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"equity:conversion is now an ordinary account","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",3,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-12,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",2,0]]}]}
```

conversion-type-tags:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"b is a conversion account","comment":"a and c are not\n","line":5,"postings":[{"account":"p","type":"RegularPosting","comment":"","amounts":[["X",2,0,["@","EUR",3,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",-2,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"q","type":"RegularPosting","comment":"","amounts":[["EUR",-6,0]]}]}
```

conversion-names:

```text
hledger: Error: spec/hledger/probes/conversion-names.journal:1-5:
1 | 2026-01-01 equity:conversions is not a conversion account
  |     a                      2 X @ 3 EUR
  |     equity:conversions            -2 X
  |     equity:conversions           6 EUR
  |     b                           -6 EUR
This multi-commodity transaction is unbalanced.
The real postings' sum should be 0 but is: 6 EUR, -2 X
(exit status 1)
```

conversion-with-cost:

```text
hledger: Error: spec/hledger/probes/conversion-with-cost.journal:1-3
Conversion postings must not have a cost:
    equity:conversion    -2 X @ 3 EUR
2026-01-01 t
    equity:conversion    -2 X @ 3 EUR
    equity:conversion           6 EUR
(exit status 1)
```

conversion-type-invalid:

```text
hledger: Error: spec/hledger/probes/conversion-type-invalid.journal:1:1:
  |
1 | account x  ; type: Q
  | ^
invalid account type code Q, should be one of A, L, E, R, X, C, V, G, U, Asset, Liability, Equity, Revenue, Expense, Cash, Conversion, Gain, UnrealisedGain
(exit status 1)
```

conversion-elided-with-pair:

```text
hledger: Error: spec/hledger/probes/conversion-elided-with-pair.journal:1-5:
1 | 2026-01-01 t
  |     a                     2 X @ 3 EUR
  |     equity:conversion            -2 X
  |     equity:conversion           6 EUR
  |     b                         -12 EUR
  |     b                             2 X
This multi-commodity transaction is unbalanced.
(exit status 1)
```

conversion-leading-run (each transaction balances only the way the rule says):

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"an odd run of conversion postings opens the transaction: by quantity","comment":"","line":1,"postings":[{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-2,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"a run of three opens it: by quantity, the amounts play no part","comment":"","line":8,"postings":[{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-2,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a posting repeats the quantity: by cost","comment":"","line":15,"postings":[{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-8,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",3,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"at the quantity's places, 3.05 repeats 3.0: by cost","comment":"","line":23,"postings":[{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-30,1]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",30,1,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-8,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",3,0]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["Y",305,2]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["Y",-305,2]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"3.051 does not repeat 3.0: by quantity","comment":"","line":33,"postings":[{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-30,1]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",30,1,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-2,0]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["Y",3051,3]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["Y",-3051,3]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"the odd run does not open the transaction: by cost, the matching pair is ignored","comment":"","line":42,"postings":[{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-12,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@@","EUR",6,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",6,0]]}]}
```

conversion-pair-places (each transaction balances only the way the rule says):

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"each pair amount is read at its own places: 6.2 is 6.25","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",25,1,["@","EUR",25,1]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-25,1]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",62,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-62,1]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"8 is 7.5","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",25,1]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",8,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-8,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a pair matches on numbers only","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["Y",-3,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["USD",6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["USD",-6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]}]}
```

conversion-pair-places-off:

```text
hledger: Error: spec/hledger/probes/conversion-pair-places-off.journal:1-5:
1 | 2026-01-01 7.0 is not 7.5: the pair does not match
  |     a                    3 X @ 2.5 EUR
  |     equity:conversion             -3 X
  |     equity:conversion          7.0 EUR
  |     b                         -7.0 EUR

This multi-commodity transaction is unbalanced.
The real postings' sum should be 0 but is: 7.5 EUR, -3 X
(exit status 1)
```

conversion-several:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"two priced postings, each with its pair","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",4,0,["@@","EUR",7,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["Y",-4,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",7,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",-13,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"the pairs need not sit next to their priced posting","comment":"","line":10,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["Y",-4,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",7,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",4,0,["@@","EUR",7,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",-13,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a pair no priced posting fits changes nothing","comment":"","line":19,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["X",3,0,["@","EUR",2,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["X",-3,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",6,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["Y",4,0,["@@","EUR",7,0]]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"equity:conversion","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",-13,0]]}]}
```

conversion-several-same-numbers:

```text
hledger: Error: spec/hledger/probes/conversion-several-same-numbers.journal:1-9:
1 | 2026-01-01 both pairs fit both priced postings: neither balances by quantity
  |     a                     3 X @ 2 EUR
  |     equity:conversion            -3 X
  |     equity:conversion           6 EUR
  |     b                    3 Y @@ 6 USD
  |     equity:conversion            -3 Y
  |     equity:conversion           6 USD
  |     c                          -6 EUR
  |     c                          -6 USD

This multi-commodity transaction is unbalanced.
The real postings' sum should be 0 but is: 6 EUR, 6 USD, -3 X, -3 Y
(exit status 1)
```
