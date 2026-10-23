# Syntax: what ledgermill reads, ignores, refuses

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-22.

Status: **built**. ledgermill does what each row says.

Every directive and construct of a Beancount file, with what ledgermill does and the probe that shows
what the reference does.

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
| `price` | supported (listed, and the rates of `convert()`) | `options-harmless`, `price-table`, `convert-rate-dates` |
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
| comment lines starting with `*` or `#` at the margin | supported | `org-heading-lines`, `hash-line`, `margin-lines`, `margin-lines-errors` |
| `pushtag`/`poptag`, `pushmeta`/`popmeta` | supported | `directives-ignored`, `check-tags-and-meta` |
| `option` in the harmless list (options-and-directives.md), repeated or not | supported | `options-harmless`, `option-title-twice`, `option-repeated-last-wins` |
| any other `option` | unsupported | `option-tolerance`, `option-unknown` |
| `plugin` | unsupported | `plugin` |
| `include` | unsupported | `include` |
| arithmetic in a number, in a transaction or a `price` | unsupported | `number-expression`, `price-expression` |
| partial costs `{2 # EUR}`, `{# 1 EUR}` | unsupported | `cost-partial-per-unit`, `cost-partial-total` |
| cost merge `{..., *}` | unsupported | `cost-merge` |
| booking method AVERAGE | unsupported | `average-booking` |
| price left to work out next to a cost | unsupported | `inferred-price-with-cost` |
| more than one Beancount file, or Beancount with hledger files | unsupported | (no probe: the harness runs one file; see refused.md) |

The probes are under `probes/` or `probes/refused/`.

## Probe

The probes named in the table, all existing. Commands and outputs are in the note of each behaviour
(refused.md, options-and-directives.md, pad.md, balance-directive.md, check-errors.md and the others
of the index); `date-slashes`, `comment-lines` and the margin-line probes have no other note, so:

```sh
spec/beancount/show.sh spec/beancount/probes/date-slashes.beancount
spec/beancount/show.sh spec/beancount/probes/comment-lines.beancount
spec/beancount/show.sh spec/beancount/probes/org-heading-lines.beancount
spec/beancount/show.sh spec/beancount/probes/hash-line.beancount
spec/beancount/show.sh spec/beancount/probes/margin-lines.beancount
spec/beancount/show.sh spec/beancount/probes/margin-lines-errors.beancount
```

**Margin lines.** A line that starts in column 0 with `*` followed by any character (an org-mode heading, `*no-space`,
`** deeper`, `*;x`), or with `#` followed by anything that cannot start a tag name (`# x`, `#! x`, `##  x`), is a
comment and is skipped. A `*` or `#` alone on a line, and `#` followed by a tag character (`#tag`, `#_a`), is a
syntax error instead (`unexpected ASTERISK`, `unexpected HASH`, `unexpected TAG`). Inside a string that spans
lines, a line starting with `*` or `#` is part of the string (`margin-lines`). A `;` comment may follow either.

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

org-heading-lines:

```text
bean-check: (no output, exit 0)
4ba0d6,transaction,7,2026-01-02,*,,after org-mode headings
4ba0d6,Assets:A,,1,EUR,,,,,,,8
4ba0d6,Assets:B,,-1,EUR,,,,,,,9
```

hash-line:

```text
bean-check: (no output, exit 0)
ff7d1e,transaction,6,2026-01-02,*,,x
ff7d1e,Assets:A,,1,EUR,,,,,,,7
ff7d1e,Assets:B,,-1,EUR,,,,,,,8
```

margin-lines (the narration keeps its two inner lines):

```text
bean-check: (no output, exit 0)
eb9172,transaction,12,2026-01-02,*,,"a string that spans lines
* does not start a comment inside it
# nor does this one"
eb9172,Assets:A,,1,EUR,,,,,,,15
eb9172,Assets:B,,-1,EUR,,,,,,,16
```

margin-lines-errors (the first error is on line 3; the reference then goes on with `Invalid token` for each word):

```text
bean-check: margin-lines-errors.beancount:3: syntax error, unexpected TAG
bean-check: margin-lines-errors.beancount:3: Invalid token: 'at'
```
