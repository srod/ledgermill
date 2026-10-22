# What bean-check reports besides balancing

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

`bean-check` exits 1 when it prints any error, 0 otherwise. This note lists the errors a plausible
journal meets, apart from the ones in balancing.md, lots-and-booking.md and
inferred-cost-and-price.md, pad.md and balance-directive.md. Each entry gives the message, the line it
is reported on, and whether `bean-query` still lists the transaction. In which order several errors
come out, and so which one is first, is in check-order.md. The messages are for `details`; the engine
need not copy their wording.

Not errors (`check-accepted`): a `close` of an account that still holds money, a `note` after the
close, a `price`, `event`, `query` or `custom` that names an unknown account or currency, a
transaction with no posting, one posting with an elided amount. A repeated `option "title"` is not an
error either (`probes/pending/option-title-twice.beancount`; ledgermill reports a parse error on the
second one today, which is wrong, to fix).

## Rule

**Account never opened.** `Invalid reference to unknown account 'A'`. The transaction's *header* line is
reported (not the posting's), once per unknown account of the transaction, so a transaction with two of
them has two errors on one line, in no fixed order. The directives that name an account are checked the
same way, each on its own line: `balance` (twice, balance-directive.md), `pad` (the directive and its
transaction, pad.md), `note`, and `document` (also when its file exists). bean-query lists the
transaction (`check-account-unknown`, `check-directive-accounts`).

**Account not active.** `Invalid reference to inactive account 'A'`, on the same lines as above, for a
date before the account's `open`, in any directive that names it (transaction, `pad` and its
transaction, `note`, `document`, `balance`), and for a date after its `close`, in a transaction or a
`pad` only: a `balance`, `note` or `document` dated after a `close` is accepted
(`check-directive-dates`). The day of the `open` and the day of the `close` are both fine, whatever
the order of the lines (`check-account-open-close-days`: a transaction written before the `open` of
its own day passes, and so does one written after the `close` of its own day). A `balance` before the
`open` is reported once, by the balance pass (check-order.md), unlike an unknown account. bean-query
lists the transaction (`check-account-dates`).

**Duplicate open.** `Duplicate open directive for A` on the later `open` in (date, line) order, for
each further one: three opens give two errors; an `open` after a `close` is one too
(`check-open-duplicate`).

**Close.** `Unopened account A is being closed` for a `close` of an account with no `open`, or whose
`open` is dated after the `close`; `Duplicate close directive for A` on the later of two closes (by
date, then line) (`check-close-errors`).

**Currency not allowed.** `Invalid currency C for account 'A'` when an account whose `open` lists
currencies gets a posting in another (units only: cost and price currencies are not checked,
`check-currency-constraint-ok`). It counts for an elided posting, whose currency is worked out, and for
a zero amount. One error per offending posting, all on the transaction's header line; for padding on
the `pad` line. bean-query lists the transaction (`check-currency-constraint`).

**Missing document.** `File does not exist: "PATH"` on the `document` line when the file is not there;
a relative path is read from the directory of the journal and shows absolute in the message. An existing
file is accepted, with tags, links and metadata (`check-document-missing`, `check-document-existing`).
ledgermill cannot look at files: it reads a `document` and reports `ignored-syntax` (warning).

**Duplicate commodity.** `Duplicate commodity directives for 'C'` on the later one (`check-commodity-duplicate`).

**Tags and metadata** (`check-tags-and-meta`), found while reading the file:
`poptag` of a tag not pushed, `Attempting to pop absent tag: 'T'`, on the `poptag` line; `popmeta` likewise
(`Attempting to pop absent metadata key: 'K'`); a `pushtag` or `pushmeta` still open at the end of the
file, `Unbalanced pushed tag: 'T'` / `Unbalanced metadata key 'K'; leftover metadata 'V'`, on **line 0**;
the same key twice in a transaction (`Duplicate metadata field on entry: ...`) or under a posting
(`Duplicate posting metadata field: ...`), both on the transaction's header line. The transaction is
still listed.

**Account names** (`check-account-names`). The root must be one of `Assets`, `Liabilities`, `Equity`,
`Income`, `Expenses`, else `Invalid account name: N` on the line of the name (the posting's own line
inside a transaction, so `balance`, `pad`, `note`, `close` each on their line); so is a component that
starts with a non-ASCII lowercase letter (`Assets:é`). A name the tokenizer cannot read at all gives
`Invalid token` instead: an ASCII lowercase component (`Assets:bank`), a lone root (`Assets`), a double
colon, an underscore. Non-ASCII uppercase letters (`Assets:Ünï`), digits (`Assets:1Fine`) and `-` inside
a component are fine. A transaction naming a bad account is still listed.

**Syntax** (`check-parse-errors`). One `Invalid token: 'X'` per unreadable word (a line of six words
gives six errors on its line), `syntax error, unexpected X, expecting ...` for a directive that stops
early (an incomplete line is reported on the *next* line when a newline follows, `unexpected EOL`; on
its own line at the end of a file without final newline, `unexpected end of file`),
`ValueError: day is out of range for month` for an impossible date. What bean-query lists afterwards
depends on how the parser recovers: here the transaction right before the bad line is gone. ledgermill
returns no transaction for a file with a parse error (spec/README.md).

**Open with a bad booking method.** `Invalid booking method: M` on the `open` line (ledgermill:
`unsupported-booking-method`, lots-and-booking.md).

## Probe

`probes/check-account-unknown.beancount`, `probes/check-account-dates.beancount`,
`probes/check-account-open-close-days.beancount`, `probes/check-directive-dates.beancount`,
`probes/check-open-duplicate.beancount`,
`probes/check-close-errors.beancount`, `probes/check-currency-constraint.beancount`,
`probes/check-currency-constraint-ok.beancount`, `probes/check-directive-accounts.beancount`,
`probes/check-document-missing.beancount`, `probes/check-document-existing.beancount`,
`probes/check-commodity-duplicate.beancount`, `probes/check-tags-and-meta.beancount`,
`probes/check-account-names.beancount`, `probes/check-parse-errors.beancount`,
`probes/check-accepted.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/check-account-unknown.beancount
spec/beancount/show.sh spec/beancount/probes/check-account-dates.beancount
spec/beancount/show.sh spec/beancount/probes/check-account-open-close-days.beancount
spec/beancount/show.sh spec/beancount/probes/check-directive-dates.beancount
spec/beancount/show.sh spec/beancount/probes/check-open-duplicate.beancount
spec/beancount/show.sh spec/beancount/probes/check-close-errors.beancount
spec/beancount/show.sh spec/beancount/probes/check-currency-constraint.beancount
spec/beancount/show.sh spec/beancount/probes/check-currency-constraint-ok.beancount
spec/beancount/show.sh spec/beancount/probes/check-directive-accounts.beancount
spec/beancount/show.sh spec/beancount/probes/check-document-missing.beancount
spec/beancount/show.sh spec/beancount/probes/check-document-existing.beancount
spec/beancount/show.sh spec/beancount/probes/check-commodity-duplicate.beancount
spec/beancount/show.sh spec/beancount/probes/check-tags-and-meta.beancount
spec/beancount/show.sh spec/beancount/probes/check-account-names.beancount
spec/beancount/show.sh spec/beancount/probes/check-parse-errors.beancount
spec/beancount/show.sh spec/beancount/probes/check-accepted.beancount
```

`show.sh` prints one row per posting and one per transaction; here only the `bean-check:` lines and the transaction rows `id,type,line,date,flag,payee,narration` are kept. `bean-check:` lines are the errors it printed (the copy of the directive after each is left out), in its order; `(no output, exit 0)` means it accepts.

## Output

check-account-unknown:

```text
bean-check: check-account-unknown.beancount:3: Invalid reference to unknown account 'Expenses:Food'
bean-check: check-account-unknown.beancount:6: Invalid reference to unknown account 'Expenses:Food'
2e2d53,transaction,3,2026-01-02,*,,a posting to an account that was never opened
8ad856,transaction,6,2026-01-03,*,,"still listed after the error,with its other posting"
```

check-account-dates:

```text
bean-check: check-account-dates.beancount:6: Invalid reference to inactive account 'Expenses:Food'
bean-check: check-account-dates.beancount:9: Invalid reference to inactive account 'Expenses:Late'
8b5919,transaction,6,2026-01-02,*,,before the open date of Expenses:Food
d5b982,transaction,9,2026-01-03,*,,after the close of Expenses:Late
```

check-account-open-close-days:

```text
bean-check: (no output, exit 0)
bdc8ef,transaction,1,2026-01-01,*,,written before the open directives of its own date
9fdd1e,transaction,7,2026-01-02,*,,written after the close of its own date
```

check-directive-dates:

```text
bean-check: check-directive-dates.beancount:8: Invalid reference to inactive account 'Assets:Late'
bean-check: check-directive-dates.beancount:6: Invalid reference to inactive account 'Assets:Late'
bean-check: check-directive-dates.beancount:7: Invalid reference to inactive account 'Assets:Late'
bean-check: check-directive-dates.beancount:12: Invalid reference to inactive account 'Assets:Closed'
bean-check: check-directive-dates.beancount:12: Invalid reference to inactive account 'Assets:Closed'
c7f68b,transaction,12,2026-01-04,P,,(Padding inserted for Balance of 5 EUR for difference 5 EUR)
```

check-open-duplicate:

```text
bean-check: check-open-duplicate.beancount:6: Duplicate open directive for Assets:C
bean-check: check-open-duplicate.beancount:7: Duplicate open directive for Assets:C
bean-check: check-open-duplicate.beancount:2: Duplicate open directive for Assets:A
bean-check: check-open-duplicate.beancount:10: Duplicate open directive for Assets:D
bean-check: check-open-duplicate.beancount:3: Duplicate open directive for Assets:B
```

check-close-errors:

```text
bean-check: check-close-errors.beancount:6: Unopened account Assets:B is being closed
bean-check: check-close-errors.beancount:3: Duplicate close directive for Assets:A
bean-check: check-close-errors.beancount:4: Unopened account Assets:Never is being closed
bean-check: check-close-errors.beancount:8: Duplicate close directive for Assets:C
```

check-currency-constraint:

```text
bean-check: check-currency-constraint.beancount:6: Invalid currency USD for account 'Expenses:Food'
bean-check: check-currency-constraint.beancount:9: Invalid currency EUR for account 'Assets:Usd'
bean-check: check-currency-constraint.beancount:12: Invalid currency GBP for account 'Assets:Bank'
5d73a1,transaction,6,2026-01-03,*,,posting in a currency the account does not allow
52b973,transaction,9,2026-01-04,*,,an elided posting gets the other currency
a1117d,transaction,12,2026-01-05,*,,a zero amount counts too
```

check-currency-constraint-ok:

```text
bean-check: (no output, exit 0)
055374,transaction,4,2026-01-02,*,,the cost currency is not checked
dcb2bb,transaction,7,2026-01-03,*,,nor the price currency
```

check-directive-accounts:

```text
bean-check: check-directive-accounts.beancount:4: Invalid reference to unknown account 'Assets:Never'
bean-check: check-directive-accounts.beancount:5: Invalid reference to unknown account 'Assets:Never'
```

check-document-missing:

```text
bean-check: check-document-missing.beancount:2: File does not exist: "/nonexistent/statement.pdf"
```

check-document-existing:

```text
bean-check: (no output, exit 0)
db7e89,transaction,7,2026-01-07,*,,a transaction after them
```

check-commodity-duplicate:

```text
bean-check: check-commodity-duplicate.beancount:2: Duplicate commodity directives for 'EUR'
bean-check: check-commodity-duplicate.beancount:5: Duplicate commodity directives for 'USD'
```

check-tags-and-meta:

```text
bean-check: check-tags-and-meta.beancount:1: Attempting to pop absent tag: 'never-pushed'
bean-check: check-tags-and-meta.beancount:4: Attempting to pop absent metadata key: 'other'
bean-check: check-tags-and-meta.beancount:8: Duplicate metadata field on entry: KeyValue(key='note', value='b')
bean-check: check-tags-and-meta.beancount:8: Duplicate posting metadata field: KeyValue(key='memo', value='b')
bean-check: check-tags-and-meta.beancount:0: Unbalanced pushed tag: 'left-open'
bean-check: check-tags-and-meta.beancount:0: Unbalanced metadata key 'trip'; leftover metadata 'x'
ffbe6a,transaction,8,2026-01-02,*,,the same metadata key twice
```

check-account-names:

```text
bean-check: check-account-names.beancount:5: Invalid account name: Asset:Typo
bean-check: check-account-names.beancount:6: Invalid account name: Assets:é
bean-check: check-account-names.beancount:7: Invalid token: 'Assets:bank'
bean-check: check-account-names.beancount:8: Invalid token: 'Assets'
bean-check: check-account-names.beancount:9: Invalid token: 'Assets::Double'
bean-check: check-account-names.beancount:11: Invalid account name: Asset:Typo
bean-check: check-account-names.beancount:13: Invalid account name: Asset:Typo
bean-check: check-account-names.beancount:13: Balance failed for 'Asset:Typo': expected 0 EUR != accumulated 1 EUR (1 too much)
c04503,transaction,10,2026-01-02,*,,the root is checked on every use
```

check-parse-errors:

```text
bean-check: check-parse-errors.beancount:7: Invalid token: 'this'
bean-check: check-parse-errors.beancount:7: Invalid token: 'line'
bean-check: check-parse-errors.beancount:7: Invalid token: 'is'
bean-check: check-parse-errors.beancount:7: Invalid token: 'not'
bean-check: check-parse-errors.beancount:7: Invalid token: 'a'
bean-check: check-parse-errors.beancount:7: Invalid token: 'directive'
bean-check: check-parse-errors.beancount:8: ValueError: day is out of range for month
bean-check: check-parse-errors.beancount:9: Invalid token: 'eur'
bean-check: check-parse-errors.beancount:14: syntax error, unexpected EOL
00df30,transaction,10,2026-01-04,*,,after the errors
```

check-accepted:

```text
bean-check: (no output, exit 0)
d239d7,transaction,4,2026-01-02,*,,"one posting,amount elided"
434e42,transaction,6,2026-01-03,*,,no posting at all
8e6682,transaction,11,2026-01-05,*,,money before the close
```

