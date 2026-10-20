# Account aliases and apply account

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20.

## Rule

- `alias OLD = NEW` replaces `OLD` when it is the whole account name or its leading segments
  (`exp:food` becomes `expenses:food`; `exposed` is unchanged).
- `alias /REGEX/ = REPLACEMENT` replaces every match, case-insensitively; `\N` in the
  replacement is the Nth group (`\0` the whole match), and referring to a group that does not
  exist is an error. A backslash before a letter or digit in the pattern stands for that
  character (`\d` is `d`), except `\b`, `\<` and `\>`; lazy quantifiers and `(?` groups are
  errors. An empty match is replaced too, including one right after another match.
- Several aliases: the one declared last is applied first, each at most once. `end aliases`
  clears them. Aliases reach the lines after them in their own file.
- `apply account NAME` prefixes every account (virtual ones included) with `NAME:` until the
  matching `end apply account`; blocks nest. Aliases apply to the full name, prefix included.

## Probe

`probes/directive-alias.journal`, `probes/directive-alias-order.journal`, `probes/directive-apply-account.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/directive-alias.journal
spec/hledger/show.sh spec/hledger/probes/directive-alias-order.journal
spec/hledger/show.sh spec/hledger/probes/directive-apply-account.journal
```

## Output

directive-alias:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":5,"postings":[{"account":"expenses:grub","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"expenses:x","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"expenses","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"exposed","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"bank:a","type":"RegularPosting","comment":"","amounts":[["",-4,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"cleared","comment":"","line":14,"postings":[{"account":"exp:food","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"assets:bank","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

directive-alias-order:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"the latest alias first, each once","comment":"","line":4,"postings":[{"account":"c","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"c","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

directive-apply-account:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":4,"postings":[{"account":"top:mid:a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"top:mid:v","type":"VirtualPosting","comment":"","amounts":[["",1,0]]},{"account":"top:mid:bv","type":"BalancedVirtualPosting","comment":"","amounts":[["",1,0]]},{"account":"top:mid:bw","type":"BalancedVirtualPosting","comment":"","amounts":[["",-1,0]]},{"account":"top:mid:b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"t","comment":"","line":13,"postings":[{"account":"top:a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"top:b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"t","comment":"","line":19,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
