# Ordering

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

`print` lists transactions sorted by date, and keeps parse order among equal dates; the file's
own order does not matter. With several files, parse order is the order of the `-f` options, so
equal dates list the first file's transactions first. The same file given twice is read twice.
Each transaction's line is counted in its own file.

## Probe

`probes/order-by-date.journal`, `probes/order-file-a.journal`, `probes/order-file-b.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/order-by-date.journal
spec/hledger/show.sh spec/hledger/probes/order-file-a.journal spec/hledger/probes/order-file-b.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a2 same date, later in the file","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"b","comment":"","line":9,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"c","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"from file b","comment":"","line":1,"postings":[{"account":"x","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"y","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"from file a","comment":"","line":1,"postings":[{"account":"x","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"y","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"from file b, later","comment":"","line":5,"postings":[{"account":"x","type":"RegularPosting","comment":"","amounts":[["",2,0]]},{"account":"y","type":"RegularPosting","comment":"","amounts":[["",-2,0]]}]}
```
