# Account aliases and apply account

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-20; the
`directive-alias-missing-group*` and `directive-apply-account-alias-once` probes 2026-11-22.

## Rule

- `alias OLD = NEW` replaces `OLD` when it is the whole account name or its leading segments
  (`exp:food` becomes `expenses:food`; `exposed` is unchanged).
- `alias /REGEX/ = REPLACEMENT` replaces every match, case-insensitively; `\N` in the
  replacement is the Nth group (`\0` the whole match). Referring to a group that does not
  exist is an error, but only when the alias matches an account name: the account of a posting
  or of an `account` directive. An alias that matches nothing is fine, and so is an `apply
  account` block that holds no account. hledger names no file or line for this error;
  ledgermill reports a `parse-error` on the line of the first account the alias matches.
  A backslash before a letter or digit in the pattern stands for that
  character (`\d` is `d`), except `\b`, `\<` and `\>`; lazy quantifiers and `(?` groups are
  errors. An empty match is replaced too, including one right after another match.
- Several aliases: the one declared last is applied first, each at most once. `end aliases`
  clears them. Aliases reach the lines after them in their own file.
- `apply account NAME` prefixes every account (virtual ones included) with `NAME:` until the
  matching `end apply account`; blocks nest. Aliases apply to the full name, prefix included,
  and only once: under `apply account a`, `alias a = a:b` turns `x` into `a:b:x`, and
  `alias /^(.*)$/ = \1:z` turns it into `a:x:z`.

## Probe

`probes/directive-alias.journal`, `probes/directive-alias-order.journal`, `probes/directive-apply-account.journal`,
`probes/directive-apply-account-alias-once.journal`, `probes/directive-alias-missing-group.journal`,
`probes/directive-alias-missing-group-account.journal`, `probes/directive-alias-missing-group-unmatched.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/directive-alias.journal
spec/hledger/show.sh spec/hledger/probes/directive-alias-order.journal
spec/hledger/show.sh spec/hledger/probes/directive-apply-account.journal
spec/hledger/show.sh spec/hledger/probes/directive-apply-account-alias-once.journal
hledger --no-conf -f spec/hledger/probes/directive-alias-missing-group.journal print -O json
hledger --no-conf -f spec/hledger/probes/directive-alias-missing-group-account.journal print -O json
spec/hledger/show.sh spec/hledger/probes/directive-alias-missing-group-unmatched.journal
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

directive-apply-account-alias-once:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"a plain alias on the prefixed name","comment":"","line":4,"postings":[{"account":"a:b:x","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"a:b:y","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"a regex alias on the prefixed name","comment":"","line":13,"postings":[{"account":"a:x:z","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"a:y:z","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

directive-alias-missing-group (the alias matches the posting account `a`) and
directive-alias-missing-group-account (it matches `account a`; no posting uses `a`) print the same
error, with no file or line:

```text
hledger: Error: problem in account alias applied to a: no match group exists for backreference "\a"
(exit status 1)
```

directive-alias-missing-group-unmatched (the same alias, an empty `apply account a` block, no
account that matches):

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"no account matches the alias","comment":"","line":5,"postings":[{"account":"x","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
