# Postings

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Status: **built**. padding rows are in the list as described.

## Rule

`postings()` is a flat list: one row per booked posting, in the order this query gives:

```text
SELECT date, flag, payee, narration, account, number, currency, filename, lineno ORDER BY date, filename, lineno
```

Fields. `date` and `flag` are the transaction's (the flag of a posting is not reported here, and the
word `txn` reads as `*`); `payee` is empty when the header has one string or none, `narration` is
empty for no string or `""`; one string is the narration, two are payee then narration. `number` is
the posting's units, with the places as printed (`2.50`), or as worked out for an elided posting
(`-2.50`); bean-query pads it with spaces, which are not part of the number. `lineno` is the line
of the posting itself, as in source-lines-and-ordering.md.

Rows.

- A transaction without postings gives no row.
- A posting booked against several lots gives one row per lot, all with the posting's line.
  Their order is the order the lots were consumed: oldest first for FIFO, newest first for LIFO,
  dearest first for HIFO. The sort keeps it (it is stable), so rows tied on date, file and line
  keep booking order, which is the order `transactions()` gives them.
- A transaction that bean-check rejects gives no row (its errors are printed before the CSV).
- A padding transaction (pad.md) gives two rows, the padded account then the source account, both on the
  line of the `balance` that caused it, so they sort there and not at the `pad`.

Order. By date, then file, then line. Transactions come out in source order within a date
(source-lines-and-ordering.md) and a posting's line lies inside its transaction, so rows of
different transactions never interleave: several transactions on one date come out in source order,
and a transaction written last with an earlier date comes first. Within a transaction the rows
follow the line, that is the order the postings were written: bean-query's unsorted postings list
groups them by currency instead (numbers-and-interpolation.md), and this query puts them back
(`interpolation-groups`, below). Rows tied on a line keep their booking order. Directives other
than transactions take no part. A Beancount file is its own ledger, so within it the file never breaks a
tie; several ledgers are merged by date and then by the file's place in the manifest (several-files.md).

## Probe

`probes/postings-lot-ties.beancount`, `probes/postings-order.beancount`, and the existing
`probes/interpolation-groups.beancount`

Command:

```sh
spec/beancount/show-postings.sh spec/beancount/probes/postings-lot-ties.beancount
spec/beancount/show-postings.sh spec/beancount/probes/postings-order.beancount
spec/beancount/show-postings.sh spec/beancount/probes/interpolation-groups.beancount
```

`show-postings.sh` runs the query above and cleans the output for reading (CRs and padding
dropped, directory cut from the file name, here shown as `F`). `bean-check:` output is empty
(exit 0) for both journals.

## Output

postings-lot-ties (the buys on 2026-01-02 to 04 are cut; sizes 1, 2, 3 at costs 30, 10, 20, so
each sale row shows its lot):

```text
2026-01-10,*,,sell all six in each account,Assets:Lifo,-3,BTC,F,24
2026-01-10,*,,sell all six in each account,Assets:Lifo,-2,BTC,F,24
2026-01-10,*,,sell all six in each account,Assets:Lifo,-1,BTC,F,24
2026-01-10,*,,sell all six in each account,Assets:Hifo,-1,BTC,F,25
2026-01-10,*,,sell all six in each account,Assets:Hifo,-3,BTC,F,25
2026-01-10,*,,sell all six in each account,Assets:Hifo,-2,BTC,F,25
2026-01-10,*,,sell all six in each account,Assets:Fifo,-1,BTC,F,26
2026-01-10,*,,sell all six in each account,Assets:Fifo,-2,BTC,F,26
2026-01-10,*,,sell all six in each account,Assets:Fifo,-3,BTC,F,26
2026-01-10,*,,sell all six in each account,Assets:Bank,330,EUR,F,27
```

LIFO gives 3, 2, 1 (20, 10, 30 by date, newest first); HIFO gives 1, 3, 2 (cost 30, 20, 10); FIFO
gives 1, 2, 3.

postings-order:

```text
2026-01-04,P,,"written last,earlier date,other flag",Assets:A,8,EUR,F,28
2026-01-04,P,,"written last,earlier date,other flag",Assets:B,-8,EUR,F,29
2026-01-05,*,Shop,"same date,written first",Assets:A,1,EUR,F,6
2026-01-05,*,Shop,"same date,written first",Assets:B,-1,EUR,F,7
2026-01-05,!,,"same date,no payee,pending",Assets:A,2.50,EUR,F,9
2026-01-05,!,,"same date,no payee,pending",Assets:B,-2.50,EUR,F,10
2026-01-05,*,,"same date,txn word",Assets:A,3,EUR,F,13
2026-01-05,*,,"same date,txn word",Assets:B,-3,EUR,F,14
2026-01-05,*,,"same date,flag on a posting",Assets:A,4,EUR,F,16
2026-01-05,*,,"same date,flag on a posting",Assets:B,-4,EUR,F,17
2026-01-05,*,,,Assets:A,5,EUR,F,19
2026-01-05,*,,,Assets:B,-5,EUR,F,20
2026-01-05,*,,,Assets:A,6,EUR,F,22
2026-01-05,*,,,Assets:B,-6,EUR,F,23
2026-01-05,*,Payee only?,,Assets:A,7,EUR,F,25
2026-01-05,*,Payee only?,,Assets:B,-7,EUR,F,26
2026-01-06,*,,buy two lots same day,Assets:Stock,1,BTC,F,31
2026-01-06,*,,buy two lots same day,Assets:Stock,1,BTC,F,32
2026-01-06,*,,buy two lots same day,Assets:A,-30.50,EUR,F,33
2026-01-07,!,,"sell both,with a posting before and two after",Assets:A,30.50,EUR,F,35
2026-01-07,!,,"sell both,with a posting before and two after",Assets:Stock,-1,BTC,F,36
2026-01-07,!,,"sell both,with a posting before and two after",Assets:Stock,-1,BTC,F,36
2026-01-07,!,,"sell both,with a posting before and two after",Assets:B,1,EUR,F,37
2026-01-07,!,,"sell both,with a posting before and two after",Assets:A,-1,EUR,F,38
```

- The transaction written last (line 28, `P`) comes first, being dated 2026-01-04.
- The `txn` word reads as `*`; the `!` on a posting (line 16) does not show: the row has the
  transaction's flag.
- The transaction with no postings (line 11) has no row.
- Lines 31 and 32: two postings, one lot each, two rows. Line 36: one posting, two lots, two rows
  with the same line, between the postings at lines 35 and 37.

interpolation-groups, last two transactions (the unsorted list gives their lines as 15, 17, 16,
18 and 20, 22, 21, 23, grouped by currency; here they are in line order):

```text
2026-01-04,*,,a price puts the posting in the price currency's group,Assets:A,2,BTC,F,15
2026-01-04,*,,a price puts the posting in the price currency's group,Assets:A,5,EUR,F,16
2026-01-04,*,,a price puts the posting in the price currency's group,Assets:B,-200,USD,F,17
2026-01-04,*,,a price puts the posting in the price currency's group,Assets:B,-5,EUR,F,18
2026-01-05,*,,a cost does the same,Assets:A,5,EUR,F,20
2026-01-05,*,,a cost does the same,Assets:A,2,BTC,F,21
2026-01-05,*,,a cost does the same,Assets:B,-5,EUR,F,22
2026-01-05,*,,a cost does the same,Assets:B,-200,USD,F,23
```
