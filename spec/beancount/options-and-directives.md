# Options and directives

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

Status: **built**. as described; a repeated option is now accepted too.

## Rule

These change no transaction and no posting, and ledgermill reads past them:

- the options `title`, `operating_currency`, `render_commas`, `documents`, `long_string_maxlines`,
  `conversion_currency` and the `account_previous_*`, `account_current_*`, `account_unrealized_gains`
  and `account_rounding` names. `booking_method` is also accepted, and it is used
  (lots-and-booking.md);
- the directives `open` (its booking method is used, and the currencies it lists, check-errors.md), `close`,
  `commodity`, `event`, `note`, `custom`, `price`, and `query` (they change no transaction and no posting,
  though `check()` reports some of their mistakes, check-errors.md);
- `pushtag`/`poptag` and `pushmeta`/`popmeta`: a transaction inside them has the same rows as one
  outside.

Every other option is refused (refused.md), as are `include` and `plugin`. `pad` and `balance` are
read (pad.md, balance-directive.md), and `document` is skipped with a warning (check-errors.md).

A harmless option may be given more than once, and the **last** one counts: two `booking_method` lines with
FIFO then LIFO book by LIFO (`option-repeated-last-wins`, which also repeats `operating_currency` and
`render_commas`; `option-title-twice` repeats `title`). ledgermill drops the earlier lines, since the parser
calls a repeat an error. The whole list of constructs is in syntax.md.

## Probe

`probes/options-harmless.beancount`, `probes/directives-ignored.beancount`, `probes/option-repeated-last-wins.beancount`,
`probes/option-title-twice.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/options-harmless.beancount
spec/beancount/show.sh spec/beancount/probes/directives-ignored.beancount
spec/beancount/show.sh spec/beancount/probes/option-repeated-last-wins.beancount
spec/beancount/show.sh spec/beancount/probes/option-title-twice.beancount
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

options-harmless:

```text
bean-check: (no output, exit 0)
345066,transaction,23,2026-01-02,*,,options and other directives change nothing here
345066,Assets:A,,1.50,EUR,,,,,,,24
345066,Assets:B,,-1.50,EUR,,,,,,,25
```

directives-ignored:

```text
bean-check: (no output, exit 0)
19fce7,transaction,6,2026-01-02,*,,inside pushtag and pushmeta
1defc2,transaction,11,2026-01-03,*,,outside
19fce7,Assets:A,,1,EUR,,,,,,,7
19fce7,Assets:B,,-1,EUR,,,,,,,8
1defc2,Assets:A,,1,EUR,,,,,,,12
1defc2,Assets:B,,-1,EUR,,,,,,,13
```

option-repeated-last-wins (the sale takes the 20 EUR lot, the newest, as LIFO says):

```text
bean-check: (no output, exit 0)
f83e26,transaction,12,2026-01-02,*,,buy two lots
c6afbe,transaction,17,2026-01-10,*,,sell with the last booking_method given: the newest lot goes
f83e26,Assets:Stock,,1,BTC,10,EUR,2026-01-02,,,,13
f83e26,Assets:Stock,,1,BTC,20,EUR,2026-01-03,,,,14
f83e26,Assets:Bank,,-30,EUR,,,,,,,15
c6afbe,Assets:Stock,,-1,BTC,20,EUR,2026-01-03,,,,18
c6afbe,Assets:Bank,,20,EUR,,,,,,,19
```

option-title-twice:

```text
bean-check: (no output, exit 0)
98fef9,transaction,7,2026-01-02,*,,a repeated title option
98fef9,Assets:A,,1,EUR,,,,,,,8
98fef9,Assets:B,,-1,EUR,,,,,,,9
```
