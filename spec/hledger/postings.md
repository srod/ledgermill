# Posting lines: status, virtual postings, account names

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

- A posting may start with a status `*` or `!` (with or without a space after it); it is not
  reported.
- `(account)` is a virtual posting: never balanced. `[account]` is a balanced virtual posting:
  balanced among the other `[...]` postings of the transaction, separately from the regular ones.
- The account name ends at two blanks in a row (spaces, tabs, no-break spaces in any mix). A
  single tab or no-break space inside the name reads as a space.
- A transaction code `(CODE)` after the status is read and not reported.

## Probe

`probes/posting-status-and-virtual.journal`, `probes/posting-account-separators.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/posting-status-and-virtual.journal
spec/hledger/show.sh spec/hledger/probes/posting-account-separators.journal
```

## Output

posting-status-and-virtual:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"v","type":"VirtualPosting","comment":"","amounts":[["",5,0]]},{"account":"bv","type":"BalancedVirtualPosting","comment":"","amounts":[["",2,0]]},{"account":"bw","type":"BalancedVirtualPosting","comment":"","amounts":[["",-2,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["",-2,0]]}]}
```

posting-account-separators:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a tab or a no-break space inside the name reads as a space","comment":"","line":1,"postings":[{"account":"a b","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"c d","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"e","type":"RegularPosting","comment":"","amounts":[["",-2,0]]}]}
```
