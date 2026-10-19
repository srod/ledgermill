# PROTOTYPE: observed behaviour of native `hledger print -O json` (1.52.4)

Clean-room notes. Everything here was learned by running the native `hledger`
binary on journals we wrote (`probes/`) and on our first user's test journals.
No hledger source code was read. Probe file names are given as evidence; rerun
`hledger -f probes/<file> print -O json` to see the exact output.

Notation: `(m,d)` = `decimalMantissa m, decimalPlaces d`, value `m × 10^-d`.

## Fields our first user reads (the comparison target)


- transaction: `tdate`, `tstatus` (`Unmarked`/`Cleared`/`Pending`), `tdescription`,
  `tcomment` (trimmed), `tsourcepos[0].sourceName` + `.sourceLine`
- posting: `paccount`, `ptype` (`RegularPosting`/`VirtualPosting`/`BalancedVirtualPosting`),
  `pcomment` (trimmed), `pamount[]`
- amount: `acommodity`, `aquantity (m,d)`, `acost` (`null` or `{tag: UnitCost|TotalCost, contents: {acommodity, aquantity}}`),
  `acostbasis` (`null` or `{cbCost: {acommodity, aquantity} | null, cbDate, cbLabel}`)

Everything else (`astyle`, `floatingPoint`, `ptags`, `tindex`, `pdate`, …) is out of scope.
Transactions are emitted sorted by date, stable in parse order (files in `-f` order).

## Parsing

**Dates.** `2026-01-05`, `2026/01/05`, `2026.1.7`, unpadded `2026-1-5`; `Y 2025` + `01-02` or `1/3`
(partial dates). Output ISO padded. `DATE=DATE2` secondary date allowed (ignored here).

**Header.** `DATE[=DATE2] [*|!] [(CODE)] DESCRIPTION [; COMMENT]`.
- `*` → Cleared, `!` → Pending, none → Unmarked.
- Description ends at the first `;` even without a space (`Coffee;memo` → `Coffee` + comment `memo`); trailing spaces trimmed.
  `|` kept verbatim (`Pay | note`). Tabs and control chars kept. Empty description → `""`.
- `2026-01-02   (x)   ` → code `x`, description `""`.

**Comments.**
- Column-0 line comments: `;`, `#`, `*`. Block `comment` … `end comment` is ignored.
- tcomment = header comment + each indented `;` line before the first posting, one per line
  (each trimmed). Our first user trims the whole thing, so only inner `\n` joins matter:
  header `; c1` + next line `; second` → `c1\nsecond`.
- pcomment = same-line comment + following indented `;` lines up to the next posting
  (`;  x  y  ` + `   ;   z  ` → `x  y\nz`). An indented `;` after a blank line is ignored (q1809).
- A posting with no amount needs 2+ spaces before `;`: `b ; c` is account `b ; c`; `b;c` is account `b;c`.
- `e  1 EUR;x` → amount then comment `x` (no space needed after an amount).

**Posting line.** Indent (space or tab), optional `*`/`!` + space, account, then 2+ whitespace chars, then amount.
- The account ends at 2+ consecutive whitespace characters (spaces/tabs, any mix). A single tab is NOT a
  separator and becomes a space in the name: `\tg\t-1 EUR` → account `g -1 EUR`, no amount (u2, u3);
  `e\t\t1 EUR` and `g \t-1 EUR` separate (q1512, u3b); `a\tb  1 EUR` → account `a b`.
- Trailing whitespace trimmed from the account.
- `(acc)` → VirtualPosting, `[acc]` → BalancedVirtualPosting, else RegularPosting.

**Amounts.**
- Commodity left or right, spaced or not: `10EUR`, `EUR10`, `EUR 10`, `$-3`, `-$3`, `-EUR 5`, `EUR -5`, `- 10 EUR`, `-  EUR 10`, `+1`.
- Quoted commodities: `"AB C" 10`, `10 "AB 1"` (quotes stripped). Symbols like `$`, `€` fine.
- Number forms: `-.5` → (-5,1); `5.` → (5,0); `1e2` → (100,0); `1.5e2` → (150,0); `1E-2` → (1,2); `1.5E` → parse error.
- Trailing zeros are kept on written amounts: `1.50` → (150,2).
- More than 10 decimal places are rounded to 10: `0.000000000001` → (0,10), `1.123456789012` → (11234567890,10).
- NBSP / narrow NBSP digit groups → parse error.
- Commodity-less amount → commodity `""`. `D 1,000.00 EUR` gives commodity-less amounts the commodity `EUR` (file-scoped: da/db).

**Decimal mark (per amount).**
- Two different separators: the last one is the decimal mark (`1,000.5`, `1.000,5`).
- One separator used twice or more: digit grouping (`1,000,000` → (1000000,0); `1.000.000` same).
- Space is always a digit group (`1 000,5` → (10005,1); `1 000.5` → (10005,1); `1 000` → (1000,0)).
- One `.` or `,` once, followed by other than exactly 3 digits: decimal mark (`1,5` → (15,1); `10,00` → (1000,2)).
- **Ambiguous** — one `.` or `,` once, followed by exactly 3 digits (`1,000`, `1.000`):
  - `decimal-mark ,` or `decimal-mark .` directive in the same file, if seen before, decides (it beats `D` and `commodity`);
  - else a `commodity` directive for that commodity seen before in the same file (`commodity 1,000.00 EUR`,
    `commodity 1. EUR`, or `commodity EUR` + indented `format 1.000,00 EUR`) decides for that commodity;
  - else a `D` directive seen before in the same file decides, for ALL commodities (q1406: `D 1.000,00 EUR` then `1.000 USD` → (1000,0) and `1,000 USD` → (1,000 → (1000,3));
  - else it is a decimal mark: `1,000` → (1000,3), `1.000` → (1000,3) — even if earlier amounts used `1,000.5` (q1902, q1903).
  - Directives are file-scoped and only affect later lines (x6, x7, cd1/cd2, dd1/dd2, dm1/dm2).
- Exception: when the number also has space digit groups, a lone `.`/`,` is the decimal mark whatever was declared
  (`commodity 1,000.00 EUR` then `1 000,50 EUR` → (100050,2); dm*-mark). Without spaces the declared mark wins
  even when the separator is not followed by 3 digits (`commodity 1,000.00 EUR` then `1,5 EUR` → (15,0)).
- `alias /RE/` regexes are POSIX-ERE-like: `\w`, `\d`, `\s` are not classes (they match the plain letter); lazy or
  non-capturing syntax is an error.

**Directives.**
- `alias OLD = NEW`: replaces OLD when it is a whole leading segment run (`alias a:b = z`: `a:b:c` → `z:c`, `a:bc` unchanged).
- `alias /RE/ = REPL`: regex, case-insensitive (`/FOOD/` matched `food`), `\1` back-references; `\w` is not supported (did not match).
- Several aliases: the most recently declared is applied first, each at most once (`alias exp = expenses`, then `alias expenses = spend`: `exp:food` → `expenses:food`, `expenses:x` → `spend:x`).
- `end aliases` clears them. Aliases are file-scoped (sibling-alias test journals, p4a/p4b).
- `apply account x` … `end apply account` prefixes accounts with `x:`.
- Ignored for these fields: `account` (except `type:`, see below), `payee`, `tag`, `commodity` (except decimal mark), `~` periodic and `=` auto-posting blocks (their indented lines too).
- `include`, `!include`, `plugin`, `document`, `option` are refused by our first user before hledger runs — out of scope.

## Balancing (the core)

Two groups are checked separately: regular postings, and balanced-virtual `[...]` postings. Virtual `(...)` postings are never balanced.

**Cost of a posting.**
- `Q C @ P D` → `acost UnitCost P` as written (trailing zeros kept: `@ 0.50` → (50,2)); contributes `normalize(Q × P)` in D.
- `Q C @@ T D` → `acost TotalCost`, value `normalize(T)` negated when Q < 0 (`-2 @@ 100` → -100, `2 @@ -100` → -100, `-2 @@ -100` → 100, `0 @@ 100` → 100); contributes that value in D.
- `normalize` = strip trailing zeros (`@@ 100.00` → (100,0); `@@ 1.50` → (15,1); `2 X @ 0.25` contributes (5,1)).
- A lot `{C}` / `{=C}` / `{{T}}` alone sets `acostbasis` only; it does NOT create `acost` and does NOT convert for balancing
  (`1 X {1 EUR}` + elided → elided gets `-1 X`). `{{T}}` → `cbCost = T / Q` to 10 dp (`2 X {{160 EUR}}` → 80; `3 X {{10 EUR}}` → (33333333333,10); `-3` → negative).
  Ledger-style `[2026-01-02]` → `cbDate`, `(lbl)` → `cbLabel`. Beancount-style `{80 EUR, 2026-01-02, "lbl"}` is a parse error. `{}` → `cbCost null`.
- `{C} @ P` → both `acostbasis` and `acost`; balancing uses `@`.

**Elided amount.** At most one posting without an amount per group (two → error, y14). It gets the negated sum of the other postings in its group (costs converted), one component per non-…-commodity, sorted by commodity (codepoint order: `AA`, `ZZ`, `bb`).
- Zero-sum commodities are kept as `0` components when the posting has other components (`[0 EUR, -1 USD]`, q1005, q1309); a single zero sum gives one `0` amount (`c` → (0,1) for `1.5 - 1.5`).
- decimalPlaces of a sum = max decimalPlaces of its terms (no normalisation): `1.5 + -1.50` → (0,2); `1.50 + 1.50` → (-300,2).
- A commodity contributed by exactly one amount keeps that amount's `acostbasis` (q1605: `-1 X {1 EUR}`); summing two or more amounts drops it (q1601).
- A virtual `(f)` posting with no amount → one amount `0`, commodity `""` (y10, y11).

**Tolerance.** A group balances when, for every commodity, the residual rounded half-to-even to that commodity's
precision is zero. Precision = max decimalPlaces of the written posting amounts of that commodity in the transaction;
if there are none, max decimalPlaces of the price/cost amounts written in that commodity.
- `3 X @ 0.5 EUR` + `-1 EUR` balances (residual 0.5 → 0, q911); `1 X @ 2.5 EUR` + `-1 EUR` fails (1.5 → 2, q1212);
  `1 X @ 0.44 EUR` + `-0.4 EUR` balances (precision 1, q? k5); `0.46` fails (k6); `0.45` balances (k7, half-even).
- `1.5 X @ 0.3 EUR` + `-1 Y @ 0.4 EUR` balances (no EUR posting amount, precision 1 from the prices, residual 0.05 → 0.0, q1501).
- Adding `g  0 EUR` sets precision 0 and makes `@ 0.4` / `@ 0.3` balance (q1215).
- `1.004 EUR` + `-1.00 EUR` fails (precision 3, q901, q1221). Other transactions and `commodity` directives don't change precision (q902, q906).

**Price inference (no `@` anywhere in the group, no elided posting).** Ignore commodities whose sum is zero. If exactly two
commodities remain, the commodity of the first posting (in posting order) among them gets a cost in the other one:
- that commodity appears in exactly one posting → `TotalCost` = −(sum of the other commodity), not normalised
  (`1 X`, `-2 Y` → TotalCost 2 Y; `-1 X`, `2 Y` → -2 Y; `1 X`, `-1.5 Y`, `0.5 Y` → (10,1); `b -2 Y`, `e 1 X` → b gets TotalCost -1 X);
- it appears in two or more postings (zero-quantity ones count, q1308) → each gets `UnitCost` = −(sum other) / (sum this),
  rounded half-to-even to 10 dp, then normalised (`1 X`,`1 X`,`-3 Y` → 1.5; `3 X`,`4 X`,`-1 Y` → (1428571429,10); `1,1,-0.0000000005` → (2,10)).
- A posting with an explicit `@`/`@@` in the group → no inference; an unbalanced residual is an error (q1014, q1015).
- Three or more non-zero commodities → error. Works the same in a `[...]` group (q1612); never in `(...)`.
- Which posting gets the cost: the first posting holding an amount in one of the two unbalanced commodities. If that
  posting holds more than one amount (e.g. a `==` assignment that also zeroes another commodity), or any posting holding
  that commodity has more than one amount, there is no inference and the transaction is unbalanced (pi*-infer, lb23).
- Quantities are exact internally; only emitted costs/lots are rounded half-to-even to 10 places.

**Conversion postings.** Accounts named `equity:conversion`, `equity:trade`, `equity:trades`, `equity:trading` (case-insensitive,
or any sub-account of those), or declared `account X ; type: V`, are conversion accounts. When a posting `Q C @ P D` (or `@@`) is
accompanied by two **adjacent** conversion postings whose amounts are exactly `-Q C` and `+cost D` (either order, before or after),
the priced posting balances by its own quantity (its cost is not added). Mismatch (z6), a single conversion posting (z7),
non-adjacent pair (q1312), or one pair for two priced postings (q1311) → error. `equity:conversions`, `equity:x:conversion`,
plain `equity` are not conversion accounts (q1506–q1508).
- **Declared types switch the names off.** Once a file declares any account of type `V`/`Conversion`, the names above no longer
  make a conversion account in that file; only the declared accounts and their sub-accounts do (val-ty2, val-ty4, val-ty7,
  val-ty20–23). Declaring other types (`A`, `C`, `Cash`, `E`, `X`, …) changes nothing (val-ty1, val-ty3, val-ty14–17).
  Per file: a declaration in a sibling file has no effect (val-ty9a/b, both orders).
- An `account` directive is split like a posting: 2+ whitespace characters end the name, so `account x ; type: V` (one space) declares nothing
  and names an account `x ; type: V` (val-ty30).
- `type:` is read from the first `type:` tag of the `account` line comment or of its following indented `;` lines (the first tag wins:
  `type: A, type: V` is not V, `type: V, type: A` is, val-ty24/25); the value runs to the next comma or ` ;`. Values are case-insensitive
  and must be one of `A L E R X C V G U`, `Asset Liability Equity Revenue Expense Cash Conversion Gain UnrealisedGain`; anything else
  (empty, `assets`, `V W`, `Q`) is a **parse error** (val-ty18; the empty/`assets`/`V W` variants were tried by hand, only `Q` is a probe file).
- Two **adjacent** conversion-account postings (greedy left to right, any amounts, `(virtual)` included) where either carries a
  cost (`@`, `@@`) → error "Conversion postings must not have a cost" (val-cost11). A single priced one, a non-adjacent pair, a `[bv]` pair are fine (val-cost12).

## `P` directives (parsing, also in plain `print`)

`P DATE [HH:MM[:SS]] COMMODITY AMOUNT [anything]`. A bad `P` line fails the whole `print` even without `--value` (`P x 2 EUR`, bad date,
no price: val-p1-bad … val-p8-bad). Details, from val-g19-p-syntax, val-m4-d-year-p, val-m5-decimal-mark-p:
- date forms like transactions (`2026/01/01`, `2026-1-2`, partial date with `Y`); a time of day is accepted and ignored; `P2026-01-01 …` (no space after `P`) is accepted;
  `;` starts a comment; everything after the amount is ignored (`P … X 2 EUR @ 3 USD`, `… EUR 3 4 5`).
- commodity may be quoted (`"E F"`), must be followed by whitespace (`P 2026-01-01 x2 EUR` is an error); the price is a normal amount: `4EUR`, `EUR 3`, `-EUR 2`, `1e2`,
  `1,5`, `1 000.5`, negative, zero all work, with the per-file decimal-mark rules. `€2` is commodity `€`, not `EUR`.
- a commodity-less price takes the `D` commodity (val-m4); with no `D` its commodity is `""`, which is a real graph node (val-g20: `P X 2` values a bare `3` as `1.5 X`).
- the price keeps **full precision** (30 decimals survive, val-g13); the 10-place rounding applies only to what is printed.
- `P` is **global** across the `-f` files (val-f1+f2 value each other's commodities), whatever file order; for one ordered pair the latest date ≤ DATE wins; the
  same date → the later directive in parse order (files in `-f` order) wins (val-f5/f6 both orders, val-g18).

## Valuation

### `--cost`
- Every amount with a cost becomes an amount in the cost commodity and loses `acost` and `acostbasis`. `UnitCost`: `normalise(q × p)`;
  `TotalCost`: the stored total as is (an explicit `@@` was normalised when parsed, `-2 @@ 100` is already −100; a total *inferred* by balancing keeps its
  places, `1 X` / `-2.0 Y` → `2.0`, q1801). `{{T}}` plus `@@` (val-cost2) and lot plus price (valuation-costs) lose the lot.
- A lot with no price (`2 BTC {80 EUR}`) is untouched, lot included (valuation-costs).
- Virtual `(v)` and `[bv]` postings convert too (val-cost1). Zero results have 0 places (`0.00 X @ 3 EUR` → `0`, val-cost10).
- Precision: the inferred unit price (price inference, "3 X, 4 X, -1 Y") is kept to 255 places internally and printed rounded to 10; `--cost`
  multiplies the **unrounded** price (`2 X`, `-1 Y`, `1 X` → `0.6666666667` + `0.3333333333`, q1019). Products are capped at 255 places (below).
- **Postings are dropped** (the transaction stays, possibly with `"tpostings": []`, val-cost5 tx 1):
  1. adjacent conversion-account pairs, found greedily left to right on the whole transaction, any amounts, even with no priced posting
     (val-cost8, val-cost9: three → first two go, four → all, five → last stays; `conv, a, conv, conv` → only the last two) —
     **but only when the transaction has at least one non-conversion posting without a cost** (virtual ones count); a transaction made only of
     conversion postings, or whose only other posting is priced, keeps them all (val-cost13, val-cost15). Known gap: when two non-conversion
     postings have equal written amounts hledger often keeps the pairs too (gaps/val-cost14), rule not found;
  2. postings tagged `_conversion-posting:` (any value, comma-separated among other tags, own line comment or the following indented `;` lines;
     case-sensitive; `x_conversion-posting:` doesn't count) **unless they carry a cost** (val-cost4, val-cost1, val-cost5). A tag in the
     header comment does nothing (val-cost4). An elided posting with such a tag is dropped (val-cost5).
  The two sets are independent (a tagged posting plus an untouched pair leave only the unpaired third one, val-cost9 tx 7).
- A single `equity:conversion` posting is kept (valuation-cost-suppression-same-account); plain `print` never drops anything.

### `--value=DATE,COMM`
Per amount, independent of the transaction or posting date (only DATE matters, val-v1, val-f*):
- Amount already in COMM → untouched, cost and lot included. No price path → untouched, cost and lot included (val-g14, val-g15). Commodity-less amounts
  follow the same rules with commodity `""` (val-g20).
- Converted → loses `acost` and `acostbasis`; the cost commodity plays no role (val-g16: `3 X @ 2 EUR` with only EUR→USD known stays as it is).
- Value = `normalise(q × rate)`: trailing zeros removed (`1.50 X` at 2 → `3`), zero → `0`.

**Which prices exist on DATE.** For each ordered pair `(from, to)` the latest `P` with date ≤ DATE (the P on DATE itself counts, val-g17); same date → later in parse order (val-g18).
`P X 2 EUR` is an edge X→EUR with rate 2 (a *forward* edge) and also an *inverse* edge EUR→X with rate 1/2.
A zero price is a real edge; its inverse rate is 0 (1/0 does not fail, val-g10). Negative prices work. `P X 2 X` (same commodity) changes nothing.

**Path search** (val-g1 … val-g9, then fuzzed densely):
1. Look for a path using **forward edges only**; shortest wins, and among equally short paths the one with the smallest commodity names in
   codepoint order (`Z` < `a`, never locale order). A forward chain of any length beats any inverse path, even a direct one,
   even one with a newer date (val-g1, val-g2, val-g3: a 4-hop forward chain beats a direct inverse).
2. Only if there is none: shortest path over forward + inverse edges (val-g6, val-g7: a direct inverse beats a longer mixed path). Equal length → the path
   that leaves by a forward edge first, then by name (val-g8). This is breadth first with forward neighbours (sorted) tried before inverse neighbours (sorted).
3. Cycles are harmless (val-g9, val-m6).
4. The rate is the product of the edges along the path, folded from the amount's side; every product is capped at 255 decimal places (half-even).

**Arithmetic** (val-g11, val-g12, val-g13, val-m2):
- An inverse rate is `1/p` to **255 places**, half-even (`1/3` is 255 threes; `1/8` is exactly 0.125; `1e300 EUR` at `P X 3 EUR` gives 255 threes and zeros). It is not a rational: `3 EUR` →
  `0.999…9` (255 nines) → printed `1.0000000000` with **10 places**, whereas `0.9 A` over two ÷3 hops gives exactly `0.1` (255-place rounding carries, then normalisation).
- Only at output is a quantity rounded to 10 places, half-even (`5e-11` → 0, `1.5e-10` → `2e-10`, `2.5e-10` → `2e-10`); an exact 10-place result is not shortened, e.g. `1.0000000000`.
- Amounts that are the product of many digits stay exact up to 255 places: results above 2^53 mantissa are skipped by the harness like in raw mode.

**One posting with several amounts** (elided posting, `==` assignment): after conversion the amounts are **merged per commodity** — summed at full precision
(rounded once, at output: three `1/3` give `1.0000000000`, not `0.9999999999`, val-m2), sorted by commodity name (`""` first), and a zero sum is kept (`EUR 0 | Z -1`,
val-m1). The sum runs in posting-amount order, and adding to a zero amount gives the other amount unchanged, decimal places included:
`1.25 C, 3 C, -1.25 C` → `3` (0 places), `3 C, 1.25 C, -1.25 C` → `3.00` (val-z15, val-z22, val-z25, val-z28); the lot survives only if exactly one amount contributed and it was not converted (val-m3). The elided posting is computed before valuation and then valued like the others;
balance assertions and assignments run on the original amounts (val-m7).

**Balancing shortcut found on the way (raw `print`, not valuation)**: a priced posting that is preceded by an odd number of postings that are *all*
conversion accounts is balanced by quantity, not by cost, as soon as the transaction holds any adjacent conversion pair, whatever the amounts (hledger
then reports "unbalanced"). It fits all 604 enumerated shapes of {conversion, priced, plain} postings (one priced posting, up to 7 postings; the enumerator was ad hoc and is not kept, val-cv1..3 are samples), but I
found no reason for it; it smells like position bookkeeping that confuses equal-looking postings, the same family as the duplicate-amount gap above.
Another gap: a priced posting that is itself on a conversion account and directly follows a conversion pair is also balanced by quantity
(gaps/val-gap-priced-conversion-posting.journal).

**Not verified / assumed**: folding order of rate products for three or more hops (only matters at the 255th digit); self prices; behaviour of `--value` combined with `--cost`; `--value=now`, `end`, `then`, `DATE` without a commodity (our first user never passes them).


## Balance assertions and assignments

- `acc  AMOUNT = B` asserts acc's balance in B's commodity, excluding sub-accounts, after this posting. `=*` includes sub-accounts.
  `==` also requires every other commodity to be zero (m8). Exact comparison (`1.004` vs `1.00` fails; `1` vs `1.0` passes).
- Running balances are kept **per file** (each `-f` file has its own balances), in date order within the file (stable
  by parse order), including virtual postings and earlier postings of the same transaction. An assertion in one file does
  not see postings from a sibling file (m1a/m1b pass, m2a/m1b fails, y8a/y8b fails, x13/x13b assigns 20 not 19).
- `acc  = B` with no amount is an assignment: amount = B − running balance (decimalPlaces = max of the two). If the difference is zero, `pamount` is `[]` (q1413, q1514, q1206) — note our first user's decoder would reject that.
  The assigned amount takes part in balancing (v7: `b = -4 EUR` then elided `e` → -6).
- After an assignment the account balance is exactly the asserted amount (its written precision); with `=*` the
  account's own balance is the asserted amount minus its sub-accounts (ia*-incl).
- An assigned amount never carries a lot basis. A `==` assignment also emits amounts zeroing the account's other
  commodities; those keep the account's lot only while that commodity received exactly one amount (lb*-basis).
- `-I` skips assertion checks (assignments still apply). A failing assertion makes the whole command fail (exit 1).

## Errors

Parse errors, unbalanced groups, two elided postings in a group, failing assertions → no output, exit 1.
For the comparison, "both fail" is a match; the error message does not need to match.

## Large numbers

Mantissas above 2^53 are emitted by hledger (`9223372036854775808`, `1.8446744073709551616e19`), but our first user's decoder
requires safe integers, so they are out of scope.
