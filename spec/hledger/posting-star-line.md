# A posting line that holds only `*`

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

- An indented line of `*` and nothing else (blanks after it are fine: spaces, tabs, no-break
  spaces) is **not a posting**. It ends the transaction, or the `=` / `~` rule, and is read as a
  comment line.
- Why: a posting needs an account after its status mark, so the line fails as a posting; the
  file level then reads it as a comment, because a line that starts with `*` is a comment line
  there, indented or not (like `;` and `#`).
- After it, the lines are read at file level:
  - an indented comment line (`;`, `#` or `*` first) is a comment. It no longer belongs to the
    last posting: `a  1 ; k:v`, then `*`, then `; z:1` leaves `z:1` out of the posting.
  - any other indented line, a posting included, is an error **on that line**
    (`unexpected 'c'`, `expecting newline`; the column is the first non-blank).
  - a blank line, then the next transaction, reads as usual.
- Only `*` is special. `!` alone is an error on its own line (4:5 in the probe).
- Something after the star makes it a posting with a status: `*x`, `* x`, `* ; c` (the account
  is `; c`) and `**` (account `*`) all are. With one amount missing above them, hledger rejects
  those probes as unbalanced on the transaction's lines (`1-4`).
- Since a `*` line is a comment at file level, an indented `#` or `*` line there, after an
  `alias`-like directive, or outside everything, is accepted too (as `;` already was).
- Same in a rule: `= x`, then `*`, is accepted; a posting after it is an error on its line
  (3:5), for `~ monthly` too.

## Probe

`probes/posting-star-alone.journal`, `posting-star-alone-then-posting`, `posting-star-alone-blanks`,
`posting-star-alone-comments`, `posting-star-alone-next-transaction`,
`posting-star-alone-comment-detached`, `posting-star-alone-in-middle`, `posting-bang-alone`,
`posting-star-account`, `posting-star-space-account`, `posting-star-star`,
`posting-star-comment-text`, `comment-line-indented-hash-star`,
`comment-line-indented-after-alias`, `rule-star-alone`, `rule-star-alone-then-posting`,
`rule-star-alone-after-posting`, `rule-periodic-star-alone`, `rule-periodic-star-alone-then-posting`
(all under `spec/hledger/probes/`)

Command:

```sh
spec/hledger/probe-verdicts.sh spec/hledger/probes/posting-star-*.journal \
  spec/hledger/probes/posting-bang-alone.journal spec/hledger/probes/comment-line-indented-*.journal \
  spec/hledger/probes/rule-star-*.journal spec/hledger/probes/rule-periodic-star-*.journal
spec/hledger/show.sh spec/hledger/probes/posting-star-alone-comment-detached.journal
```

## Output

```text
posting-star-account | rejects | posting-star-account.journal:1 | two-elided
posting-star-alone-blanks | accepts | 1 transactions
posting-star-alone-comment-detached | accepts | 1 transactions
posting-star-alone-comments | accepts | 1 transactions
posting-star-alone-in-middle | rejects | posting-star-alone-in-middle.journal:4:5 | parse
posting-star-alone-next-transaction | accepts | 2 transactions
posting-star-alone-then-posting | rejects | posting-star-alone-then-posting.journal:5:5 | parse
posting-star-alone | accepts | 1 transactions
posting-star-comment-text | rejects | posting-star-comment-text.journal:1 | two-elided
posting-star-space-account | rejects | posting-star-space-account.journal:1 | two-elided
posting-star-star | rejects | posting-star-star.journal:1 | two-elided
posting-bang-alone | rejects | posting-bang-alone.journal:4:5 | parse
comment-line-indented-after-alias | accepts | 0 transactions
comment-line-indented-hash-star | accepts | 0 transactions
rule-star-alone-after-posting | accepts | 0 transactions
rule-star-alone-then-posting | rejects | rule-star-alone-then-posting.journal:3:5 | parse
rule-star-alone | accepts | 0 transactions
rule-periodic-star-alone-then-posting | rejects | rule-periodic-star-alone-then-posting.journal:3:5 | parse
rule-periodic-star-alone | accepts | 0 transactions
```

The kind `two-elided` is the first sentence the script finds under the `1-4` error; the error is
the transaction's, not a syntax error.

posting-star-alone-comment-detached (the second posting is not the one the comment would
follow, but the first keeps its own comment and `b` gets none):

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"k:v\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```

A `*` line after a `;` line: `2026-01-01 t`, `a  1`, `b`, `; x`, then `*`, then `; z:1` is accepted
the same way (the `; x` belongs to `b`; `z:1` belongs to nothing).

## What ledgermill does

`*` alone (blanks aside) ends the transaction or rule; the next indented non-comment line is a
`parse-error` on its own line, as in hledger. Indented `#` and `*` lines are comments at file
level and after a directive that takes no body.
