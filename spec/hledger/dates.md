# Dates and Y

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

A date is `YEAR SEP MONTH SEP DAY` with `-`, `/` or `.` used for both separators; months and days
may be one digit, and the year at least four digits (`0026` is year 26). It is printed padded
(`2026-01-06`). Mixed separators and impossible dates are errors.

A date without a year (`01-02`, `1/3`) takes the year of the last `Y YEAR` (or `year`, or
`apply year`) directive in the file. Without one, hledger uses the current year: ledgermill
refuses such dates (`date-without-year`), because the answer would depend on the day it runs,
unless the date is wrong in every year, which is an error either way.

A secondary date `DATE=DATE2` is checked (it may omit its year, taking the primary date's) and
otherwise ignored. In posting comments, `date:`/`date2:` tags and bracketed dates are checked
the same way, taking the transaction's year (see [comment-dates](comment-dates.md)).

## Probe

`probes/directive-year.journal`, `probes/header-code-and-dates.journal`, `probes/header-date-mixed-separators.journal`, `probes/directive-date-without-year.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/directive-year.journal
spec/hledger/show.sh spec/hledger/probes/header-code-and-dates.journal
hledger --no-conf -f spec/hledger/probes/header-date-mixed-separators.journal print -O json
hledger --no-conf -f spec/hledger/probes/directive-date-without-year.journal print -O json
```

## Output

directive-year:

```text
{"date":"2024-02-29","status":"Unmarked","code":"","description":"apply year is Y","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2025-01-02","status":"Unmarked","code":"","description":"a date without a year takes Y","comment":"","line":3,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"a secondary date takes the primary date year","comment":"","line":7,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

header-code-and-dates:

```text
{"date":"0026-01-10","status":"Unmarked","code":"","description":"a four-digit year with zeros","comment":"","line":17,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-05","status":"Cleared","code":"42","description":"slashes","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-06","status":"Pending","code":"a b","description":"dots, unpadded","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-07","status":"Unmarked","code":"","description":"secondary date","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-08","status":"Cleared","code":"x","description":"y  spacing","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

header-date-mixed-separators:

```text
hledger: Error: spec/hledger/probes/header-date-mixed-separators.journal:1:1:
  |
1 | 2026-01/05 t
  | ^^^^^^^^^^
This date has different separators, please use consistent separators.
(exit status 1)
```

directive-date-without-year:

```text
[
  {
    "tcode": "",
    "tcomment": "",
    "tdate": "2026-01-02",
    "tdate2": null,
    "tdescription": "t",
    "tindex": 1,
    "tpostings": [
      {
        "paccount": "a",
        "pamount": [
          {
            "acommodity": "",
            "acost": null,
            "acostbasis": null,
            "aquantity": {
              "decimalMantissa": 1,
              "decimalPlaces": 0,
              "floatingPoint": 1
            },
            "astyle": {
              "ascommodityside": "L",
              "ascommodityspaced": false,
              "asdecimalmark": ".",
              "asdigitgroups": null,
              "asprecision": 0,
              "asrounding": "NoRounding"
            }
          }
        ],
        "pbalanceassertion": null,
        "pcomment": "",
        "pdate": null,
        "pdate2": null,
        "poriginal": null,
        "pstatus": "Unmarked",
        "ptags": [],
        "ptransaction_": "1",
        "ptype": "RegularPosting"
      },
      {
        "paccount": "b",
        "pamount": [
          {
            "acommodity": "",
            "acost": null,
            "acostbasis": null,
            "aquantity": {
              "decimalMantissa": -1,
              "decimalPlaces": 0,
              "floatingPoint": -1
            },
            "astyle": {
              "ascommodityside": "L",
              "ascommodityspaced": false,
              "asdecimalmark": ".",
              "asdigitgroups": null,
              "asprecision": 0,
              "asrounding": "NoRounding"
            }
          }
        ],
        "pbalanceassertion": null,
        "pcomment": "",
        "pdate": null,
        "pdate2": null,
        "poriginal": null,
        "pstatus": "Unmarked",
        "ptags": [],
        "ptransaction_": "1",
        "ptype": "RegularPosting"
      }
    ],
    "tprecedingcomment": "",
    "tsourcepos": [
      {
        "sourceColumn": 1,
        "sourceLine": 1,
        "sourceName": "spec/hledger/probes/directive-date-without-year.journal"
      },
      {
        "sourceColumn": 1,
        "sourceLine": 4,
        "sourceName": "spec/hledger/probes/directive-date-without-year.journal"
      }
    ],
    "tstatus": "Unmarked",
    "ttags": []
  }
]
```
