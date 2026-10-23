# Converting a position (units, cost currency)

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-23.

Status: to build (`Ledger::value()` and `Ledger::holdings()` for Beancount). The rate of one pair of commodities
is in convert-rates.md; this note is what `convert(position, "V")`, `convert(position, "V", date)` and
`convert(cost(position), "V")` do with a posting.

## Rule

`convert(position, V, D)` turns a posting's position, units and cost, into one amount: `number()` and
`currency()` of the result are what the value query reads. In order:

1. **Units already in `V`:** the units, as they are. The cost is not looked at (`10 EUR {1.1 USD}` valued in
   `EUR` is `10 EUR`).
2. **A rate from the units' commodity to `V`** (convert-rates.md, direct or inverse): the units multiplied by
   that rate, in `V`. The cost plays no part, whatever it is.
3. **Otherwise, for a position that has a cost, through the cost's currency `K`:** needs a rate from the
   units to `K` **and** one from `K` to `V`. The units are converted to `K` (rounded to 28 digits), then that amount
   to `V` (rounded again): `(units * rate1) * rate2`, not `units * (rate1 * rate2)`; the last digit can differ
   (below). Either rate may be an inverse. A zero rate gives a zero amount. This is the only bridge: no chain
   of three commodities, and when one of the two rates is missing the position is not partly converted.
4. **Otherwise,** the units, as they are (in their own commodity).

The cost is the currency it was booked in: a position `{3 USD}` bridges through `USD`; the cost's number is
never a rate, and a cost in `V` itself (`{3 EUR}`) has no bridge to take (rule 2 or nothing). A posting
without a cost has no bridge.

**`@` and `@@` are not rates.** A price written on a posting is not read by `convert()`, with or without a
cost, whatever it says, and a `price` directive of the commodity is used as always (below: `PPP` is worth
`5 EUR`, not the `3 EUR` it was bought at). `{}` is not a rate either: `2 CCC {80 EUR}` valued in `EUR` stays
`2 CCC`.

**`convert(cost(position), V)`** converts the cost amount (cost-of-position.md: the units times the cost per unit, in
the cost currency, or the units when there is no cost): rule 1 and rule 2 only, no cost is left to bridge
through. An amount in a commodity with no rate to `V` stays as it is: `(30 K5)`.

Sums of these amounts are in holdings.md.

## Probe

`probes/convert-cost-bridge.beancount`, `probes/convert-annotations.beancount`,
`probes/convert-bridge-arith.beancount`

Command:

```sh
spec/beancount/show-holdings.sh spec/beancount/probes/convert-cost-bridge.beancount EUR - '^Assets:'
spec/beancount/show-holdings.sh spec/beancount/probes/convert-annotations.beancount EUR - '^Assets:'
spec/beancount/show-holdings.sh spec/beancount/probes/convert-bridge-arith.beancount EUR - '^Assets:'
spec/beancount/show-value.sh spec/beancount/probes/convert-bridge-arith.beancount EUR 2026 2 1 '^Assets:'
```

`bean-check` has no output (exit 0) for the three journals. The `units` section of `show-holdings.sh` is left
out below (holdings.md has it); the columns are account, currency, units, cost basis, cost basis valued,
market value.

## Output

convert-cost-bridge (ten units of each; `T1` to `T9` have a cost `{3 Kn}` in their own currency, `Kn`, and
prices as the journal's comments say):

```text
Assets:Direct,T1,(10 T1)
Assets:UnitsAreTarget,EUR,(10 EUR)
Assets:Bridge,T2,(10 T2)
Assets:InverseFirstLeg,T3,(10 T3)
Assets:InverseSecondLeg,T4,(10 T4)
Assets:NoSecondLeg,T5,(10 T5)
Assets:NoFirstLeg,T6,(10 T6)
Assets:NoPrice,T7,(10 T7)
Assets:ZeroSecondLeg,T9,(10 T9)
Assets:CostIsTarget,T10,(10 T10)
Assets:NoCost,T11,(10 T11)
Assets:CostIsTarget,T11,(10 T11)
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
Assets:Direct,T1,(10 T1),(30 K1),(60 EUR),(50 EUR)
Assets:UnitsAreTarget,EUR,(10 EUR),(11.0 K1),(22.0 EUR),(10 EUR)
Assets:Bridge,T2,(10 T2),(30 K2),(60 EUR),(200 EUR)
Assets:InverseFirstLeg,T3,(10 T3),(30 K3),(60 EUR),(200 EUR)
Assets:InverseSecondLeg,T4,(10 T4),(30 K4),(60 EUR),(200 EUR)
Assets:NoSecondLeg,T5,(10 T5),(30 K5),(30 K5),(10 T5)
Assets:NoFirstLeg,T6,(10 T6),(30 K6),(60 EUR),(10 T6)
Assets:NoPrice,T7,(10 T7),(30 K8),(60 EUR),(10 T7)
Assets:ZeroSecondLeg,T9,(10 T9),(30 K9),(),()
Assets:CostIsTarget,T10,(10 T10),(30 EUR),(30 EUR),(10 T10)
Assets:NoCost,T11,(10 T11),(10 T11),(50 EUR),(50 EUR)
Assets:CostIsTarget,T11,(10 T11),(30 EUR),(30 EUR),(50 EUR)
```

- `Direct`: `T1` has a price in EUR itself (50), which beats the bridge through `K1` (it would give 200).
- `Bridge`, `InverseFirstLeg`, `InverseSecondLeg`: 10 * 10 * 2 = 200.
- `NoSecondLeg`: `T5` has a price in `K5` but `K5` has none in EUR: `10 T5`, not `100 K5`.
- `NoFirstLeg`: no price from `T6` to `K6`, only through `K7`: `10 T6`.
- `NoPrice`: no price of `T7`: `10 T7`, although `K8` is worth 2 EUR.
- `ZeroSecondLeg`: `K9` is worth zero: a zero in EUR (an empty cell, holdings.md).
- `CostIsTarget`: `T10 {3 EUR}` has no price in EUR and stays `10 T10`; `T11 {3 EUR}` has one (5): `50 EUR`.
- `UnitsAreTarget`: `10 EUR {1.1 K1}` is `10 EUR`; its cost basis valued is `22.0 EUR` (11.0 K1, direct rate of 2).
- `NoCost`: `T11` without a cost converts by its price in EUR (50), and so does its cost basis (the units).

convert-annotations (`UUU`: `@ 100 EUR`; `TTT`: `@@ 100 EUR`; `CCC`: `{80 EUR}`; `BBB`: `{80 EUR} @ 100 EUR`;
`PPP`: `@ 3 EUR` and a price directive of 5 EUR):

```text
Assets:UnitPrice,UUU,(2 UUU)
Assets:TotalPrice,TTT,(-2 TTT)
Assets:Cost,CCC,(2 CCC)
Assets:CostAndPrice,BBB,(2 BBB)
Assets:PriceAndDirective,PPP,(2 PPP)
Assets:Native,EUR,(10 EUR)
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
Assets:UnitPrice,UUU,(2 UUU),(2 UUU),(2 UUU),(2 UUU)
Assets:TotalPrice,TTT,(-2 TTT),(-2 TTT),(-2 TTT),(-2 TTT)
Assets:Cost,CCC,(2 CCC),(160 EUR),(160 EUR),(2 CCC)
Assets:CostAndPrice,BBB,(2 BBB),(160 EUR),(160 EUR),(2 BBB)
Assets:PriceAndDirective,PPP,(2 PPP),(2 PPP),(10 EUR),(10 EUR)
Assets:Native,EUR,(10 EUR),(10 EUR),(10 EUR),(10 EUR)
```

convert-bridge-arith (12889439741600 `QQQ` `{1 USD}`; `QQQ` is 35884403.57 `USD`; `EUR` is 62.2 `USD`):

```text
Assets:Lot,QQQ,(12889439741600 QQQ)
-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value
Assets:Lot,QQQ,(12889439741600 QQQ),(12889439741600 USD),(207225719318.3279742765273312 EUR),(7436171342102426326.559485529 EUR)
== convert-bridge-arith.beancount V=EUR on 2026-2-1
-- value: account,number,currency,amount,commodity,lineno
Assets:Lot,12889439741600,QQQ,7436171342102426326.559485529,EUR,10
```

The market value is `7436171342102426326.559485529`: the two products one after the other (the units times
35884403.57, rounded to 28 digits, times `1 / 62.2`). The product of the two rates first would end in `530`.
The value on a date and the value with no date agree.
