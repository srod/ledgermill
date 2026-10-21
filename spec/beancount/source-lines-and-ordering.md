# Source lines and ordering

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

Lines. A transaction's line is the line of its header (the date line). A posting's line is its
own line, wherever comment lines, metadata lines (`key: value`, on the transaction or under a
posting), tags and links lines or blank lines sit around it. Comments, metadata, tags and links are
not reported. The rows of a posting that a reduction splits over several lots all carry the line
of that posting (lots-and-booking.md, `booking-fifo`).

Order. Transactions come out sorted by date, and by source line within a date; other directives
(`open`, ...) do not take part, and a transaction written first with a later date comes last.
ledgermill sorts by date, then by line. Findings are ordered by line.

## Probe

`probes/source-lines.beancount`, `probes/ordering.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/source-lines.beancount
spec/beancount/show.sh spec/beancount/probes/ordering.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

source-lines:

```text
bean-check: (no output, exit 0)
3873b4,transaction,5,2026-01-02,*,,plain
fbb591,transaction,9,2026-01-03,*,,with a comment line and metadata
fd5608,transaction,17,2026-01-04,*,,long header
7303d3,transaction,22,2026-01-05,*,,"the last one,after comments and metadata"
3873b4,Assets:A,,1,EUR,,,,,,,6
3873b4,Assets:B,,-1,EUR,,,,,,,7
fbb591,Assets:A,,1,EUR,,,,,,,12
fbb591,Assets:B,,-1,EUR,,,,,,,15
fd5608,Assets:A,,1,EUR,,,,,,,19
fd5608,Assets:B,,-1,EUR,,,,,,,20
7303d3,Assets:A,,1,EUR,,,,,,,23
7303d3,Assets:B,,-1,EUR,,,,,,,24
```

ordering:

```text
bean-check: (no output, exit 0)
923a76,transaction,13,2026-01-01,*,,"earliest,but after the others in the file"
35e5ef,transaction,4,2026-01-05,*,,"same date,written first"
055bec,transaction,7,2026-01-05,*,,"same date,written second"
c84282,transaction,16,2026-01-05,*,,"same date as the opens,written last"
66d4b6,transaction,1,2026-03-01,*,,"last date,first in the file"
923a76,Assets:C,,1,EUR,,,,,,,14
923a76,Assets:C,,-1,EUR,,,,,,,15
35e5ef,Assets:A,,1,EUR,,,,,,,5
35e5ef,Assets:B,,-1,EUR,,,,,,,6
055bec,Assets:A,,2,EUR,,,,,,,8
055bec,Assets:B,,-2,EUR,,,,,,,9
c84282,Assets:A,,3,EUR,,,,,,,17
c84282,Assets:B,,-3,EUR,,,,,,,18
66d4b6,Assets:A,,1,EUR,,,,,,,2
66d4b6,Assets:B,,-1,EUR,,,,,,,3
```

