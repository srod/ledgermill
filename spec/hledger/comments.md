# Comments

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-19.

## Rule

- A comment starts at the first `;` of a header or posting line (after an amount no space is
  needed: `-1 ;c` and `-1;c`). Its text is trimmed. hledger reports a comment with a trailing
  newline (`"memo\n"`); ledgermill's `comment` is trimmed, so `"memo"`.
- Indented `;` lines belong to what is above them: before the first posting they continue the
  transaction comment, after a posting they continue that posting's comment. The lines are joined
  with `\n`, each trimmed; hledger keeps empty lines in the middle (`a`, `;`, `b` gives
  `a\n\nb`), and a comment that is only empty lines is empty once trimmed.
- Text after a second `;` is part of the comment (`c;d`).
- Lines starting with `;`, `#` or `*` in column 0 are comments, and so are indented `;` lines
  outside any transaction. They are not attached to anything.
- A column-0 comment, a blank line or a line of only spaces **ends** the transaction: an indented
  line after it is no longer a posting of that transaction (hledger fails on the probes below).
- Tags written in comments (`date:2026-02-01`, `tag: v`) change nothing here: the comment text
  is reported as written and the transaction date is unchanged.
- **But posting comments are read for dates**: an invalid `date:`/`date2:` tag or bracketed date
  makes hledger fail on the whole journal. See [comment-dates](comment-dates.md).

## Probe

`probes/comments.journal`, `probes/comment-tags-do-nothing.journal`,
`probes/comments-col0-ends-transaction.journal`, `probes/comments-blank-ends-transaction.journal`,
`probes/comments-spaces-only-line-ends-transaction.journal`,
`probes/posting-comment-date-tag-invalid.journal`, `probes/posting-comment-bracket-date-invalid.journal`,
`probes/posting-comment-date-tags-ok-and-tx-unchecked.journal`

Command:

```sh
spec/hledger/show.sh spec/hledger/probes/comments.journal
spec/hledger/show.sh spec/hledger/probes/comment-tags-do-nothing.journal
hledger -f spec/hledger/probes/comments-col0-ends-transaction.journal print -O json
hledger -f spec/hledger/probes/comments-blank-ends-transaction.journal print -O json
hledger -f spec/hledger/probes/comments-spaces-only-line-ends-transaction.journal print -O json
hledger -f spec/hledger/probes/posting-comment-date-tag-invalid.journal print -O json
hledger -f spec/hledger/probes/posting-comment-bracket-date-invalid.journal print -O json
spec/hledger/show.sh spec/hledger/probes/posting-comment-date-tags-ok-and-tx-unchecked.journal
```

## Output

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"c1\nc2\n","line":6,"postings":[{"account":"a","type":"RegularPosting","comment":"p1\np1b\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"\np2\n","amounts":[["",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"comments only after postings","comment":"","line":13,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"\n\nlast   one\n","amounts":[["",-1,0]]}]}
{"date":"2026-01-03","status":"Unmarked","code":"","description":"empty comment kinds","comment":"\n\n","line":19,"postings":[{"account":"a","type":"RegularPosting","comment":"\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"spaced   inner  text   ;  with semicolons\n","amounts":[["",-1,0]]}]}
{"date":"2026-01-04","status":"Unmarked","code":"","description":"glued to previous comment","comment":"\nleading comment line, no space\n","line":24,"postings":[{"account":"a","type":"RegularPosting","comment":"glued\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"date:2026-02-01 x:y\n","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"date:2026-03-01\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"tag: v, date: 2026-04-01\n","amounts":[["",-1,0]]}]}
```
```text
hledger: Error: spec/hledger/probes/comments-col0-ends-transaction.journal:4:3:
  |
4 |   b  -1
  |   ^
unexpected 'b'
(exit status 1)
```
```text
hledger: Error: spec/hledger/probes/comments-blank-ends-transaction.journal:4:3:
  |
4 |   b  -1
  |   ^
unexpected 'b'
(exit status 1)
```
```text
hledger: Error: spec/hledger/probes/comments-spaces-only-line-ends-transaction.journal:4:3:
  |
4 |   b  -1
  |   ^
unexpected 'b'
(exit status 1)
```
```text
hledger: Error: spec/hledger/probes/posting-comment-date-tag-invalid.journal:2:15:
  |
2 |   a  1 ; date:2026-02-30
  |               ^^^^^^^^^^

(exit status 1)
```
```text
hledger: Error: spec/hledger/probes/posting-comment-bracket-date-invalid.journal:3:12:
  |
3 |   b  -1 ; [2026-13-45]
  |            ^^^^^^^^^^

(exit status 1)
```
```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"t","comment":"date:2026-02-30 [2026-13-45]\n","line":1,"postings":[{"account":"a","type":"RegularPosting","comment":"[2026-02-01] [=2026-01-02] date2:2026-03-01\n","amounts":[["",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["",-1,0]]}]}
```
