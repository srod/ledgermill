# Balance assertions: the order of the running balances

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

Status: built.

## Rule

An assertion sees the postings that come before it in hledger's own order, not in the order of
the file. This note covers journals whose transactions have no balance assignment; a transaction
with an assignment is processed differently ([assignment-processing](assignment-processing.md)).

- **Per file.** Each file has its own running balances. A file read after another one does not see
  that file's postings: with `-f A -f B`, an assertion in B on an account that only A touched
  fails. The same file given twice is two files, each starting from nothing. Files are processed
  one after the other in `-f` order, each completely (parse, balancing, assertions) before the
  next ([assertion-failures](assertion-failures.md) says what that does to errors).
- **By date.** Inside a file the postings are put in order of their date, and the balance at an
  assertion is the sum of the postings up to and including its own position in that order. A
  transaction written later in the file but dated earlier comes first.
- **A posting's date** is its own date when a `date:` tag or a `[DATE]` (or `[DATE=DATE2]`) in its
  comment gives one (a date with no year takes the transaction's year; `[=DATE2]` gives no
  date), otherwise its transaction's date. So the postings of one transaction can be far apart: a posting
  dated in February is not in the balance on 15 January, and a posting dated in January inside a
  March transaction is. The second date of a transaction (`DATE=DATE2`), `date2:` tags and
  `--date2` change nothing.
- **Ties.** Postings with the same date keep their order in the file: the transaction written
  first, then the posting order inside it. (A posting dated 5 January in a transaction of 1 January
  ties with the transactions of 5 January by where it is written, not by its transaction's date.)
- **Inside a transaction**, so, an assertion on a posting sees the earlier postings of the
  transaction, itself included, and not the later ones:
  `a 5 = 10` then `a 5` fails, `a 5 = 5` then `a 5 = 10` holds. Two assertions on different
  accounts in one transaction are looked at in posting order; the first failing one is reported.
- **Elided amounts** of a transaction are worked out before any assertion is looked at, so an
  assertion after an amount-less posting of the same account sees its amount.
- The balance is the same whatever the kind of posting (`(a)`, `[a]`, flags): see
  [assertions](assertions.md).

## Probe

`probes/assert-order-date.journal`, `probes/assert-order-posting-date.journal`,
`probes/assert-order-posting-date-forms.journal`,
`probes/assert-fail-order-same-date.journal`, `probes/assert-fail-order-in-transaction.journal`,
`probes/assert-fail-order-posting-date.journal`,
`probes/assert-fail-order-posting-date-in-transaction.journal`,
`probes/assert-fail-first-by-date.journal`, `probes/assert-fail-first-by-date-accounts.journal`,
`probes/assert-fail-first-in-transaction.journal`,
`probes/assert-file-a.journal`, `probes/assert-file-b.journal`, `probes/assert-file-c.journal`

Command:

```sh
spec/hledger/assertion-verdicts.sh spec/hledger/probes/assert-order-date.journal \
  spec/hledger/probes/assert-order-posting-date.journal \
  spec/hledger/probes/assert-order-posting-date-forms.journal \
  spec/hledger/probes/assert-fail-order-*.journal \
  spec/hledger/probes/assert-fail-first-*.journal
spec/hledger/assertion-verdicts.sh \
  spec/hledger/probes/assert-file-a.journal+spec/hledger/probes/assert-file-b.journal \
  spec/hledger/probes/assert-file-b.journal+spec/hledger/probes/assert-file-a.journal \
  spec/hledger/probes/assert-file-a.journal+spec/hledger/probes/assert-file-c.journal \
  spec/hledger/probes/assert-file-c.journal+spec/hledger/probes/assert-file-c.journal
```

## Output

`assert-order-date` holds a later-dated transaction first in the file, an amount left out, two transactions on the
same day, and one transaction with a regular, a virtual and a balanced virtual posting on `a`;
`assert-order-posting-date` holds posting dates before and after the transaction's, a second date
and a `date2:` tag; `assert-order-posting-date-forms` the other ways to write a posting date. All three are accepted. The failing journals each break one of the rules above:

```text
assert-order-date | accepts |
assert-order-posting-date | accepts |
assert-order-posting-date-forms | accepts |
assert-fail-order-in-transaction | rejects | assert-fail-order-in-transaction.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 10 EUR; calculated 5 EUR
assert-fail-order-posting-date-in-transaction | rejects | assert-fail-order-posting-date-in-transaction.journal:3:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 8 EUR; calculated 3 EUR
assert-fail-order-posting-date | rejects | assert-fail-order-posting-date.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 6 EUR; calculated 1 EUR
assert-fail-order-same-date | rejects | assert-fail-order-same-date.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 15 EUR; calculated 5 EUR
assert-fail-first-by-date-accounts | rejects | assert-fail-first-by-date-accounts.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 77 EUR; calculated 1 EUR
assert-fail-first-by-date | rejects | assert-fail-first-by-date.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 77 EUR; calculated 1 EUR
assert-fail-first-in-transaction | rejects | assert-fail-first-in-transaction.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 99 EUR; calculated 1 EUR
```

In `assert-fail-order-posting-date-in-transaction` the second posting (line 3) has the earlier
date, so it is looked at first and the first posting is never reached. In the two
`assert-fail-first-by-date*` journals the failing assertion that is written first has the later
date: the reported one is the earlier-dated, whichever account it is on.

Files (the same file twice, `assert-file-c`, holds: its `a 5 = 5` starts from nothing each time):

```text
assert-file-a+assert-file-b | rejects | assert-file-b.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 10 EUR; calculated 0 EUR
assert-file-b+assert-file-a | rejects | assert-file-b.journal:2:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 10 EUR; calculated 0 EUR
assert-file-a+assert-file-c | accepts |
assert-file-c+assert-file-c | accepts |
```

(`assert-file-b` asserts `a = 10 EUR` on 2 January; `assert-file-a` posts the 10 EUR on 1 January.
Read in either order, the file with the assertion never sees the other's postings. The existing
pairs `m1a`+`m1b`, `m2a`+`m1b`, `y8a`+`y8b` of the harness say the same;
[assertion-verdicts](assertion-verdicts.md) has them.)
