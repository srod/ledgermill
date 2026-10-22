# Balance assertions: failures, the line blamed, which error comes first, `-I`

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

**Verdict.** A journal with a failing assertion is rejected by every command that reads it:
`print` (also `print -O json`) prints nothing on stdout, writes the error to stderr and exits with
status 1; `bal` and `check` do the same. With `-I` / `--ignore-assertions` the same journal is
accepted, and `print` shows it.

**The error.** The first line of the error is `hledger: Error: FILE:LINE:COLUMN:`, then the
transaction with the assertion marked, then `Balance assertion failed in ACCOUNT`, then a sentence
saying what was compared and the two amounts:

- **LINE is the line of the posting that carries the assertion** (the posting's own line in the
  file, for an assertion on the first posting or the last, in a transaction with comments between
  the postings, after a block `comment`, with CRLF or a BOM, with tab indentation, on the last line
  of a file with no final newline), not the line of the transaction's header. **COLUMN** is the
  column of the first `=` of the assertion (a tab moves to the next multiple of 8). FILE is the
  file that holds the posting, as an absolute path; the harness compares file names by basename.
- The sentence is `In commodity C at this point, excluding subaccounts, ignoring costs` for `=`
  and `Across all commodities at this point, excluding subaccounts, ignoring costs` for `==`
  (`including subaccounts` for the `*` forms). Then `the asserted balance is: A` and `but the
  calculated balance is: B`, and `(difference: D)`. The amounts are exact, with the places they have
  ([assignments](assignments.md) for how a balance gets its places). An absent commodity is
  shown in an odd way (`USD0`, `EUR0`: the commodity sign and a zero).
- For a failing `==` the commodity shown is: the asserted commodity if its balance is wrong;
  otherwise the first commodity of the account (code point order) whose balance is not zero, with
  an asserted balance of 0 for it. With 10 EUR, 5 USD, 1 GBP and `== 10 EUR`, it is GBP
  (asserted 0, calculated 1); with 10 EUR and 5 USD and `== 10 USD`, it is USD (asserted 10,
  calculated 5) though EUR is wrong too.

**Only one failure is reported: the first in the order of the walk**
([assignment-processing](assignment-processing.md), [assertion-order](assertion-order.md)): by
date, then by position in the file, then by posting order inside a transaction. Not the first in
the file, and not the first account in name order.

**Several files.** The files are processed in `-f` order, one after the other, each completely
(parse, balancing, assertions) before the next; the error is the one of the first file that has
any. So an assertion failing in the first file is reported even if the second file does not parse,
is unbalanced, or has an earlier-dated failing assertion; and a parse error in the first file
hides an assertion failure in the second. The same file given twice is two files.

**Which error comes first, within one file:**

1. a parse error (any syntax error, impossible date, malformed assertion...): the first in the file;
2. a problem with a plain transaction (one without an assignment): unbalanced, two postings with no
   amount, price inference that fails...: the first in file order, not date order;
3. then, in the order of the walk (date, then file position): for a plain posting, a failing assertion;
   for a transaction with an assignment, in the order of its postings: a posting date on an
   assignment, a failing assertion on a posting, and, after the last posting, a problem with the
   balancing of the transaction (two postings with no amount, unbalanced).

So an unbalanced transaction hides every failing assertion of its file, whatever the dates, unless
the unbalanced transaction has an assignment and the failing assertion comes earlier in the walk.
Within a transaction with an assignment an assertion failing on one of its postings comes before
the transaction's own balance error. A transaction that is both unbalanced and has a failing
assertion, if plain, is reported as unbalanced.

**Positions of the other errors.** Parse errors: `FILE:LINE:COLUMN`, the line of the offending
posting or header. Unbalanced transaction and two postings with no amount: `FILE:FIRST-LAST`, the
lines of the whole transaction (so the header line first); the error text is one of
`This transaction is unbalanced.` / `This multi-commodity transaction is unbalanced.`, with
`There can't be more than one real posting with no amount.` for the two-amounts case. An
assignment with a posting date: no position, the message shows the transaction.

**`-I` / `--ignore-assertions`.** It only skips the check of assertions. Assignments are still
worked out (their amounts are filled in), and every other error stays: parse errors,
unbalanced transactions, two amount-less postings, the posting date on an assignment. When all
the assertions of a journal hold, `print` and `print -O json` give the same bytes with and without
`-I`. When one fails, `-I` makes `print` succeed, and nothing is changed by the failed assertion:
the running balance is the postings' sum, so a later assignment uses it. `print` shows the
assertion in the posting's line as written (`= 9 EUR`); in JSON it is `pbalanceassertion` in both
cases.

## Probe

`probes/assert-fail-basic.journal`, `probes/assert-fail-total-several.journal`,
`probes/assert-fail-total-asserted-first.journal`, `probes/assert-fail-line-bom-crlf.journal`,
`probes/assert-fail-line-tabs.journal`, `probes/assert-fail-line-no-final-newline.journal`,
`probes/assert-fail-line-block-comment.journal`, `probes/assert-fail-first-by-date.journal`,
`probes/assert-fail-first-in-transaction.journal`, `probes/assert-prec-unbalanced.journal`,
`probes/assert-prec-two-elided.journal`, `probes/assert-prec-parse.journal`,
`probes/assert-prec-both.journal`, `probes/assign-prec-*.journal`,
`probes/assert-file-late-fail.journal`, `probes/assert-file-early-fail.journal`,
`probes/assert-file-parse-error.journal`, `probes/assert-file-unbalanced.journal`,
`probes/assert-ignored.journal`

Command:

```sh
hledger --no-conf -f spec/hledger/probes/assert-fail-basic.journal print; echo "exit $?"
hledger --no-conf -f spec/hledger/probes/assert-fail-basic.journal print 2>/dev/null | wc -c
spec/hledger/assertion-verdicts.sh spec/hledger/probes/assert-fail-line-*.journal \
  spec/hledger/probes/assert-prec-*.journal spec/hledger/probes/assign-prec-*.journal
P=spec/hledger/probes
spec/hledger/assertion-verdicts.sh $P/assert-file-late-fail.journal+$P/assert-file-early-fail.journal \
  $P/assert-file-early-fail.journal+$P/assert-file-late-fail.journal \
  $P/assert-file-late-fail.journal+$P/assert-file-parse-error.journal \
  $P/assert-file-parse-error.journal+$P/assert-file-late-fail.journal \
  $P/assert-file-late-fail.journal+$P/assert-file-unbalanced.journal \
  $P/assert-file-unbalanced.journal+$P/assert-file-late-fail.journal
hledger --no-conf -f $P/assert-ignored.journal print -I; echo "exit $?"
hledger --no-conf -f $P/assert-ignored.journal print
```

## Output

```text
hledger: Error: …/spec/hledger/probes/assert-fail-basic.journal:6:12:
  | 2026-01-02 the second posting fails: 10 + 5 is not 99
6 |     a           5 EUR = 99 EUR
  |                       ^^^^^^^^
  |     b          -5 EUR

Balance assertion failed in a
In commodity EUR at this point, excluding subaccounts, ignoring costs,
the asserted balance is:        99 EUR
but the calculated balance is:  15 EUR
(difference: 84 EUR)
To troubleshoot, check this account's running balance with assertions disabled, eg:
hledger reg -I 'a$' cur:EUR
exit 1
```

(The second command prints `0`: nothing on stdout.) The line of the posting, in files where it is
not obvious (`assert-fail-line-*`; the posting is the last, line 7, 7, 3, 3 of its file, the column
is the one of the `=`):

```text
assert-fail-line-block-comment | rejects | assert-fail-line-block-comment.journal:7:17 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 5 EUR; calculated -1 EUR
assert-fail-line-bom-crlf | rejects | assert-fail-line-bom-crlf.journal:7:15 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 5 EUR; calculated -1 EUR
assert-fail-line-no-final-newline | rejects | assert-fail-line-no-final-newline.journal:3:17 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 5 EUR; calculated -1 EUR
assert-fail-line-tabs | rejects | assert-fail-line-tabs.journal:3:33 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 5 EUR; calculated -1 EUR
```

`==`: three commodities, then the asserted commodity first (in the second, EUR is wrong too):

```text
assert-fail-total-several | rejects | assert-fail-total-several.journal:8:12 | assertion-failed: Across all commodities, excluding subaccounts; asserted 0 GBP; calculated 1 GBP
assert-fail-total-asserted-first | rejects | assert-fail-total-asserted-first.journal:7:12 | assertion-failed: Across all commodities, excluding subaccounts; asserted 10 USD; calculated 5 USD
```

First failure (`assert-fail-first-by-date`: lines 2 and 6 both fail, line 6 has the earlier date;
`assert-fail-first-in-transaction`: lines 2 and 3 fail, line 2 comes first):

```text
assert-fail-first-by-date | rejects | assert-fail-first-by-date.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 77 EUR; calculated 1 EUR
assert-fail-first-in-transaction | rejects | assert-fail-first-in-transaction.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 99 EUR; calculated 1 EUR
```

Which error first. In each `assert-prec-*` journal the failing assertion is on line 2 of the
file (the first transaction) and the other problem comes later (`assert-prec-both` has both in
one transaction). The `assign-prec-*` journals mix plain and assignment transactions: in
`-assertion-first` the failing assertion is dated before an assignment that does not balance
(assertion reported); `-assignment-first` the other way round (unbalanced reported);
`-plain-first` a plain unbalanced transaction written and dated after an unbalanced assignment
(plain reported); `-plain-parse-order` two unbalanced plain transactions (file order, not date order);
`-date-order` two unbalanced transactions with assignments (date order); `-posting-date`,
`-two-elided` an earlier failing assertion before an error of the assignment transaction
(assertion reported); `-in-transaction` and `-assertion-in-transaction` an assertion failing
before the balance error of its own transaction (assertion reported):

```text
assert-prec-both | rejects | assert-prec-both.journal:1 | unbalanced
assert-prec-parse | rejects | assert-prec-parse.journal:8:8 | parse
assert-prec-two-elided | rejects | assert-prec-two-elided.journal:5 | two-elided
assert-prec-unbalanced | rejects | assert-prec-unbalanced.journal:5 | unbalanced
assign-prec-assertion-first | rejects | assign-prec-assertion-first.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 9 EUR; calculated 1 EUR
assign-prec-assertion-in-transaction | rejects | assign-prec-assertion-in-transaction.journal:3:13 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 9 EUR; calculated -4 EUR
assign-prec-assignment-first | rejects | assign-prec-assignment-first.journal:1 | unbalanced
assign-prec-date-order | rejects | assign-prec-date-order.journal:5 | unbalanced
assign-prec-in-transaction | rejects | assign-prec-in-transaction.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 9 EUR; calculated 1 EUR
assign-prec-plain-first | rejects | assign-prec-plain-first.journal:5 | unbalanced
assign-prec-plain-parse-order | rejects | assign-prec-plain-parse-order.journal:1 | unbalanced
assign-prec-posting-date | rejects | assign-prec-posting-date.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 9 EUR; calculated 1 EUR
assign-prec-two-elided | rejects | assign-prec-two-elided.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 9 EUR; calculated 1 EUR
```

(`assert-prec-unbalanced`: the failing assertion at line 2 is not reported, the unbalanced
transaction at line 5 is; `assert-prec-parse` the same for the unparsable line 8. In
`assign-prec-plain-first` the unbalanced assignment transaction is on lines 1-3 and the reported
plain one on line 5.)

Files (`assert-file-late-fail` fails at 1 May, `assert-file-early-fail` at 1 January,
`assert-file-parse-error` has an impossible date on line 5, `assert-file-unbalanced` does not balance):

```text
assert-file-late-fail+assert-file-early-fail | rejects | assert-file-late-fail.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 99 EUR; calculated 1 EUR
assert-file-early-fail+assert-file-late-fail | rejects | assert-file-early-fail.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 77 EUR; calculated 1 EUR
assert-file-late-fail+assert-file-parse-error | rejects | assert-file-late-fail.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 99 EUR; calculated 1 EUR
assert-file-parse-error+assert-file-late-fail | rejects | assert-file-parse-error.journal:5:1 | parse
assert-file-late-fail+assert-file-unbalanced | rejects | assert-file-late-fail.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 99 EUR; calculated 1 EUR
assert-file-unbalanced+assert-file-late-fail | rejects | assert-file-unbalanced.journal:1 | unbalanced
```

`-I` (`assert-ignored`: line 2 fails without it; the assignment of the second transaction takes 5
minus the 1 EUR of the first, `a` 4 EUR, even though the assertion `= 9 EUR` was wrong):

```text
2026-01-01 fails unless -I is given
    a           1 EUR = 9 EUR
    b          -1 EUR == 5 EUR
    c           0 EUR =* 3 EUR
    d                 = 0

2026-01-02 an assignment still works under -I
    a                 = 5 EUR
    b

exit 0
```

The plain `print` of that journal exits 1 with the error of line 2 (column 12). With `-O json` and
`-I`, the second transaction's `a` has `[["EUR",4,0]]`.
