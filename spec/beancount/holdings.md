# Holdings: the sums of the units and holdings queries

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-23.

Status: to build (`Ledger::holdings()` for Beancount).

## Rule

Plainledger runs two queries, each over every posting (or over those dated up to `D`, below), grouped by
account and by the posting's own commodity, with every column printed by `str()`:

```text
units:    SELECT account, currency, str(sum(units(position))) AS units [WHERE date <= D] GROUP BY account, currency
holdings: SELECT account, currency, str(sum(units(position))) AS units, str(sum(cost(position))) AS cost_basis,
          str(sum(convert(cost(position), "V"))) AS cost_basis_value, str(sum(convert(position, "V"))) AS market_value
          [WHERE date <= D] GROUP BY account, currency
```

The `units` column of both is the same. What each column sums, per group:

- **A cell is a set of amounts, one per commodity:** `(3 EUR)`, `(35 USD,20 EUR,3 CHF,2 MMM)`, and `()` when
  it is empty. The order of the amounts in a cell is not settled: the journals tried give orders that are
  neither insertion, nor alphabetical, nor by size. Plainledger looks the commodity up in the cell. ledgermill
  lists them in code point order of commodity and the harness compares them as sets.
- **`units`:** the units of the group's postings, added one after the other in the order of the postings
  (the order of `transactions()`: by date and line, the postings of a transaction as booked, a padding
  transaction at its `pad`). The sum is a number and a commodity; it is **dropped as soon as it is exactly
  zero**, so what follows starts again with its own places (`1.50`, `-1.50`, `3` gives `3`, not `3.00`), and a
  zero added to nothing is nothing, while a zero added to a sum that is held gives the finer places
  (`5` and `0.00` give `5.00`). Each addition is rounded to 28 digits. A group whose sum ends at zero is still a
  row, with `()`.
- **`cost_basis`:** the same sums of `cost(position)` (cost-of-position.md), one amount per commodity, so lots in
  different cost currencies give several amounts, and a posting with no cost adds its units under their own
  commodity (the last row below: `2 MMM`). Zero is dropped the same way.
- **`cost_basis_value`:** `cost(position)` of each posting converted to `V` (convert-positions.md: the amount
  in `V` stays, a rate converts, no rate leaves it), and then summed by commodity as above. The conversions come
  first and are rounded one by one; the sum is of the rounded products.
- **`market_value`:** `convert(position, V)` of each posting, summed the same way, with the bridge of
  convert-positions.md and the latest prices there are (convert-rates.md: no date).
- **`str()` prints plain digits, never an exponent:** `1E-7` is `0.0000001` and `1.0E+2` is `100`, where
  `number()` of the value query prints `1E-7` and `1.0E+2` (convert-rates.md).
- **`WHERE date <= D`** keeps the postings of transactions dated on or before `D`: the day itself counts, a
  date literal is written `2026-01-03`. It changes nothing else: the prices are the latest of the whole file.
  A padding transaction is dated on its `pad`. Every transaction counts, whatever its flag (`Assets:Pending` has a `!` transaction). A group with no
  posting up to `D` has no row, so a `D` before every transaction gives no rows at all.
- The rows come in no order the notes rely on.

### What ledgermill gives

`Holding` is the same type as for hledger. A Beancount `holdings()` has the account, the commodity, the units
as a plain decimal string (`"0"` for an empty cell), and `cost`, `value`, `cost_value` as the amounts of the
cells (`str()`'s digits, no rounding to ten places: the 28 digits above), in code point order of commodity.
The group is listed when its units are not zero or one of the three lists is not empty. Sorted by the account's
`:`-separated names and then the commodity, as for hledger.

`end` is exclusive, as for hledger: the postings dated before `end`, which is `WHERE date <= D` with `D` the
day before `end`. The value date is **not** `end - 1` as it is for hledger: Plainledger's query gives no date
to `convert()`, so the prices are the latest of the ledger, today's date and `end` never looked at, and
`HoldingsQuery.today` is not used.

## Probe

`probes/holdings-sums.beancount`

Command:

```sh
spec/beancount/show-holdings.sh spec/beancount/probes/holdings-sums.beancount EUR - '^Assets:'
spec/beancount/show-holdings.sh spec/beancount/probes/holdings-sums.beancount EUR 2026-01-03 '^Assets:'
spec/beancount/show-holdings.sh spec/beancount/probes/holdings-sums.beancount EUR 2026-01-01 '^Assets:'
```

`show-holdings.sh` runs the two queries above (`-` is no date; the last argument keeps the accounts that
match). `bean-check` has no output (exit 0). The journal has a pad (`Assets:Padded`), an account booked with
`NONE` (`Assets:Short`: ten units bought at 2 EUR and a sale at 3 EUR, which is another lot), and lots in
three cost currencies (`Assets:Mixed`).

## Output

```text
== holdings-sums.beancount V=EUR accounts ^Assets:
-- units: account,currency,units
Assets:Padded,EUR,(100 EUR)
Assets:Reset,EUR,(3 EUR)
Assets:Finer,EUR,(3.75 EUR)
Assets:Zeros,EUR,(5.00 EUR)
Assets:Zeros,USD,(7 USD)
Assets:Short,XXX,()
Assets:Mixed,MMM,(21 MMM)
Assets:Pending,EUR,(4 EUR)
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
Assets:Padded,EUR,(100 EUR),(100 EUR),(100 EUR),(100 EUR)
Assets:Reset,EUR,(3 EUR),(3 EUR),(3 EUR),(3 EUR)
Assets:Finer,EUR,(3.75 EUR),(3.75 EUR),(3.75 EUR),(3.75 EUR)
Assets:Zeros,EUR,(5.00 EUR),(5.00 EUR),(5.00 EUR),(5.00 EUR)
Assets:Zeros,USD,(7 USD),(7 USD),(14 EUR),(14 EUR)
Assets:Short,XXX,(),(-10 EUR),(-10 EUR),()
Assets:Mixed,MMM,(21 MMM),"(35 USD,20 EUR,3 CHF,2 MMM)","(102 EUR,2 MMM)","(154 EUR,10 MMM)"
Assets:Pending,EUR,(4 EUR),(4 EUR),(4 EUR),(4 EUR)
== holdings-sums.beancount V=EUR date <= 2026-01-03 accounts ^Assets:
-- units: account,currency,units
Assets:Padded,EUR,(100 EUR)
Assets:Reset,EUR,()
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
Assets:Padded,EUR,(100 EUR),(100 EUR),(100 EUR),(100 EUR)
Assets:Reset,EUR,(),(),(),()
== holdings-sums.beancount V=EUR date <= 2026-01-01 accounts ^Assets:
-- units: account,currency,units
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
```

Reading it: `Reset` (1.50, -1.50, 3) is `3`; `Finer` (1.5, 2.25) is `3.75`; `Zeros` is `5.00` in EUR (a zero
added to a held 5) and `7` in USD (a zero before it counted for nothing); `Short` has no units left and a cost
basis of -10 EUR (20 - 30), and no market value, its units cancelling; `Mixed` has a cost basis in four
commodities (35 USD is 30 + 5), 102 EUR once the three that have a price in EUR are converted (70 + 20 + 12), and
a market value of 154 EUR: its 11 units bought with USD go through USD at 7 USD and 2 EUR; the other 10 have no
rate. On 2026-01-03 `Reset` holds nothing, and `Padded` already has its padding (dated on the `pad`, 2026-01-02).
