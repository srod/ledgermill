# Syntax: what ledgermill reads, ignores, refuses

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Every directive and construct of a Beancount file, with what ledgermill does and the probe that shows
what the reference does. This is the target, not the state of the code: "today" says where the code is
behind it.

## Rule

Three classes.

- **supported**: read, and the result is what the reference gives, errors included (check-errors.md,
  check-order.md). A directive that changes nothing (`price`, `event`, ...) is supported: there is
  nothing to ignore, and no finding.
- **ignored (warning)**: skipped on purpose with an `ignored-syntax` warning (`details.construct`
  names it), because ledgermill cannot reproduce what the reference does with it. The file is still
  read in full.
- **unsupported (error)**: `unsupported-syntax` (`details.construct`), the file gives no transaction
  (refused.md). This is for what could change which transactions or postings come out and is not
  reproduced.

| Construct | Class | Probe |
|---|---|---|
| `open` (currencies, booking method) | supported | `check-currency-constraint`, `booking-option` |
| `close` | supported | `check-close-errors`, `directives-ignored` |
| `commodity` | supported | `check-commodity-duplicate`, `options-harmless` |
| `price` | supported | `options-harmless` |
| `event`, `query`, `custom` | supported | `options-harmless`, `directives-ignored`, `check-accepted` |
| `note` | supported | `options-harmless`, `check-directive-accounts` |
| `balance` | supported | `balance-directive` and the `balance-directive-*` probes |
| `pad` | supported | `pad` and the `pad-*` probes |
| `document` | ignored (warning) | `check-document-existing`, `check-document-missing` |
| transaction: `*`, `!`, `txn`, other flag characters | supported | `header-flags` |
| payee and narration, escapes, tags, links | supported | `header-strings` |
| metadata on transactions and postings | supported | `source-lines`, `check-tags-and-meta` |
| posting flags | supported | `posting-flags` |
| elided amounts, several currencies | supported | `interpolation`, `interpolation-groups` |
| costs `{}`, `{{ }}`, with date and label | supported | `lots-costs`, `lots-inferred-cost-double-braces`, `lots-inferred-cost-date-and-label` |
| inferred cost or price (`{}`, `@ EUR`) | supported | `lots-inferred-cost`, `prices-inferred-currency` |
| prices `@`, `@@` | supported | `prices` |
| booking methods STRICT, STRICT_WITH_SIZE, NONE, FIFO, LIFO, HIFO | supported | `booking-strict-single-lot`, `booking-strict-with-size`, `booking-none`, `booking-fifo`, `booking-lifo`, `booking-hifo` |
| number forms (commas, `+`, trailing dot, small values) | supported | `number-forms` |
| dates written `YYYY-MM-DD` or `YYYY/MM/DD` | supported | `date-slashes` |
| comment lines starting with `;`, trailing comments | supported | `comment-lines` |
| comment lines starting with `*` or `#` at the margin | supported (today: parse error) | `probes/pending/org-heading-lines`, `probes/pending/hash-line` |
| `pushtag`/`poptag`, `pushmeta`/`popmeta` | supported | `directives-ignored`, `check-tags-and-meta` |
| `option` in the harmless list (options-and-directives.md), repeated or not | supported (today: a repeated `title` is a parse error) | `options-harmless`, `probes/pending/option-title-twice` |
| any other `option` | unsupported | `option-tolerance`, `option-unknown` |
| `plugin` | unsupported | `plugin` |
| `include` | unsupported | `include` |
| arithmetic in a number | unsupported | `number-expression` |
| partial costs `{2 # EUR}`, `{# 1 EUR}` | unsupported | `cost-partial-per-unit`, `cost-partial-total` |
| cost merge `{..., *}` | unsupported | `cost-merge` |
| booking method AVERAGE | unsupported | `average-booking` |
| price left to work out next to a cost | unsupported | `inferred-price-with-cost` |
| more than one Beancount file, or Beancount with hledger files | unsupported | (no probe: the harness runs one file; see refused.md) |

The probes are under `probes/`, `probes/refused/` or `probes/pending/`.

## Probe

The probes named in the table, all existing. Commands and outputs are in the note of each behaviour
(refused.md, options-and-directives.md, pad.md, balance-directive.md, check-errors.md and the others
of the index); `date-slashes` and `comment-lines` have no other note, so:

```sh
spec/beancount/show.sh spec/beancount/probes/date-slashes.beancount
spec/beancount/show.sh spec/beancount/probes/comment-lines.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`.

## Output

date-slashes:

```text
bean-check: (no output, exit 0)
cd0dce,transaction,3,2026-01-02,*,,x
cd0dce,Assets:A,,1,EUR,,,,,,,4
cd0dce,Assets:B,,-1,EUR,,,,,,,5
```

comment-lines:

```text
bean-check: (no output, exit 0)
294382,transaction,6,2026-01-02,*,,comments around a transaction
294382,Assets:Bank,,1,EUR,,,,,,,8
294382,Equity:Opening,,-1,EUR,,,,,,,9
```

