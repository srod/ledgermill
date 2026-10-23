# Spec notes

One note per behaviour of the reference tools. Code is written from these notes, not from the
reference tools' source (see [CONTRIBUTING.md](../CONTRIBUTING.md)).

Each note has:

- **Rule**: what the tool does, in our own words;
- **Probe**: the journal we wrote (under `probes/`), run unchanged;
- **Command**: exactly what was run;
- **Reference**: the tool and version;
- **Output**: the part of the output that shows the rule.

## Layout

- `hledger/` – notes on hledger. The commands use `hledger/show.sh`, which runs
  `hledger -f FILE print -O json` and keeps the fields ledgermill compares, one transaction per
  line. An amount is `[commodity, decimalMantissa, decimalPlaces]`, then its price as
  `["@"|"@@", commodity, mantissa, places]` and its lot as `{cost, date, label}` when it has them.
  `hledger/show.sh` and the other helpers pass any argument that starts with `-` to `hledger print`
  (`show.sh --cost FILE`, `show.sh --value=2026-01-01,EUR FILE`). `hledger/show-amounts.sh` prints the same
  transactions in a short form, one line each, amounts as decimal numbers (the valuation notes use it), and
  `hledger/show-tags.sh` the tags hledger puts on postings (`_conversion-posting`, `_cost-posting`).
  `hledger/show-postings.sh` prints the rows of `print --location -O csv`, one line each (the flat postings note uses it).
  `hledger/show-prices.sh` prints the `P` lines `prices` lists, as Plainledger's two runs read them (the prices note uses it),
  and `hledger/show-holdings.sh` the exact amounts per account of `balance --real`, from `-O json` (the holdings note uses it).
- `hledger/probes/` – the probe journals.
- `hledger/show-assertions.sh` prints the assertions hledger reads (position, asserted amount, flags, the
  amounts an assignment got). `hledger/assertion-verdicts.sh` prints hledger's verdict on every journal with an
  assertion or assignment, and for a rejected one the file, line and kind of its first error.
- `hledger/probe-verdicts.sh` prints hledger's verdict and first error (file, line, column) on probe journals
  (by default the `rule-*` ones); `hledger/period-verdicts.sh` does the same for period expressions, and
  `hledger/period-forms.txt` is its output on a corpus of 474 of them. `hledger/directive-verdicts.sh` does
  it for directive lines (`hledger/directive-forms.txt`, 200 of them), and `hledger/check-vs-print.sh` compares
  `hledger check` with `print` on the whole corpus.
- `hledger/probes/refused/` holds the hledger probes ledgermill refuses (not run by the harness).
- `hledger/prototype-observations.md` – the notes of our earlier TypeScript prototype, kept as a
  starting point. They are our own text, written from running hledger. Its valuation sections (`--cost`, `--value=DATE,COMM`) are superseded by value-prices, value-arithmetic and cost.

- `beancount/` – notes on Beancount. The commands use `beancount/show.sh`, which prints what
  `bean-check` says and, from `bean-query -f csv`, one row per transaction and one per posting
  (the two queries the harness uses). Postings are `id,account,posting_flag,number,currency,
  cost_number,cost_currency,cost_date,cost_label,price_number,price_currency,filename,lineno`;
  `id` is shortened to 6 characters and shared by the postings of a transaction. A price is the
  per-unit price, a cost the lot's cost per unit.
- `beancount/probes/` – the probe journals; `beancount/probes/refused/` those ledgermill refuses (not run by the harness).
  `beancount/verdicts.sh` prints bean-check's verdict and first error for every journal of the corpus.

## Index (hledger 1.52.4)

| Note | Behaviour |
|---|---|
| [amount-forms](hledger/amount-forms.md) | where the commodity goes, signs, spaces, symbol characters |
| [numbers](hledger/numbers.md) | decimal marks, digit groups, exponents, exactness, big values |
| [decimal-places](hledger/decimal-places.md) | quantities keep their written places; ten-place output |
| [directives-numbers](hledger/directives-numbers.md) | `decimal-mark`, `commodity`, `D`: which mark a lone separator is |
| [directives-other](hledger/directives-other.md) | `account`, `P`, `comment` blocks and the directives read and ignored |
| [aliases-and-parents](hledger/aliases-and-parents.md) | `alias`, regex aliases, `apply account` |
| [dates](hledger/dates.md) | date forms, `Y`, secondary dates |
| [transaction-header](hledger/transaction-header.md) | status, code, description, comment on the header line |
| [comments](hledger/comments.md) | where comments attach, trimming, what ends a transaction |
| [comment-dates](hledger/comment-dates.md) | `date:` tags and bracketed dates in posting comments |
| [source-lines](hledger/source-lines.md) | the reported line, CRLF, BOM, tabs, missing final newline |
| [posting-lines](hledger/posting-lines.md) | account/amount separator, elided amounts, markers |
| [posting-star-line](hledger/posting-star-line.md) | a posting line of only `*` ends the transaction like a comment line |
| [postings](hledger/postings.md) | posting status, virtual and balanced virtual postings, account names |
| [prices-and-lots](hledger/prices-and-lots.md) | `@`, `@@`, `{}`, `{{}}`, `[date]`, `(label)` |
| [balancing](hledger/balancing.md) | groups, what must sum to zero, tolerance |
| [elided-amounts](hledger/elided-amounts.md) | what an amount-less posting gets, and its places |
| [inferred-prices](hledger/inferred-prices.md) | the price given when two commodities do not balance |
| [conversion](hledger/conversion.md) | conversion accounts, `type:` tags, pairs that balance by quantity |
| [ordering](hledger/ordering.md) | output order, several files |
| [assertion-syntax](hledger/assertion-syntax.md) | `=`, `==`, `=*`, `==*`: spacing, the asserted amount, what is a parse error |
| [assertions](hledger/assertions.md) | what an assertion compares: commodities, sub-accounts, exactness, costs |
| [assertion-order](hledger/assertion-order.md) | the order of the running balances: dates, posting dates, files |
| [assignments](hledger/assignments.md) | the amount an assignment gets: places, `==`, `=*`, the balance afterwards |
| [assignment-lots-and-costs](hledger/assignment-lots-and-costs.md) | lots on the amounts `==` zeroes, a price in the asserted amount |
| [assignment-processing](hledger/assignment-processing.md) | balancing, elided amounts, plain vs assignment transactions, what is refused |
| [assertion-failures](hledger/assertion-failures.md) | the line blamed, which failure and which error comes first, `-I` |
| [assertion-verdicts](hledger/assertion-verdicts.md) | hledger's verdict and first error on every corpus journal with an assertion |
| [rules-ignored](hledger/rules-ignored.md) | `=` and `~` rules: what is read, what is an error, period expressions, no effect on the rest |
| [syntax](hledger/syntax.md) | every construct of the journal format: supported, ignored with a warning, or refused (the published list) |
| [check-vs-print](hledger/check-vs-print.md) | `hledger check` gives print's verdict and first error; which checks it runs by default |
| [value-prices](hledger/value-prices.md) | `--value=DATE,COMM`: which `P` lines exist on DATE, the path of prices, inverse rates |
| [value-arithmetic](hledger/value-arithmetic.md) | `--value=DATE,COMM`: products, rounding, cost and lot of an amount, merged amounts of a posting |
| [cost](hledger/cost.md) | `--cost`: unit and total costs, lots, inferred prices, a posting with several amounts, which conversion postings are dropped |
| [postings-flat](hledger/postings-flat.md) | `print --location -O csv`: one row per posting and commodity, row order, header line range, virtual postings, costs, elided amounts, the posting's own line |
| [prices-list](hledger/prices-list.md) | `prices`: only the `P` lines, sorted by date, the exact rate under the marks in force, names, the `-c` run |
| [holdings](hledger/holdings.md) | `balance --layout=tidy --real` with `-e`, `--cost`, `--value=end,COMM`, `cur:`: which postings count, the valuation date, cost then value, rounding |

## Index (Beancount 3.2.3)

Reference: Beancount 3.2.3 and beanquery 0.2.0.

| Note | Behaviour |
|---|---|
| [header-and-flags](beancount/header-and-flags.md) | flags, payee and narration, escapes, posting flags |
| [numbers-and-interpolation](beancount/numbers-and-interpolation.md) | number forms, exponents, elided amounts, places, currency groups |
| [prices](beancount/prices.md) | `@`, `@@`, per-unit prices, the weight of a total price |
| [lots-and-booking](beancount/lots-and-booking.md) | `{}` costs, reductions, negative lots, booking methods |
| [inferred-cost-and-price](beancount/inferred-cost-and-price.md) | `{}` and `@ EUR`: the cost or price left to balancing |
| [balancing](beancount/balancing.md) | tolerance, unbalanced transactions, two elided postings |
| [source-lines-and-ordering](beancount/source-lines-and-ordering.md) | reported lines, output order |
| [pad](beancount/pad.md) | the transaction `pad` adds, when it pads, unused pads, sub-accounts, lots |
| [balance-directive](beancount/balance-directive.md) | `balance` assertions: when checked, what is summed, tolerance, the failure message |
| [check-errors](beancount/check-errors.md) | bean-check's other errors: accounts, open/close, currencies, document, tags, names, syntax |
| [check-order](beancount/check-order.md) | the order bean-check prints errors in, so which one is first |
| [check-verdicts](beancount/check-verdicts.md) | bean-check's verdict and first error on every journal of the corpus |
| [syntax](beancount/syntax.md) | every directive and construct: supported, ignored with a warning, or refused |
| [postings](beancount/postings.md) | the flat posting list: row order, lot rows, flags |
| [options-and-directives](beancount/options-and-directives.md) | options and directives that change nothing |
| [refused](beancount/refused.md) | what ledgermill refuses, and what the reference says |

## ledgermill findings

`parse-error` (error): hledger rejects this line too. ledgermill drops the transaction it belongs
to.

`unsupported-syntax` (error): the line uses something ledgermill does not read yet.
`details.construct` names it: `include`, `date-without-year`, `control-character`,
`number-too-large`, `conversion-postings`, `price-annotation` (a `P` line whose rate is followed by a price, a
lot or a label), `period-expression` (a period expression in a `~` rule
that the notes cannot settle), and `timeclock`, `timedot`, `csv` (a file of that kind, by its
extension, on line 1). A transaction with a refused line is not returned. `!include` is a
`parse-error`, as in hledger. [hledger/syntax.md](hledger/syntax.md) is the published list of
every construct and its class.

`ignored-syntax` (warning), for hledger: an automated (`=`) or periodic (`~`) rule, read in full and
left out. `details.construct` is `automated-rule` or `periodic-rule`; one finding per rule, on its
header line. It never changes the verdict ([rules-ignored](hledger/rules-ignored.md)).

`unbalanced-transaction` (error): a group does not sum to zero (`details.group`,
`details.residual`), two postings of a group leave out their amount (`details.reason:
two-elided-amounts`), or a conversion pair carries a price (`conversion-posting-with-cost`).
hledger fails the whole command here; ledgermill reports the finding and still returns the
transaction.

#### Balance assertions and assignments

Built. Backed by the notes [assertion-syntax](hledger/assertion-syntax.md),
[assertions](hledger/assertions.md), [assertion-order](hledger/assertion-order.md),
[assignments](hledger/assignments.md), [assignment-lots-and-costs](hledger/assignment-lots-and-costs.md),
[assignment-processing](hledger/assignment-processing.md) and
[assertion-failures](hledger/assertion-failures.md). hledger fails the whole command on any of
these; like `unbalanced-transaction`, ledgermill reports the finding and still returns the
transactions, with the amounts of the assignments filled in (an empty amount list when the difference is zero).

`assertion-failed` (error), on the **line of the posting that carries the assertion**, with `column` at the first `=`
(a tab counts to the next multiple of eight). `details`:
`account` (after aliases), `commodity` (`""` for a number with no commodity; for `==` the commodity
hledger names: the asserted one if its balance is wrong, else the first other commodity of the
account whose balance is not zero), `expected` (the asserted quantity, a decimal string as
written), `actual` (the running balance in that commodity, an exact decimal string; `"0"` when the
account holds none), `total` (true for `==`, `==*`) and `inclusive` (true for `=*`, `==*`).
One finding per failing assertion, in the order given below (not
file order); the first is the one hledger prints. A failed assertion changes no balance.

`unbalanced-transaction` (existing) also covers a group that does not sum to zero once the assigned amounts
are in, and two amount-less postings (`reason: two-elided-amounts`), on the transaction's header
line, as hledger's `FILE:FIRST-LAST`. An assignment is not an amount-less posting.

`assignment-with-posting-date` (error): an assignment posting with a `date:` tag or a bracketed
date in its comment. `details.account`. hledger names no line (it prints the transaction); the finding is on the
posting's line. `date2:` is fine.

`parse-error` (existing): a malformed assertion ([assertion-syntax](hledger/assertion-syntax.md)), on the posting's line.

Order of findings, which is the order of hledger's errors within a file: parse errors; the problems of
plain transactions (those without an assignment) in file order; then, in date order (ties by file position), the
failing assertions of plain postings and, for a transaction with an assignment taken as a block at its
date, its posting-date error, its failing assertions, and its balancing problems. With several files, a file
is finished before the next, so `check()`'s first error is hledger's first error. A file with a line that was
not read (a `parse-error` or `unsupported-syntax` finding) is not walked: its balances would be wrong, so it has
no `assertion-failed` finding.

Decisions the notes leave open: a price in the asserted amount of an assignment is read, as the note says
(no `unsupported-syntax`); the sub-accounts of an `=*` assignment are summed in name order, so the places of the result
can differ from hledger's when a partial sum is exactly zero ([assignments](hledger/assignments.md), "Not settled").

### Beancount

A `.beancount` or `.bean` file is read on its own (see [refused](beancount/refused.md)).

`unsupported-syntax` (error): `details.construct` is one of

- `include`, `plugin`: refused before the parser runs;
- `option` (`details.name`): any option outside the harmless list (options-and-directives);
- `number-expression` (arithmetic in a number) and `number-too-large` (more than 22 characters);
- `inferred-price` (a price to work out next to a cost, or with zero units), `inferred-cost` (a
  cost to work out next to a reduction with an open cost, or with zero units and nothing to pay),
  `cost-partial` (`{# 5 EUR}`, `{2 # EUR}`), `cost-merge` (`{..., *}`), `number-too-precise`;
- `average-booking` (`details.account`): an account booked with AVERAGE;
- `several-beancount-files` and `mixed-formats`: reported at line 1 of a file when `load` gets
  several Beancount files, or Beancount and hledger files together.

Any of them leaves the file with no transaction at all.

`parse-error` (error): the parser rejects a line (`details.reason` is its message); the
transactions are not returned.

`booking-error` (error): bean-check rejects the transaction while booking it. For two elided
postings bean-query leaves it out and ledgermill does the same; for the other reasons bean-query
keeps it (often with no posting rows, or with an inferred cost that is negative), and ledgermill
leaves it out: the transactions of such a journal are for diagnostics only (the harness does not diff
them). `details.reason` is one of `ambiguous-lot-match`,
`ambiguous-auto-posting`, `not-enough-lots`, `no-matching-lot`, `too-many-missing-numbers`,
`cannot-determine-currency`, `cannot-infer-units`, `cannot-infer-currency`,
`cannot-infer-anything`, `cannot-infer-price`, `cannot-infer-price-currency`,
`several-cost-currencies-match`, `negative-cost`, `negative-price`, `unbalanced`,
`unsupported-booking-method`, `zero-amount-at-cost` (zero units with a cost per unit; on the
posting, and the transaction stays listed) or `auto-posting-with-several-currencies`.

`unbalanced-transaction` (error): a currency group is outside the tolerance
(`details.residual`: a `{commodity, quantity}` per group). bean-check reports it but bean-query
still lists the transaction; ledgermill returns it too.

#### What `check()` reports

The rest of what `bean-check` rejects (check-errors.md, check-order.md, balance-directive.md, pad.md).
`check()` gives the same verdict as `bean-check` and, as its first finding, the one on the same file and
line as the first error `bean-check` prints. That order is not line order: it is the steps of
check-order.md, and the Beancount findings come in that order. One finding per error; an error `bean-check`
prints twice is reported once. `details` names the account or currency, not the reference's message.

All are errors, on the line check-errors.md gives for each:

- `account-not-open`: `details.account`, `details.reason`: `never-opened`, `not-yet-open` or `closed`.
  The line of a transaction's header, or of the directive.
- `duplicate-open`, `duplicate-close`, `close-without-open`: `details.account`.
- `currency-not-allowed`: `details.account`, `details.currency`, `details.directive`: `transaction`
  (including the padding transaction, on the pad's line) or `balance`.
- `assertion-failed`: a `balance` that does not hold: `details.account`, `details.currency`,
  `details.expected` and `details.accumulated` (decimal strings, as written or summed), `details.difference`
  (`accumulated - expected`).
- `duplicate-assertion`: `details.account`, `details.currency`, `details.expected`, `details.first`
  (the first assertion's amount of that day).
- `unused-pad`: `details.account`, `details.source`; on the pad's line.
- `pad-with-cost`: `details.account`, `details.currency`; on the balance's line. The padding is still made.
- `duplicate-commodity`: `details.currency`.
- `invalid-account-name`: `details.account`.
- `unbalanced-push`: `details.kind` (`tag` or `metadata`), `details.name`, `details.reason`
  (`pop-without-push`, or `push-not-popped` on line 0).
- `duplicate-metadata`: `details.key`, `details.scope` (`entry` or `posting`), on the transaction's header line.
- `parse-error` (existing) also covers tokens, syntax errors, impossible dates, and a lone `*` or `#` or a
  tag at the margin; `booking-error` and `unbalanced-transaction` stay as they are, except that a
  `booking-error` sits on the line of the posting the reference blames for what it cannot categorize or work
  out, and on the transaction's header for lot matching.

Warning: `ignored-syntax` (`details.construct`: `document`). The directive is read and skipped:
ledgermill does not look at files, so it cannot give the `File does not exist` error `bean-check` gives.
syntax.md lists what is supported, ignored with this warning, or refused.

Padding transactions are in `transactions()` and `postings()` (pad.md): flag `P`, on the pad's line, their
postings on the balance's line. `transactions()` on a journal with errors is for diagnostics only. It still lists
what bean-query lists for the errors of the checks (accounts, currencies, duplicates, assertions, unused pads,
unbalanced transactions); a transaction rejected by booking is left out, as there. A file with a parse-level error
(syntax, a bad account root, a tag or metadata mistake) has no transaction at all, although bean-query lists some of
them: the parser we build on gives up on the whole file. Any `unsupported-syntax` leaves the list empty.
