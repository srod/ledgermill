# Directives that decide number marks

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

A lone `.` or `,` followed by three digits (`1,000`) is read with what the file declared before
that line:

1. `decimal-mark .` or `decimal-mark ,` decides for every commodity;
2. else a `commodity` directive for the amount's commodity: either a sample amount
   (`commodity 1.000,00 EUR`) or a symbol with an indented `format` line;
3. else a `D` directive's sample amount decides for every commodity;
4. else the separator is the decimal mark.

The declared mark wins even when the separator is not followed by three digits: after
`commodity 1,000.00 EUR`, `1,5 EUR` is 15. Space groups override every declaration.

A `commodity` or `D` sample amount is read like any amount, with what is already declared, and must
end up with a decimal mark (`commodity 1000 EUR` is an error; `D 1.00 EUR` after `decimal-mark ,`
is an error because `1.00` then has no decimal mark).

`D AMOUNT` also gives its commodity to every amount written without one (prices and lot costs
included). Directives reach only the lines after them, in their own file: a sibling file sees none
of them.

## Probe

`probes/directive-decimal-mark.journal`, `probes/directive-commodity-format.journal`, `probes/directive-default-commodity.journal`, `probes/directive-precedence.journal`, `probes/directive-commodity-needs-mark.journal`, `probes/directive-scope-a.journal` + `probes/directive-scope-b.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/directive-decimal-mark.journal
spec/hledger/show.sh spec/hledger/probes/directive-commodity-format.journal
spec/hledger/show.sh spec/hledger/probes/directive-default-commodity.journal
spec/hledger/show.sh spec/hledger/probes/directive-precedence.journal
spec/hledger/show.sh spec/hledger/probes/directive-scope-a.journal spec/hledger/probes/directive-scope-b.journal
hledger --no-conf -f spec/hledger/probes/directive-commodity-needs-mark.journal print -O json
```

## Output

directive-decimal-mark:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a lone comma is the decimal mark, a lone point groups","comment":"","line":3,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1000,3]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1000,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["",999000,3]]}]}
```

directive-commodity-format:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"per commodity","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1000,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["USD",-1000,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["X",1000,3]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["EUR",-1000,0],["USD",1000,0],["X",-1000,3]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"the declared mark wins even without three digits","comment":"","line":11,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",15,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-150,1]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",135,1]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"but space groups make a lone mark the decimal mark","comment":"","line":16,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",100050,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-100050,2]]}]}
```

directive-default-commodity:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"amounts without a commodity get it","comment":"","line":3,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",5,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-5,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"its mark decides for every commodity","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["USD",1000,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["USD",-1000,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["USD",1000,3]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["USD",-1000,3]]}]}
```

directive-precedence:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"decimal-mark beats commodity and D","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1000,0,["@","USD",1,0]]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["USD",-1000,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",1000,3,["@","USD",1,0]]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["USD",-1000,3]]}]}
```

directive-scope-a + directive-scope-b:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"nothing from the other file applies","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1000,3]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1000,3]]}]}
```

directive-commodity-needs-mark:

```text
hledger: Error: spec/hledger/probes/directive-commodity-needs-mark.journal:1:11:
  |
1 | commodity 1000 EUR
  |           ^
Please include a decimal point or decimal comma in commodity directives,
to help us parse correctly. It may be followed by zero or more decimal digits.
Examples:
commodity $1000.            ; no thousands mark, decimal period, no decimals
(exit status 1)
```
