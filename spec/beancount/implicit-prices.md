# `plugin "beancount.plugins.implicit_prices"`

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-11-09.

Status: **built**. With the plugin on, the price annotations (`@`, `@@`) and the costs of the postings add `price`
entries. That changes `prices()`, and so `value()` and `holdings()`, which take their rates from the prices table
(convert-rates.md). Nothing else changes: `transactions()`, `postings()` and `check()` are those of the journal
without the plugin line (the plugin adds no error).

## Rule

### Where a price comes from

For each posting of a transaction that booking kept (a transaction booking rejects is not seen by the plugin,
the same rule as auto-accounts.md), in the order of the booked postings (postings.md: grouped by the currency
they balance in, not the order written):

- **A price annotation** gives an entry for the posting's commodity: the price per unit and the price's currency,
  as booked (prices.md, inferred-cost-and-price.md). `@ 100.00 EUR` is `100.00`; `4 BTC @@ 10 EUR` is `2.5`;
  `3 BTC @@ 10.00 EUR` is `3.333333333333333333333333333` (28 digits); `0.5 BTC @@ 100 EUR` is `2.0E+2`,
  with the exponent the booked price has; negative units give the same positive price; a price left to work out
  (`@ EUR`, `@@ EUR`) is worked out first; a price of zero is an entry of zero; `0 BTC @ 5 EUR` gives `5`.
  A price written with a minus sign is an error of the reference (`Negative prices are not allowed`) and
  its entry has the price without the sign. A price in the commodity of the units (`1 BTC @ 2 BTC`) is an entry.
- **A cost** gives an entry for the posting's commodity at the cost per unit, in the cost's currency, when the
  posting has **no price annotation** and **opens a lot**. The cost is the one booked: `{{20 EUR}}` on 4 BTC is
  `5`, `{10 # 4 EUR}` on 2 BTC is `12`, `{}` is the inferred cost. A cost with a date or a label gives the entry
  on the transaction's date, not the lot's. A cost of zero is an entry of zero, and so are zero units at a cost
  (which are an error of the reference, `Amount is zero`, with the transaction kept).
- **A price and a cost on one posting:** the price annotation is the entry and the cost makes none
  (`2 BTC {10 EUR} @ 12.50 EUR` is `12.50`; `{{20 EUR}} @@ 50 EUR` on 2 BTC is `25`). The reference rejects the two in different currencies
  (`Cost and price currencies must match`); ledgermill does not report that error (a gap that does not depend on
  the plugin), so that case has no probe.
- **A posting that reduces a lot makes no price from its cost**: `-1 BTC {}`, `-1 BTC {10 EUR}`, `-1 BTC
  {2026-01-02}`, whatever the booking method (FIFO, LIFO, HIFO, STRICT), also when a sale takes several lots, and also the
  same day as the purchase. With a price annotation it makes the price entry like any posting
  (`-1 BTC {10 EUR} @ 14 EUR` is `14`; a sale across two lots with one `@` price is one entry). Reducing is
  decided as booking does: a posting adds a lot when the account holds none of the commodity with the opposite sign
  (a short sale with a cost, `-2 BTC {10 EUR}` with nothing held, adds a negative lot and makes a price; so does a
  posting that comes after the lot was sold out), and **with `NONE` booking every posting with a cost adds a lot**
  (`-3 BTC {}` and `-1 BTC {11 EUR}` against a held BTC both make a price).
- **A posting with neither** makes nothing.

The entry is dated the transaction's date, and its `lineno` and file (`entries.lineno`) are the transaction's
line and file, not the posting's.

### Which entries are dropped

One run of the plugin drops an entry equal to one it made before: same date, same commodity, same quote currency
and the same **value** of the rate (`100.0` and `100.00` are equal), whether it came from the same transaction
(two postings, one transaction), from two transactions the same day, or from a rate seen again after another rate. The first one stays,
with its places as written. The commodity, the quote currency and the date all tell entries apart (`100 EUR` and `100 USD`, `BTC` and `ETH`,
two days). An entry is **never** compared with a `price` directive: a directive written for the same day and the same rate is listed
beside it, twice in the table. Which rate wins is the usual rule (convert-rates.md): the one listed last for
the day.

**Each `plugin` line runs the plugin once more**, unlike `auto_accounts`: a journal naming it three times has every entry three
times, the whole list of the first run, then the whole list of the second (`1 AAA @ 5`, `1 BBB @ 6`, `1 AAA @ 7`
are listed `5 6 7 5 6 7`), all on the line of their transaction. A `plugin` line may stand anywhere in the file, with a tab or a comment as
`auto_accounts` allows. With a configuration string the reference fails to apply the plugin (`<load>:0: Error applying plugin`, no price
at all) and ledgermill refuses it by name, as for `auto_accounts` (refused.md). The two plugins together, in either order, do
what each does alone. The plugin belongs to its file (several-files.md): in a manifest of several Beancount files, the prices of one file's
postings are not the other's.

### Where the entries are in the table

The prices table lists the entries sorted by date, and the same date in the order of their line (price-table.md).
The entries of the plugin have the line of their transaction, so on one day they stand between the directives
written above and below the transaction: a directive written above a transaction is listed before its entry,
one written below it after. Entries of one transaction (one line) stay in the order of its postings, and
the entries of a second run come after those of the first. `convert()` takes the **latest entry on or before the
date**; the last one listed on a day wins, so a posting written below a directive of the same day beats it, and
a directive written below the posting beats the posting. The rest of convert-rates.md
(inverses, a pair priced both ways, no chain) applies to these entries as to the others.

### Errors

None are added. `check()` gives for a journal with the plugin what it gives without (compared on every probe
of the corpus, 196 journals with the plugin line added at the end, and the app's journals).

## Probe

Accepted:

- `probes/implicit-prices.beancount`: `@`, `@@`, a cost, a total cost.
- `probes/implicit-prices-cost-and-price.beancount`: a price and a cost on one posting, a date and a label, `#`.
- `probes/implicit-prices-reductions.beancount`: purchases and sales at cost, with and without a price, a sale with nothing left (a negative lot).
- `probes/implicit-prices-reduction-lots.beancount`: a sale across two lots (FIFO) with and without a price.
- `probes/implicit-prices-booking-methods.beancount`: sales under FIFO, LIFO, HIFO and NONE booking; only NONE makes a price from the cost of a sale.
- `probes/implicit-prices-inferred.beancount`: costs and prices left to work out, an inexact one.
- `probes/implicit-prices-negative-and-zero.beancount`: negative units, zero units, a price of zero, a price in the units' commodity.
- `probes/implicit-prices-numbers.beancount`: expressions, an inexact total price, a very small and a large rate, places.
- `probes/implicit-prices-cost-places.beancount`: the places of a cost per unit, a total cost on fractional units, a cost of zero.
- `probes/implicit-prices-duplicates.beancount`, `probes/implicit-prices-dedupe.beancount`: the entries dropped, and `price` directives beside them.
- `probes/implicit-prices-twice.beancount`, `probes/implicit-prices-order.beancount`: several plugin lines, the order of the entries.
- `probes/implicit-prices-rejected.beancount`: a transaction that does not balance, with an unknown account, with a closed account (kept), two postings to work out (rejected).
- `probes/implicit-prices-convert.beancount`, `probes/implicit-prices-both-ways.beancount`: valuation with implicit entries, directives on the same day, a pair priced both ways.
- `probes/implicit-prices-auto-accounts.beancount`, `probes/implicit-prices-auto-accounts-reversed.beancount`: with `auto_accounts`, both orders.
- `probes/several-files/implicit-a.beancount`, `implicit-b.beancount`: one file with the plugin, one without.

Rejected (the harness compares the first error only): `probes/implicit-prices-negative-price.beancount`, `probes/implicit-prices-zero-units-at-cost.beancount`.

Refused (`probes/refused/`, not run by the harness): `plugin-implicit-prices-config.beancount` (a configuration string), and
`implicit-prices-zero-units-total.beancount` (`0 BTC @@ 10 EUR`: the reference lists a price of `0`, ledgermill refuses the division
by zero as `number-too-precise`, a gap of booking that does not depend on the plugin).

Command (a Python traceback replaced by a marker, like `show-opens.sh`):

```sh
spec/beancount/show-implicit.sh spec/beancount/probes/implicit-prices*.beancount
spec/beancount/show-value.sh spec/beancount/probes/implicit-prices-convert.beancount EUR 2026 1 4 '^Assets:'
```

`show-implicit.sh` prints what `bean-check` says, the prices table and the date, line and file of each price entry.
Reductions were also tried under STRICT_WITH_SIZE and with more than the lots hold (an error of the reference, no entry).

## Output

implicit-prices (line of the transaction, not of the posting):

```text
2026-01-02,BTC,100.00,EUR      line 6
2026-01-03,BTC,2.5,EUR         line 10
2026-01-04,BTC,101.50,EUR      line 14
2026-01-05,BTC,12.5,EUR        line 18
```

implicit-prices-cost-and-price (the price annotation wins; the cost with a date and a label is dated the transaction's day):

```text
2026-01-02,BTC,12.50,EUR      {10 EUR} @ 12.50 EUR
2026-01-04,BTC,10,EUR         {10 EUR} @ 10 EUR
2026-01-05,BTC,25,EUR         {{20 EUR}} @@ 50 EUR on 2 units
2026-01-06,BTC,10,EUR         {10 EUR, 2026-01-02, "lot"}
2026-01-07,BTC,12,EUR         {10 # 4 EUR} on 2 units
```

implicit-prices-reductions (a buy of 4 on 01-02 at 10; the sales of 01-03, 01-04 make nothing; 01-05 and 01-06 have a price; on 01-07
nothing is left, so `-1 BTC {2026-01-02}` adds a negative lot):

```text
2026-01-02,BTC,10,EUR
2026-01-05,BTC,14,EUR
2026-01-06,BTC,15,EUR
2026-01-07,BTC,10,EUR
```

implicit-prices-booking-methods (buys on 01-02 at 10, 20, 30, 40 and on 01-03 at 11, 21, 31, 41 in four accounts; on 01-04 the FIFO, LIFO
and HIFO sales make nothing, the NONE account's `-1 DDD {42 EUR}` makes `2026-01-04,DDD,42,EUR`).

implicit-prices-dedupe (dropped: the second 100.00 of 01-02, the 100.000 of 01-03, the second `100 EUR` of 01-08):

```text
2026-01-02,BTC,100.00,EUR
2026-01-03,BTC,100.0,EUR
2026-01-04,BTC,100,EUR
2026-01-04,BTC,100,USD
2026-01-05,BTC,100,EUR
2026-01-05,ETH,100,EUR
2026-01-06,BTC,100,EUR
2026-01-07,BTC,100,EUR
2026-01-08,BTC,100,EUR
2026-01-08,BTC,101,EUR
```

implicit-prices-duplicates (directives at 90.00 above and 110.00 below the transaction; on 01-04 a directive and a posting of
100.00 are both listed):

```text
2026-01-02,BTC,90.00,EUR      directive, line 6
2026-01-02,BTC,100.00,EUR     posting, line 8
2026-01-02,BTC,110.00,EUR     directive, line 12
2026-01-04,BTC,100.00,EUR
2026-01-04,BTC,100.00,EUR
```

implicit-prices-numbers:

```text
2026-01-02,BTC,15,EUR                                  @ (10 + 5) EUR
2026-01-03,BTC,1.25,EUR                                4 BTC @@ (20 / 4) EUR
2026-01-04,BTC,12,EUR                                  (1 + 1) BTC {3 * 4 EUR}
2026-01-05,BTC,3.333333333333333333333333333,EUR       3 BTC @@ 10.00 EUR
2026-01-06,BTC,3.333333333333333333333333333,EUR       3 BTC {{10.00 EUR}}
2026-01-07,BTC,2.0E+2,EUR                              0.5 BTC @@ 100 EUR
2026-01-08,BTC,1E-7,EUR                                10000000 BTC @@ 1 EUR
2026-01-09,BTC,1.000E+6,EUR                            0.001 BTC @@ 1000 EUR
2026-01-10,BTC,1234.500,EUR                            @ 1,234.500 EUR
2026-01-11,BTC,12.5,EUR                                {10 # 5 EUR} on 2 units
2026-01-12,BTC,3.333333333333333333333333333,EUR       -3 BTC @@ 10.00 EUR
```

implicit-prices-cost-places (`0.5 BTC {{100 EUR}}` is `200`, a cost written `(100 / 0.5)` is `2.0E+2`, the places of the cost as booked, a cost of zero):

```text
2026-01-02,BTC,200,EUR
2026-01-03,BTC,2.0E+2,EUR
2026-01-04,BTC,10.50,EUR
2026-01-05,BTC,10.50,EUR
2026-01-06,BTC,10.5,EUR
2026-01-07,BTC,0,EUR
```

implicit-prices-rejected (01-03 and 01-08, two postings to work out, are rejected by booking: no entry; unbalanced, unknown
account and closed account keep theirs):

```text
2026-01-02,BTC,100,EUR
2026-01-04,BTC,102,EUR
2026-01-06,BTC,103,EUR
2026-01-07,BTC,104,EUR
2026-01-09,BTC,106,EUR
```

implicit-prices-twice (three `plugin` lines): the entry `2026-01-02,BTC,100,EUR` three times, all on line 5. implicit-prices-order (two
lines, one transaction with `5 EUR`, `6 USD`, `7 EUR` postings, a directive above and below):

```text
2026-01-02,AAA,1,EUR        directive
2026-01-02,AAA,5,EUR        first run
2026-01-02,AAA,7,EUR
2026-01-02,BBB,6,USD
2026-01-02,AAA,5,EUR        second run
2026-01-02,AAA,7,EUR
2026-01-02,BBB,6,USD
2026-01-02,AAA,2,EUR        directive
```

implicit-prices-convert (value in EUR; ETH on 01-03: the posting (30) is written above the directive (31), the directive wins; on 01-05 the
directive (40) is above the posting (41), the posting wins; 2 BTC at 100.00 on 01-02 is `200.00`, on 01-04 at `520 / 4 = 130` is `260`):

```text
01-02  Assets:Btc 2 BTC -> 200.00 EUR     Assets:Usd 100 USD -> 50.0 EUR
01-03  Assets:Eth 1 ETH -> 31 EUR
01-04  Assets:Btc 2 BTC -> 260 EUR
01-05  Assets:Eth 1 ETH -> 41 EUR
```

## What ledgermill does

Built. `prices()`, `value()` and `holdings()` are those of the reference on every probe above, on 196 probe journals of
the corpus with the plugin line added at the end and on the app's 13 Beancount journals with it, compared by the harness
(`prices` with the line of each entry, `value` and `holdings` on the dates of the table). The entries are made from the transactions
booking kept, in the order of their booked postings, dropped as above, repeated once per plugin line and listed with the `price` directives
by date and line. Whether a posting opened a lot is known from booking (it is what decides the lot digits of a purchase and a sale). Refused,
still: a configuration string, every other module, and the shapes that booking refuses (`number-too-precise`, `inferred-cost`, …),
which keep the file refused with or without the plugin.
