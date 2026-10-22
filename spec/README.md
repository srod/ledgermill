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
- `hledger/probes/` – the probe journals.
- `hledger/prototype-observations.md` – the notes of our earlier TypeScript prototype, kept as a
  starting point. They are our own text, written from running hledger.

- `beancount/` – notes on Beancount. The commands use `beancount/show.sh`, which prints what
  `bean-check` says and, from `bean-query -f csv`, one row per transaction and one per posting
  (the two queries the harness uses). Postings are `id,account,posting_flag,number,currency,
  cost_number,cost_currency,cost_date,cost_label,price_number,price_currency,filename,lineno`;
  `id` is shortened to 6 characters and shared by the postings of a transaction. A price is the
  per-unit price, a cost the lot's cost per unit.
- `beancount/probes/` – the probe journals; `beancount/probes/refused/` those ledgermill refuses (not run by the harness);
  `beancount/probes/pending/` the ones it gets wrong and is to fix (not run either). `beancount/verdicts.sh` prints
  bean-check's verdict and first error for every journal of the corpus.

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
| [postings](hledger/postings.md) | posting status, virtual and balanced virtual postings, account names |
| [prices-and-lots](hledger/prices-and-lots.md) | `@`, `@@`, `{}`, `{{}}`, `[date]`, `(label)` |
| [balancing](hledger/balancing.md) | groups, what must sum to zero, tolerance |
| [elided-amounts](hledger/elided-amounts.md) | what an amount-less posting gets, and its places |
| [inferred-prices](hledger/inferred-prices.md) | the price given when two commodities do not balance |
| [conversion](hledger/conversion.md) | conversion accounts, `type:` tags, pairs that balance by quantity |
| [ordering](hledger/ordering.md) | output order, several files |

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
`details.construct` names it: `directive` (periodic and automated rules), `include`,
`balance-assertion`, `date-without-year`, `control-character`, `number-too-large`,
`conversion-postings`. A transaction with a refused line is not returned.

`unbalanced-transaction` (error): a group does not sum to zero (`details.group`,
`details.residual`), two postings of a group leave out their amount (`details.reason:
two-elided-amounts`), or a conversion pair carries a price (`conversion-posting-with-cost`).
hledger fails the whole command here; ledgermill reports the finding and still returns the
transaction.

### Beancount

A `.beancount` or `.bean` file is read on its own (see [refused](beancount/refused.md)).

`unsupported-syntax` (error): `details.construct` is one of

- `include`, `plugin`: refused before the parser runs;
- `option` (`details.name`): any option outside the harmless list (options-and-directives);
- `number-expression` (arithmetic in a number) and `number-too-large` (more than 22 characters);
- `pad` and `balance`: directives not built yet (pad.md, balance-directive.md say what to build);
- `inferred-price` (a price to work out next to a cost, or with zero units), `inferred-cost` (a
  cost to work out next to a reduction with an open cost, or with zero units and nothing to pay),
  `cost-partial` (`{# 5 EUR}`, `{2 # EUR}`), `cost-merge` (`{..., *}`), `number-too-precise`;
- `average-booking` (`details.account`): an account booked with AVERAGE;
- `several-beancount-files` and `mixed-formats`: reported at line 1 of a file when `load` gets
  several Beancount files, or Beancount and hledger files together.

Any of them leaves the file with no transaction at all.

`parse-error` (error): the parser rejects a line (`details.reason` is its message); the
transactions are not returned.

`booking-error` (error): bean-check rejects the transaction while booking it, and bean-query
leaves it out; ledgermill does the same. `details.reason` is one of `ambiguous-lot-match`,
`ambiguous-auto-posting`, `not-enough-lots`, `no-matching-lot`, `too-many-missing-numbers`,
`cannot-determine-currency`, `cannot-infer-units`, `cannot-infer-currency`,
`cannot-infer-anything`, `cannot-infer-price`, `cannot-infer-price-currency`,
`several-cost-currencies-match`, `negative-cost`, `negative-price`, `unbalanced`,
`unsupported-booking-method` or `auto-posting-with-several-currencies`.

`unbalanced-transaction` (error): a currency group is outside the tolerance
(`details.residual`: a `{commodity, quantity}` per group). bean-check reports it but bean-query
still lists the transaction; ledgermill returns it too.

#### Proposed, not built yet: what `check()` should report

The rest of what `bean-check` rejects (check-errors.md, check-order.md, balance-directive.md, pad.md).
`check()` should give the same verdict as `bean-check` and, as its first finding, the one on the same
file and line as the first error `bean-check` prints. That order is not line order: it is the steps of
check-order.md, and findings are ordered that way (today they are ordered by line, which would need to
change). One finding per error is enough; an error `bean-check` prints twice need not be repeated.
`details` names the account or currency, not the reference's message.

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
- `parse-error` (existing) also covers tokens, syntax errors and impossible dates; `booking-error` and
  `unbalanced-transaction` stay as they are.

Warning: `ignored-syntax` (`details.construct`: `document`). The directive is read and skipped:
ledgermill does not look at files, so it cannot give the `File does not exist` error `bean-check` gives.
syntax.md lists what is supported, ignored with this warning, or refused.
