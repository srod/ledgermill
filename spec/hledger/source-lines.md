# Source lines and file layout

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

- A transaction's source line is `tsourcepos[0].sourceLine`: the **header line**, counted from 1
  over the whole file, blank and comment lines included. (`tsourcepos[1]` is the end, one past
  the last line.)
- Lines end with `\n`. `\r\n` is accepted and changes nothing, including line numbers. The last
  line needs no newline. A byte order mark at the start of a file is ignored.
- Postings may be indented with spaces (even one), a tab, or a mix.
- `sourceName` is the path as hledger resolved it (absolute); ledgermill reports the name it
  was given.
- An empty file, or one with only comments, gives no transactions and no error.

ledgermill refuses tabs and unusual whitespace anywhere but in the indentation, and a lone `\r`.

## Probe

`probes/tab-indent.journal`, `probes/crlf.journal`, `probes/bom.journal`,
`probes/no-final-newline.journal`, `probes/empty.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/tab-indent.journal
spec/hledger/show.sh spec/hledger/probes/crlf.journal
spec/hledger/show.sh spec/hledger/probes/bom.journal
spec/hledger/show.sh spec/hledger/probes/no-final-newline.journal
spec/hledger/show.sh spec/hledger/probes/empty.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"tab indent","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"mixed indent","comment":"","line":5,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"crlf","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"c\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"bom","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"no final newline","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
```
