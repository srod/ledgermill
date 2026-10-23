# Price directives (the `prices` table)

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-23.

Status: to build (`Ledger::prices()` for Beancount).

## Rule

The query Plainledger runs is `SELECT date, currency, number(amount), currency(amount) FROM prices`.
The table lists the `price` directives of the file and nothing else:

- A price annotation on a posting (`@`, `@@`) and a cost (`{}`) put no row in it, and no rate anywhere
  else (convert-positions.md checks that valuation ignores them too).
- **Order: by date, not file order.** The sort is stable: directives with the same date stay in the
  order written, and a directive written twice is listed twice (no duplicates are dropped, no
  last-one-wins). A directive written below a later date comes first.
- The number is the number as parsed, printed as numbers-and-interpolation.md says: the places as written
  (`1.50`, `1.000`), no digit-group commas (`1,234.50` is `1234.50`), no `+` and no trailing dot, `-0` is
  `0`, and a number smaller than 0.000001 has an exponent (`1E-7`). Twenty digits stay whole. A price of zero,
  a negative one, and one of a commodity in itself are listed like any other.
- A price written as an expression is listed with its value (`(1 + 2)` is `3`). ledgermill refuses
  arithmetic in numbers (refused.md), here too: `unsupported-syntax`, `number-expression`.
- Metadata under a directive, or a tag, changes nothing. The entry's line (`entries`, `lineno`) is the
  line of the directive's first line, the same order as the table.

ledgermill lists the same fields as it does for hledger (`PriceDirective`): the date, the priced commodity,
the rate as `number(amount)` prints it, with the commodity of the amount, and the file and line.

## Probe

`probes/price-table.beancount`, `probes/price-table-expression.beancount`

Command:

```sh
spec/beancount/show-prices.sh spec/beancount/probes/price-table.beancount
spec/beancount/show-prices.sh spec/beancount/probes/price-table-expression.beancount
```

`show-prices.sh` runs the prices query and then a second query on the entries to get the line of each
price (`SELECT date, lineno, filename FROM entries WHERE type = 'price'`, same order). `bean-check:` has
no output (exit 0) for both journals.

## Output

```text
== price-table.beancount
-- bean-check
-- prices: date,currency,number(amount),currency(amount)
2026-01-05,BBB,2,EUR
2026-02-01,CCC,1E-7,EUR
2026-02-01,DDD,1234.50,EUR
2026-02-01,EEE,3,EUR
2026-02-01,FFF,0,EUR
2026-02-01,GGG,-2,EUR
2026-02-01,EUR,1,EUR
2026-02-01,HHH,1.123456789012345678901234,EUR
2026-02-01,JJJ,0,EUR
2026-02-01,KKK,1.000,EUR
2026-02-01,LLL,12345678901234567890,EUR
2026-03-01,AAA,1.50,EUR
2026-03-01,AAA,1.60,EUR
2026-03-01,AAA,1.60,EUR
-- price entries: date,lineno,filename
2026-01-05,8,price-table.beancount
2026-02-01,11,price-table.beancount
2026-02-01,12,price-table.beancount
2026-02-01,13,price-table.beancount
2026-02-01,14,price-table.beancount
2026-02-01,15,price-table.beancount
2026-02-01,16,price-table.beancount
2026-02-01,17,price-table.beancount
2026-02-01,18,price-table.beancount
2026-02-01,19,price-table.beancount
2026-02-01,20,price-table.beancount
2026-03-01,7,price-table.beancount
2026-03-01,9,price-table.beancount
2026-03-01,10,price-table.beancount
== price-table-expression.beancount
-- bean-check
-- prices: date,currency,number(amount),currency(amount)
2026-01-02,AAA,3,EUR
2026-01-02,BBB,6,EUR
-- price entries: date,lineno,filename
2026-01-02,3,price-table-expression.beancount
2026-01-02,4,price-table-expression.beancount
```
