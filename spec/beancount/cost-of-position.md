# The cost of a position

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-23.

Status: to build (`Ledger::at_cost()` for Beancount, which today is refused).

## Rule

The query Plainledger runs is the postings query with `number(cost(position)) AS amount,
currency(cost(position)) AS commodity` added. `cost(position)` of a posting is one amount:

- **With a cost:** the units multiplied by the cost per unit (the booked one: lots-and-booking.md, a total
  `{{ }}` is already divided by the units), in the currency of the cost. The product keeps the places of both
  numbers added (`3 * 33.33` is `99.99`, `2.50 * 4` is `10.00`, `0.5 * 200` is `100.0`), is rounded to 28
  significant digits (`3 * 3.333333333333333333333333333` is `9.999999999999999999999999999`), and prints as
  numbers-and-interpolation.md says (`1E-7`).
- **Without a cost:** the units, in their commodity, as written. A price (`@`, `@@`) is not a cost and is not
  looked at.
- A cost of zero gives a zero with the places of the product (`2.50 * 0` is `0.00`); negative units give a
  negative zero, `-0.000`, as in convert-rates.md.
- A posting booked against several lots gives one row per lot, each its own cost (the last transaction).

The row order and the rows are those of the postings query (postings.md): the cost does not change them.
ledgermill gives the same postings, each with one amount in the cost currency, and no cost or lot left on it.

## Probe

`probes/cost-of-position.beancount`

Command:

```sh
spec/beancount/show-cost.sh spec/beancount/probes/cost-of-position.beancount
```

`show-cost.sh` runs that query with the posting columns cut to account, units, the cost per unit and its
currency, then the amount and commodity of `cost(position)`, and the line. `bean-check` has no output (exit 0).

## Output

```text
== cost-of-position.beancount
-- cost: account,number,currency,cost_number,cost_currency,amount,commodity,lineno
Assets:Lots,3,AAA,33.33,EUR,99.99,EUR,9
Assets:Lots,2.50,AAA,4,EUR,10.00,EUR,10
Assets:Lots,1,AAA,1E-7,EUR,1E-7,EUR,11
Assets:Lots,10000000,AAA,1000000000,EUR,10000000000000000,EUR,12
Assets:Lots,3,AAA,3.333333333333333333333333333,EUR,9.999999999999999999999999999,EUR,13
Assets:Lots,0.5,AAA,200,EUR,100.0,EUR,14
Assets:Lots,2,AAA,102.5,EUR,205.0,EUR,15
Equity:Other,-10000000000000424.99000010000,EUR,,,-10000000000000424.99000010000,EUR,16
Assets:Plain,5,BBB,,,5,BBB,19
Assets:Plain,4,CCC,,,4,CCC,20
Assets:Plain,7.50,EUR,,,7.50,EUR,21
Equity:Other,-29.50,EUR,,,-29.50,EUR,22
Assets:Free,2.50,FFF,0,EUR,0.00,EUR,25
Assets:Free,-1.5,GGG,0.00,EUR,-0.000,EUR,26
Assets:Sold,2,HHH,10,EUR,20,EUR,30
Assets:Sold,3,HHH,12.50,EUR,37.50,EUR,31
Equity:Other,-57.50,EUR,,,-57.50,EUR,32
Assets:Sold,-2,HHH,10,EUR,-20,EUR,34
Assets:Sold,-3,HHH,12.50,EUR,-37.50,EUR,34
Equity:Other,57.50,EUR,,,57.50,EUR,35
```
