# Balance assertions: what is compared

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

Status: built.

## Rule

An assertion on a posting compares what the posting's account holds at that point (after the
posting itself; which postings come before it is in [assertion-order](assertion-order.md)) with
the asserted amount. The syntax is in [assertion-syntax](assertion-syntax.md); what hledger says
when the comparison fails is in [assertion-failures](assertion-failures.md).

**The running balance.** Per account, per commodity, the sum of every posting to that account so
far: regular, `(virtual)` and `[balanced virtual]` postings alike (`(a)` and `[a]` are the account
`a`), every status, elided amounts once worked out. An account never used holds nothing.
Aliases and `apply account` have already been applied to the names.

**Costs and lots never count.** A posting `3 X @ 2 EUR` adds 3 X and nothing in EUR; `2 X {10 EUR}`
adds 2 X. Postings of one commodity with different prices or lots add up into one balance of that
commodity (`3 X @ 2 EUR` and `2 X @ 3 EUR` make 5 X). A price written in the asserted amount is
ignored too (`= 4 X @ 2 EUR` asks for 4 X).

**`=` (one commodity).** The balance in the asserted amount's commodity must equal the asserted
quantity. The other commodities of the account are not looked at. A commodity the account does not
hold has the balance 0, so `= 0 JPY` holds and `= 5 USD` fails. A number with no commodity is the
commodity named `""` (nothing), a commodity like any other: on an account holding only EUR,
`= 0` holds and `= 10` fails. (With a `D` directive in the file, such a number takes the `D`
commodity first.) Commodity names are case sensitive (`eur` is not `EUR`).

**`==` (all commodities).** The same comparison for the asserted commodity, and every other
commodity of the account must have a balance of exactly 0 (a commodity that went to zero, like
`3 USD` then `-3 USD`, is fine). `== 0 EUR` and `== 0` on an account that holds nothing hold;
`== 0` on an account holding anything non-zero fails.

**`=*` and `==*`.** The balance is the sum over the account itself and every account below it
(`a:b`, `a:b:c`; an account is below `a` only if its name starts with `a:`, so `ab` is not).
`==*` then also wants every other commodity of that sum to be 0. Without the `*`, sub-accounts
are left out.

**Which other commodities `==*` looks at.** Only those the account holds itself, not those that only
its sub-accounts hold, and for each the sum over the whole subtree must be 0. With `a` 2 EUR, `a:b`
-3 USD and `a:b:c` 4 GBP, `a ==* 2 EUR` holds (`assert-total-inclusive-commodities`); with `a` -3 USD
and `a:b` 2 EUR it fails, in USD, calculated -3 (`assert-fail-total-inclusive-own`, line 7); with `a` 3 USD and
`a:b` 1 USD it fails, calculated 4 (`assert-fail-total-inclusive-sum`, line 8). hledger says `Across all
commodities ... including subaccounts` for these. (The assignment `==*` zeroes every commodity of the subtree:
[assignments](assignments.md).)

**Exact.** The two quantities must be equal as numbers, with no tolerance and no rounding:
`1.00` equals `1`, `2.750` equals `2.75`, `0.1e2` equals `10`; a difference in the fortieth
decimal place fails. The places written in the postings or in the assertion have no effect on
the verdict.

Twin probes (each failing journal holds the one failing assertion; `assert-compare` holds the
passing counterparts):

| Passes in `assert-compare` | Fails in |
|---|---|
| `a = 5 USD` while `a` also holds EUR, X, Y (others ignored) | – |
| `a = 5 X`: `3 X @ 2 EUR` + `2 X @ 3 EUR` | `assert-fail-cost`: `= 6 EUR` (the cost is not a balance) |
| `a = 9 Y`: lots of different costs add up | – |
| `a = 0 JPY`, `a = 0`, `c = 0 EUR` (never used) | `assert-fail-absent-commodity` (`= 5 USD`), `assert-fail-commodity-less` (`= 10`), `assert-fail-unused` (`= 3 EUR`) |
| `a = 2.000 GBP`, `a = 0.1e2 EUR`, `= 10 EUR @ 99 USD` | `assert-fail-exact` (`1.00` vs `1.004`), `assert-fail-exact-places` (40 places) |
| `w == 10 EUR` while `w` also holds `3 USD` and `-3 USD` | `assert-fail-total-other`, `assert-fail-total-absent`, `assert-fail-total-several`, `assert-fail-total-asserted-first` |
| `c == 0`, `c == 0 EUR` on a fresh account | – |
| `a =* 17 EUR` (`a` 10, `a:s` 7, not `ab`), `ab = 100 EUR` | `assert-fail-exclusive` (`= 9`), `assert-fail-inclusive` (`=* 7`), `assert-fail-sibling` (`ab` is not below `a`) |
| – | `assert-fail-case` (`1 eur`) |

## Probe

`probes/assert-compare.journal` and the `probes/assert-fail-*.journal` of the table above;
`probes/assert-total-inclusive-commodities.journal`, `probes/assert-fail-total-inclusive-own.journal` and
`probes/assert-fail-total-inclusive-sum.journal` for `==*`.

Command:

```sh
spec/hledger/assertion-verdicts.sh spec/hledger/probes/assert-compare.journal
spec/hledger/assertion-verdicts.sh spec/hledger/probes/assert-fail-cost.journal \
  spec/hledger/probes/assert-fail-absent-commodity.journal \
  spec/hledger/probes/assert-fail-commodity-less.journal \
  spec/hledger/probes/assert-fail-unused.journal \
  spec/hledger/probes/assert-fail-exact.journal \
  spec/hledger/probes/assert-fail-exact-places.journal \
  spec/hledger/probes/assert-fail-total-other.journal \
  spec/hledger/probes/assert-fail-total-absent.journal \
  spec/hledger/probes/assert-fail-total-several.journal \
  spec/hledger/probes/assert-fail-total-asserted-first.journal \
  spec/hledger/probes/assert-fail-exclusive.journal \
  spec/hledger/probes/assert-fail-inclusive.journal \
  spec/hledger/probes/assert-fail-sibling.journal \
  spec/hledger/probes/assert-fail-case.journal
```

## Output

The first prints `accepts` (all 26 assertions of the journal hold). The others (the last three
columns are the position, the kind, and what hledger says it compared; `USD0` is how it writes an
absent commodity):

```text
assert-compare | accepts |
assert-fail-cost | rejects | assert-fail-cost.journal:6:10 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 6 EUR; calculated EUR0
assert-fail-absent-commodity | rejects | assert-fail-absent-commodity.journal:6:12 | assertion-failed: In commodity USD, excluding subaccounts; asserted 5 USD; calculated USD0
assert-fail-commodity-less | rejects | assert-fail-commodity-less.journal:6:12 | assertion-failed: In commodity "", excluding subaccounts; asserted 10; calculated 0
assert-fail-unused | rejects | assert-fail-unused.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 3 EUR; calculated 0 EUR
assert-fail-exact | rejects | assert-fail-exact.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 1.004 EUR; calculated 1.00 EUR
assert-fail-exact-places | rejects | assert-fail-exact-places.journal:6:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 0.1234567890123456789012345678901234567891 EUR; calculated 0.1234567890123456789012345678901234567890 EUR
assert-fail-total-other | rejects | assert-fail-total-other.journal:8:12 | assertion-failed: Across all commodities, excluding subaccounts; asserted 0 USD; calculated 5 USD
assert-fail-total-absent | rejects | assert-fail-total-absent.journal:6:12 | assertion-failed: Across all commodities, excluding subaccounts; asserted 10 USD; calculated USD0
assert-fail-total-several | rejects | assert-fail-total-several.journal:8:12 | assertion-failed: Across all commodities, excluding subaccounts; asserted 0 GBP; calculated 1 GBP
assert-fail-total-asserted-first | rejects | assert-fail-total-asserted-first.journal:7:12 | assertion-failed: Across all commodities, excluding subaccounts; asserted 10 USD; calculated 5 USD
assert-fail-exclusive | rejects | assert-fail-exclusive.journal:7:12 | assertion-failed: In commodity EUR, excluding subaccounts; asserted 9 EUR; calculated 7 EUR
assert-fail-inclusive | rejects | assert-fail-inclusive.journal:7:12 | assertion-failed: In commodity EUR, including subaccounts; asserted 7 EUR; calculated 9 EUR
assert-fail-sibling | rejects | assert-fail-sibling.journal:7:12 | assertion-failed: In commodity EUR, including subaccounts; asserted 9 EUR; calculated 7 EUR
assert-fail-case | rejects | assert-fail-case.journal:6:12 | assertion-failed: In commodity eur, excluding subaccounts; asserted 1 eur; calculated eur0
```
