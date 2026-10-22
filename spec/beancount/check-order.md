# The order bean-check reports errors in

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Status: **built**. `check()` returns the Beancount findings in this order, not in line order. Each error that `bean-check` prints
twice is one finding. The `Transaction does not balance` error is last, and a `document` warning comes before every error.

`bean-check` prints its errors in the order they are found, not in line order, and the harness (and
`check()`) is judged on the first one: its file and line, besides the verdict. The reference loads a
file in steps, each step runs over everything before the next starts, and each step has its own order.

## Rule

**Entry order.** Where a step "walks the entries", it walks them sorted by date and, within a date,
by this group and then by line:

1. `open`;
2. `balance`;
3. everything else (transaction, `pad`, `note`, `price`, `event`, `commodity`, `custom`, ...), in line
   order; a padding transaction comes straight after its `pad`, with the pad's line;
4. `close` and `document`.

So a balance is checked before the transactions of its day, an `open` takes effect before the day's
transactions, and a `close` after them. `pad` and `balance` are in different groups: a pad on the day of
the balance comes after it (pad.md). The same order sorts the transactions list (source-lines-and-ordering.md).

**Steps**, in the order the errors come out:

1. **Parsing**, file order: unreadable tokens, syntax errors, invalid dates, invalid account names,
   bad `open` booking method, duplicate metadata keys, `poptag`/`popmeta` of nothing. Then, last,
   the tag and metadata pushes still open at the end of the file (reported on line 0).
2. **Booking**, walking the transactions: every error of balancing.md,
   lots-and-booking.md and inferred-cost-and-price.md except `Transaction does not balance`, which
   comes later. A transaction that fails here is dropped for everything after (balance and pad
   sums, and bean-query).
3. **Documents**, walking the entries: `File does not exist`.
4. **Pad**, walking the entries: first, in walking order, "Attempt to pad an entry with cost" (on
   the balance line); then, after the walk, every `Unused Pad entry`, in pad order (date, then line).
   The padding transactions are added here.
5. **Balance assertions**, walking the entries (so padding is counted): for each `balance`, in this
   order, "Invalid reference to unknown account" or "Invalid reference to inactive account" (before
   the `open` date only), "Invalid currency 'C' for Balance directive", then "Balance failed".
6. **Validation**, one pass after the other, each walking the entries:
   1. `open`/`close`: `Duplicate open directive`, `Unopened account ... is being closed`, `Duplicate close
      directive` (so an `open` is looked at before a `close` of the same day);
   2. accounts used: unknown and inactive references, of every transaction, `pad` (the directive, then
      its transaction), `note`, `document` and `balance` (an unknown account only);
   3. currencies an account allows: `Invalid currency C for account`;
   4. `Duplicate balance assertion with different amounts`;
   5. `Duplicate commodity directives`;
   6. `Transaction does not balance`.

Within one entry, errors of the same kind (two unknown accounts of a transaction) come in no fixed
order but share their line. The first error of a journal is therefore: the first parsing error, else
the first booking error, and so on down the list; the line is the one each error is reported on
(check-errors.md, balance-directive.md, pad.md).

The step 1 errors are in file order only among themselves: what the parser does after a syntax error
is not predictable (check-errors.md), so for a journal with a parse error the first error is the
only one to rely on.

## Probe

`probes/check-order-phases.beancount`, `probes/check-order-dates.beancount`, and the existing
`probes/balance-directive-fails.beancount`, `probes/check-open-duplicate.beancount`,
`probes/pad-unused.beancount`, `probes/pad-account-errors.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/check-order-phases.beancount
spec/beancount/show.sh spec/beancount/probes/check-order-dates.beancount
spec/beancount/show.sh spec/beancount/probes/balance-directive-fails.beancount
spec/beancount/show.sh spec/beancount/probes/check-open-duplicate.beancount
spec/beancount/show.sh spec/beancount/probes/pad-unused.beancount
spec/beancount/show.sh spec/beancount/probes/pad-account-errors.beancount
```

Only the `bean-check:` lines are kept below (the errors it printed, in its order, without the copy of
the directive after each).

## Output

`check-order-phases` has one error per step and its blocks are written in the reverse order of the
steps, so the lines of the output go down: 39 (parsing), 0 (parsing, end of file), 38 (booking), 34
(documents), 33 (unused pad), 32 (balance), 31 (duplicate open), 28 (inactive account), 25 (currency),
24 (duplicate assertion), 19 (duplicate commodity), 15 (does not balance).

check-order-phases:

```text
bean-check: check-order-phases.beancount:39: Invalid account name: Foo:Bad
bean-check: check-order-phases.beancount:0: Unbalanced pushed tag: 'left-open'
bean-check: check-order-phases.beancount:38: You may not have more than one auto-posting per currency
bean-check: check-order-phases.beancount:34: File does not exist: "/nonexistent/statement.pdf"
bean-check: check-order-phases.beancount:33: Unused Pad entry
bean-check: check-order-phases.beancount:32: Balance failed for 'Assets:Bf': expected 9 EUR != accumulated 0 EUR (9 too little)
bean-check: check-order-phases.beancount:31: Duplicate open directive for Assets:Other
bean-check: check-order-phases.beancount:28: Invalid reference to inactive account 'Assets:Late'
bean-check: check-order-phases.beancount:25: Invalid currency USD for account 'Assets:Eur'
bean-check: check-order-phases.beancount:24: Duplicate balance assertion with different amounts
bean-check: check-order-phases.beancount:19: Duplicate commodity directives for 'ZZZ'
bean-check: check-order-phases.beancount:15: Transaction does not balance: (-1 EUR)
```

`check-order-dates`: within a step the entries come by date (the later `UnLater` is last although
written first), and on the day 2026-02-05 in the order of the groups: the balance (line 18), then the
rest by line (pad 13 twice, transaction 14, note 17), then the document (11). In step 6.1 the duplicate
`open` (line 20) is before the unopened `close` (line 12) of the same day. The first two lines are step
5 (balance on an unknown account), the others step 6.2 (the balance again, an unknown account only).

check-order-dates:

```text
bean-check: check-order-dates.beancount:18: Invalid reference to unknown account 'Assets:UnBal'
bean-check: check-order-dates.beancount:21: Invalid reference to unknown account 'Assets:UnPad'
bean-check: check-order-dates.beancount:20: Duplicate open directive for Assets:Dup
bean-check: check-order-dates.beancount:12: Unopened account Assets:UnClose is being closed
bean-check: check-order-dates.beancount:7: Invalid reference to unknown account 'Assets:UnEarlier'
bean-check: check-order-dates.beancount:18: Invalid reference to unknown account 'Assets:UnBal'
bean-check: check-order-dates.beancount:13: Invalid reference to unknown account 'Assets:UnPad'
bean-check: check-order-dates.beancount:13: Invalid reference to unknown account 'Assets:UnPad'
bean-check: check-order-dates.beancount:14: Invalid reference to unknown account 'Assets:UnTxn'
bean-check: check-order-dates.beancount:17: Invalid reference to unknown account 'Assets:UnNote'
bean-check: check-order-dates.beancount:11: Invalid reference to unknown account 'Assets:UnDoc'
bean-check: check-order-dates.beancount:21: Invalid reference to unknown account 'Assets:UnPad'
bean-check: check-order-dates.beancount:4: Invalid reference to unknown account 'Assets:UnLater'
```

`balance-directive-fails`, `check-open-duplicate`, `pad-unused`, `pad-account-errors` (the first line
is a step 5 error, line 17, although lines 10 to 12 hold earlier ones):

balance-directive-fails:

```text
bean-check: balance-directive-fails.beancount:29: Balance failed for 'Assets:G': expected 5 USD != accumulated 0 USD (5 too little)
bean-check: balance-directive-fails.beancount:28: Balance failed for 'Assets:F': expected -500.00 EUR != accumulated 500 EUR (1000.00 too much)
bean-check: balance-directive-fails.beancount:27: Balance failed for 'Assets:E': expected 500 EUR != accumulated 500 EUR (0 too little)
bean-check: balance-directive-fails.beancount:26: Balance failed for 'Assets:D': expected 500 EUR != accumulated 500.51 EUR (0.51 too much)
bean-check: balance-directive-fails.beancount:25: Balance failed for 'Assets:C': expected 500 EUR != accumulated 500.0001 EUR (0.0001 too much)
bean-check: balance-directive-fails.beancount:24: Balance failed for 'Assets:B': expected 500.00 EUR != accumulated 499.989 EUR (0.011 too little)
bean-check: balance-directive-fails.beancount:23: Balance failed for 'Assets:A': expected 500.00 EUR != accumulated 500.011 EUR (0.011 too much)
bean-check: balance-directive-fails.beancount:30: Balance failed for 'Assets:H': expected 500.000 EUR != accumulated 500.0011 EUR (0.0011 too much)
bean-check: balance-directive-fails.beancount:31: Balance failed for 'Assets:I': expected 500.0 EUR != accumulated 500.11 EUR (0.11 too much)
```

check-open-duplicate:

```text
bean-check: check-open-duplicate.beancount:6: Duplicate open directive for Assets:C
bean-check: check-open-duplicate.beancount:7: Duplicate open directive for Assets:C
bean-check: check-open-duplicate.beancount:2: Duplicate open directive for Assets:A
bean-check: check-open-duplicate.beancount:10: Duplicate open directive for Assets:D
bean-check: check-open-duplicate.beancount:3: Duplicate open directive for Assets:B
```

pad-unused:

```text
bean-check: pad-unused.beancount:9: Unused Pad entry
bean-check: pad-unused.beancount:13: Unused Pad entry
bean-check: pad-unused.beancount:18: Unused Pad entry
bean-check: pad-unused.beancount:20: Unused Pad entry
bean-check: pad-unused.beancount:23: Unused Pad entry
bean-check: pad-unused.beancount:28: Unused Pad entry
```

pad-account-errors:

```text
bean-check: pad-account-errors.beancount:17: Invalid currency 'USD' for Balance directive: 
bean-check: pad-account-errors.beancount:19: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:10: Invalid reference to unknown account 'Equity:Missing'
bean-check: pad-account-errors.beancount:10: Invalid reference to unknown account 'Equity:Missing'
bean-check: pad-account-errors.beancount:12: Invalid reference to inactive account 'Equity:Closed'
bean-check: pad-account-errors.beancount:12: Invalid reference to inactive account 'Equity:Closed'
bean-check: pad-account-errors.beancount:18: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:18: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:19: Invalid reference to unknown account 'Assets:Never'
bean-check: pad-account-errors.beancount:14: Invalid currency EUR for account 'Equity:Usd'
bean-check: pad-account-errors.beancount:16: Invalid currency USD for account 'Assets:D'
```

