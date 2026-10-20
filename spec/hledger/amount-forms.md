# Amount forms

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

An amount is a number with an optional commodity symbol on either side, and an optional minus
sign. The reported commodity is the symbol without any spacing; a number with no symbol has the
commodity `""`.

- The symbol may come before or after the number, glued to it or after one space:
  `$3.50`, `3.50 EUR`, `3.50EUR`, `EUR 3.50`, `EUR3.50`.
- The minus sign goes in front of the whole amount (`-$3.50`, `-3.50 EUR`, `-EUR 3.50`) or in
  front of the number when the symbol is on the left (`$-3.50`, `EUR -3.50`).
- Symbols can be letters of any script (`руб`, `元`, `€`), upper or lower case. They are
  case sensitive: `eur` and `EUR` are different commodities. A lone `E` is a symbol, not an exponent.
- The quantity is the same whichever form was used.

More forms (signs, quotes, separators, exponents) are in [numbers](numbers.md). A symbol is a run
of characters other than digits, blanks (space, tab, newline) and `-+.@;"{}=*`; a no-break space
or a carriage return after it is part of it. `"quoted"` symbols may hold anything but `"`.

## Probe

`probes/amount-forms.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/amount-forms.journal
```

## Output (amounts only matter)

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"bare","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-3,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"symbol before, no space","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["$",350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["$",-350,2]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"sign after the symbol","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["$",-350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["$",350,2]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"symbol after, spaced","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-350,2]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"symbol after, glued","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-350,2]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"symbol before, spaced","comment":"","line":21,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-350,2]]}]}
{"date":"2026-01-07","status":"Unmarked","code":"","description":"symbol before, glued, sign first","comment":"","line":25,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",-350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",350,2]]}]}
{"date":"2026-01-08","status":"Unmarked","code":"","description":"non-ASCII symbols","comment":"","line":29,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["€",350,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["€",-350,2]]}]}
{"date":"2026-01-09","status":"Unmarked","code":"","description":"cyrillic symbol","comment":"","line":33,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["руб",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["руб",-3,0]]}]}
{"date":"2026-01-10","status":"Unmarked","code":"","description":"cjk symbol","comment":"","line":37,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["元",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["元",-1,0]]}]}
{"date":"2026-01-11","status":"Unmarked","code":"","description":"symbols are case sensitive","comment":"","line":41,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["eur",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["eur",-3,0]]}]}
{"date":"2026-01-12","status":"Unmarked","code":"","description":"a lone E is a symbol, not an exponent","comment":"","line":45,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["E",2,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["E",-2,0]]}]}
```
