# Numbers

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

A number is digits with optional separators: `.` and `,` (a decimal mark or a digit group), and
spaces that group digits (the ordinary space, the no-break space U+00A0, the narrow no-break space
U+202F, the thin space U+2009, U+2002, U+2003, U+2008 and U+205F; other Unicode spaces end the
number). Which separator is the decimal mark is decided per number:

- Two different marks (`1,000.5`, `1.000,5`): the last one is the decimal mark, the other groups.
- One mark used twice or more (`1,000,000`): digit groups, no decimal part.
- A space always groups. A number with space groups may hold one `.` or `,` after them, and that
  one is the decimal mark, whatever was declared (`1 000,5`, `1 000.5`).
- One `.` or `,` used once, not followed by exactly three digits (`1,5`, `10,00`): decimal mark.
- One `.` or `,` used once and followed by exactly three digits (`1,000`) is ambiguous. A
  declaration decides (see [directives-numbers](directives-numbers.md)); without one it is the
  decimal mark (`1,000` is 1 with three places).
- A leading `.` (`.5`) and a trailing mark (`5.`, `1,000.`) are decimal marks.

Other forms: a leading `+`; leading zeros (`007.50` is 7.50); one or more blanks between the sign
and the number (`- 3`); an exponent `e`/`E` with an optional sign (`1.5e2` is 150, `1.5e-1` is
0.15 with two places). A lone `e` with no digits after it is a commodity symbol (`2e` is 2 `e`).

Refused by hledger: a third alternation of marks (`1,000.000,5`), digit groups together with an
exponent (`1,000.5e2`), a space group after the decimal mark, two marks in a row.

The quantity keeps its places as written. Values are exact whatever their size: ledgermill keeps
an arbitrary-precision mantissa (hledger's JSON writes very large mantissas in floating notation,
which our harness reads back exactly). Reported quantities are rounded half to even to ten places;
the value used for balancing is the exact one.

## Probe

`probes/numbers-marks.journal`, `probes/numbers-forms.journal`, `probes/numbers-group-spaces.journal`, `probes/numbers-many-places.journal`, `probes/numbers-big.journal`, `probes/numbers-invalid-separators.journal`, `probes/numbers-groups-and-exponent.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/numbers-marks.journal
spec/hledger/show.sh spec/hledger/probes/numbers-forms.journal
spec/hledger/show.sh spec/hledger/probes/numbers-group-spaces.journal
spec/hledger/show.sh spec/hledger/probes/numbers-many-places.journal
spec/hledger/show.sh spec/hledger/probes/numbers-big.journal
hledger --no-conf -f spec/hledger/probes/numbers-invalid-separators.journal print -O json
hledger --no-conf -f spec/hledger/probes/numbers-groups-and-exponent.journal print -O json
```

## Output

numbers-marks:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"two different separators: the last is the decimal mark","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",10005,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-10005,1]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"one separator used twice groups digits","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1000000,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1000000,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"spaces always group","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",10005,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-10005,1]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"a lone separator not followed by three digits is the decimal mark","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",15,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-15,1]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"a lone separator followed by three digits, nothing declared: decimal mark","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1000,3]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1000,3]]}]}
```

numbers-forms:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a leading plus sign","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-3,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"no integer digits","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",5,1]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-5,1]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a trailing mark","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",5,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-5,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"leading zeros","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",750,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-75,1]]}]}
{"date":"2026-01-05","status":"Unmarked","code":"","description":"a space after the sign","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",-3,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",3,0]]}]}
{"date":"2026-01-06","status":"Unmarked","code":"","description":"exponents","comment":"","line":21,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",150,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-150,0]]}]}
{"date":"2026-01-07","status":"Unmarked","code":"","description":"a negative exponent keeps the places it creates","comment":"","line":25,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",15,2]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-15,2]]}]}
{"date":"2026-01-08","status":"Unmarked","code":"","description":"a lone e after the number is a commodity","comment":"","line":29,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["e",2,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["e",-2,0]]}]}
```

numbers-group-spaces:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"no-break space and narrow no-break space group digits","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1000,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1000,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"a thin space too","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1000,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1000,0]]}]}
```

numbers-many-places:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",0,10]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",0,10]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"kept exactly, reported to ten places","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",11234567890,10]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-11234567890,10]]}]}
```

numbers-big:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"above 2^53","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",9007199254740993,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-9007199254740993,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"thirty digits","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1.23456789012345678901234567890123456789E+39,10]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-1.23456789012345678901234567890123456789E+39,10]]}]}
```

numbers-invalid-separators:

```text
hledger: Error: spec/hledger/probes/numbers-invalid-separators.journal:2:15:
  |
2 |   a  1,000.000,5
  |               ^
invalid number (invalid use of separator)
(exit status 1)
```

numbers-groups-and-exponent:

```text
hledger: Error: spec/hledger/probes/numbers-groups-and-exponent.journal:2:6:
  |
2 |   a  1,000.5e2
  |      ^^^^^^^^^
invalid number: digit separators and exponents may not be used together
(exit status 1)
```
