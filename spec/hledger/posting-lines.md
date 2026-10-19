# Posting lines

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

`INDENT ACCOUNT  AMOUNT [; COMMENT]`

- The account ends at the first run of **two or more spaces** (or a tab). A single space is part
  of the account name: `a b  1` is account `a b`; `b -1` is an account named `b -1` with no
  amount.
- Account names may hold anything else too: `é:ü`, a leading colon, even `;` (`a;x  1`).
  ledgermill only reads names that start with a letter or digit and hold letters, digits and
  ``: - _ . & ' / , +`` and single spaces; other names are `unsupported-syntax` (`account-name`).
- A posting with **no amount gets one anyway**: balancing fills it in (`0` here, in the
  commodity-less form). ledgermill refuses these (`elided-amount`), as it does a name with only
  one space before its "amount".
- An indented line that is not a comment is a posting whatever it looks like: an indented
  `2026-01-02 u` becomes an account with an amount of `0`.
- A leading `*` or `!` is the posting's status; `(a)` makes a virtual posting, which is left out of
  balancing; `@` gives a price; `= 1` a balance assertion; extra text after the amount is
  an error. All refused by ledgermill.

## Probe

`probes/posting-forms.journal`, `probes/posting-elided-amount.journal`,
`probes/posting-single-space-before-amount.journal`, `probes/posting-indented-header-line.journal`,
`probes/posting-status.journal`, `probes/posting-virtual.journal`, `probes/posting-price.journal`,
`probes/posting-assertion.journal`, `probes/posting-junk-after-amount.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/<probe>.journal   # for each probe above
```

## Output

posting-forms:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"account names","comment":"","line":1,"postings":[{"account":"a b","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"c:d:e","type":"RegularPosting","comment":"","amounts":[["",-2,0]]},{"account":"é:ü","type":"RegularPosting","comment":"","amounts":[["",3,0]]},{"account":":f:","type":"RegularPosting","comment":"","amounts":[["",-2,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"separator is two or more spaces","comment":"","line":7,"postings":[{"account":"x","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"y","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
posting-elided-amount:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"an account alone gets an amount of 0","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["",0,0]]}]}
```
posting-single-space-before-amount:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"one space is part of the account name","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b -1","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
posting-indented-header-line:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]},{"account":"2026-01-02 u","type":"RegularPosting","comment":"","amounts":[["",0,0]]}]}
```
posting-status:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
posting-virtual:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"VirtualPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
posting-price:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-2,0]]}]}
```
posting-assertion:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
posting-junk-after-amount:

```text
hledger: Error: spec/hledger/probes/posting-junk-after-amount.journal:3:13:
  |
3 |   b  -1 EUR extra
  |             ^
unexpected 'e'
(exit status 1)
```
