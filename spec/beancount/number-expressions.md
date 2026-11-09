# Arithmetic in numbers

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`;
`bean-format --version`: `Beancount 3.2.3`), 2026-11-09.

Status: **built**: `crates/ledgermill/src/beancount/expr.rs` reads and evaluates the expressions with `pydec.rs`
(`crates/ledgermill/tests/beancount_number_expressions.rs` checks every table of Output). Three shapes are
still refused, in "What ledgermill does with it" below.

## Rule

**Where.** Wherever a number can be written, an expression can: the units of a posting; the number of an `@`
or `@@` price; the per-unit part, the total part or both parts (`{a # b EUR}`) of a `{ }` cost and the number
of a `{{ }}` cost; the amount and the `~` tolerance of a `balance`; the amount of a `price` directive; a
metadata value, with or without a currency (`w: 1 + 2`, `w: (1 + 2) EUR`; a posting's own metadata too);
a `custom` value. Nothing is refused by the reference in these places
(`probes/number-expression-positions.beancount`, `-balance`, `-price-directive`, `-metadata`). An elided
posting next to them balances against the evaluated weights as usual (the last transaction of `-positions`).

**Grammar.** `+ - * /`, parentheses, and a sign in front of an operand.

- `*` and `/` bind tighter than `+` and `-`; every operator goes from the left: `2 - 3 - 4` is `-5`,
  `8 / 4 / 2` is `1`, `8 / 2 * 4` is `16`, `10 - 2 + 3` is `11`, `1 + 2 * 3` is `7`.
- A sign belongs to the operand after it and can be repeated and mixed: `-2 * 3`, `2 * -3`, `2 - -3` (`5`),
  `--4` (`4`), `+-6` (`-6`), `- 7` (a blank after the sign is fine), `-(1 + 2)`, `+(1 + 2)`. A plus sign is
  accepted.
- Blanks (spaces and tabs) may be put or left out between any two tokens: `(1+2)`, `( 1 + 2 )`, `(1 +2)`,
  `(2*-3)`, `(2 --3)`. A line break or a comment inside the expression ends the line there: that is a syntax
  error (below).
- **Parentheses are not needed.** The expression runs up to the currency: `Assets:A  1 + 2 EUR` is `3 EUR`,
  `1+2EUR` too, and so is `3 -1 EUR` (`2`). The same in a price (`@ 1 + 1 EUR`), a cost (`{1 + 1 EUR}`), a
  combined cost (`{1 + 1 # 1 + 2 EUR}`), a `price` directive and a tolerance (`~ 1 / 2`). What `bean-format`
  makes of such a line is in "bean-format" below.
- The operands are the numbers of numbers-and-interpolation.md: digit-group commas (`1,000`), a trailing dot
  (`5.`), the places as written. Not accepted: a number that starts with a dot (`.5`, also outside an
  expression), an exponent (`1E3`), names, and every other operator (`//`, `**`, `^`, `%`).

**Evaluation** is the decimal arithmetic of numbers-and-interpolation.md, step by step, left operand first, each
result rounded half to even to 28 significant digits:

- `a + b` and `a - b` keep the finer exponent of the two; `a * b` adds the exponents (`1.50 * 2` is `3.00`,
  `0.10 * 0.10` is `0.0100`, `1.5 * 2.25` is `3.375`).
- `a / b` asks for the exponent `e` of `a` minus the exponent of `b`. When the quotient is exact, the result
  has the largest exponent not past `e` at which the quotient is still a whole coefficient, that is the fewest
  digits that fit under `e`: `90.00 / 3` is `30.00` (`e` is -2), `100 / 8` is `12.5` (`e` is 0, one place is
  needed), `1.0 / 4` is `0.25`, `6 / 3.0` is `2` (`e` is 1 but 0.2 is not a coefficient), `50 / 0.5` is
  `1.0E+2` (`e` is 1 and 10 x 10^1 fits: a positive exponent shows), `5 / 0.25` is `2E+1` (`e` is 2, 0.2 is not
  whole, 2 x 10^1 is). When the quotient is not exact, the result has 28 significant digits, the last one
  rounded: `10 / 3`, `2 / 3`, `1 / 7` are `3.333333333333333333333333333`, `0.6666666666666666666666666667`,
  `0.1428571428571428571428571429`. The places of the divisor do not matter then (`1 / 3.00` is
  `0.3333333333333333333333333333`).
- It is the same rule as the per-unit price of a total price (`@@`, `{{ }}`) in numbers-and-interpolation.md,
  and `pydec.rs` already has it: `Num::add`, `Num::mul` and `Num::div` (`None` for a zero divisor),
  `Num::negated` for a sign. `rust_decimal` must not be used (its quotient has 28 decimal places).
- The result is printed as any number is: plain unless the exponent is positive or the number is smaller than
  `0.000001` (`1E-7`, `3.333333333333333333333333333E-7`, `1.0E+2`, `1.524157875323883675019051999E+38`).
  An operand of 29 digits is fine; the sum is rounded (`...678 + 0.5` is `...678`, `...679 + 0.5` is `...680`:
  half to even).
- The sign of a zero follows the decimal rules, as `-0.00` already did (a written `-0.00` is `0.00`): a
  product or quotient is negative when exactly one operand is (`0.00 * -1` is `-0.00`, `0 * -1` is `-0`,
  `0.00 / -5` is `-0.00`); a sign in front of a zero gives a plain zero (`-(0.00 * -1)` is `0.00`); a sum
  is a negative zero only when both its terms are (`-0.00 + -0.00`), and `a - b` is `a + (-b)`, so
  `-0.00 - 0.00` is `-0.00` and `0.00 - -0.00` is `0.00`;
  a zero reached by cancelling is plain and keeps its places (`-1.0 + 1.0` is `0.0`)
  (`probes/number-expression-zero.beancount`).

**Tolerance.** An expression is only a way to write a number: what counts afterwards is the value and its
exponent. Balancing.md applies unchanged, with the places of the *result*.

- `1 / 4` has two places, so a transaction with `(1 / 4) EUR` and `-0.254 EUR` balances (tolerance 0.005) and
  one with `-0.256 EUR` does not; `(1.50 * 2)` has two places (`3.00`) and `-3.004` balances; `(50 / 0.5)` has
  none (a positive exponent counts like an integer) and `-101` or `-100.04` do not balance.
- An inexact division has 28 significant digits (26 places for 33.33..., 27 for 3.33...), so its tolerance is
  minute: three postings of `(100 / 3)` against
  `-100` fail (the residual is `1E-26`, the tolerance `5E-27`), three of `(10 / 3)` against `-10` fail, and so
  do `1 / 3 + 1 / 3 + 1 / 3` and `1 / 3 * 3` against `-1` (residual `1E-28`). The same three postings against
  `-100.00` pass: the coarsest written amount decides, and `(100 / 3)` against `-33.33` passes too.
  `(2 / 3)` with `(1 / 3)` against `-1` passes: the sum is exact, `1.0000000000000000000000000000`.
- The elided posting of a transaction with expressions takes the sum of the written ones and rounds it to the
  fewest places among them, as in numbers-and-interpolation.md: two `(100 / 3)` leave
  `-66.66666666666666666666666666`; `(10 / 3)` with `1.5` leaves `-4.8`; `(1.5 * 2)` (`3.0`) with `1.25`
  leaves `-4.2` (4.25 rounded half to even to one place).

**Display.** `bean-query` prints a `number` with all its digits and the exponent forms above, unrounded
(`33.33333333333333333333333333`). A `position` (`sum(position)`, `position`) is shown rounded to the display
precision of the file, not by the arithmetic: `33.33 EUR` for the same amount in a file where the other amounts
have two places (`probes/number-expression-display.beancount`). ledgermill, like the harness, reads `number`.

**Errors.** Two kinds.

- *Malformed* expressions are syntax errors: `FILE:LINE: syntax error, ...` (an unexpected token) or
  `FILE:LINE: Invalid token: 'x'` (a lexical error: `^`, a name, `.5`, `1.5.2`; the token quoted can run on,
  `'.2)'`). Always on the line of the
  posting (or `balance`/`price` line) that holds the expression; for an expression broken over lines (or cut by a
  comment), the line **after** the break, the one that carries the rest (lines 154 and 163 below). **Only that entry is lost.** The transaction (with all its postings, also the good ones)
  is left out of the entries and does not count in any balance; everything before and after is read; exit
  code 1; one error per bad entry
  (`probes/number-expression-errors.beancount`: 20 bad entries, 20 errors, the 21 good transactions come
  out). The messages are the reference's own (listed below); the harness compares the verdict and the line only, not the text.
- *Division by zero* is not an error the reference reports; it breaks. `(1 / 0)`, `(0 / 0)`, `(1 / 0.00)`,
  `(1 / (2 - 2))`, `-(1 / 0)` and `2 * (1 / 0)` as the whole amount **crash** `bean-check` and `bean-query`
  (the shell says "Segmentation fault", exit status 139, nothing printed; the same in a `price` directive, a
  price, a `balance`; every run). When the zero quotient is the **left operand** of another operation
  (`(1 / 0) + 1`, `(1 / 0) * 0`, `(1 / 0) - 1`) the tool prints
  `FILE:LINE: SystemError: <class 'decimal.Decimal'> returned a result with an exception set` at the posting
  line, exit 1, and the entry is dropped like a syntax error
  (`probes/refused/number-expression-division-by-zero.beancount`, `-first-operand`). This is a defect of
  the reference in this environment, so neither is a rule: the probes are in `refused/` because the first
  one kills the tools.

**bean-format.** It never rewrites inside an expression and never evaluates one; it moves blanks and nothing
else, so the ledger is the same afterwards (format.md has the shapes; `probes/number-expression-format.beancount`
checks them on expressions). In summary:

- A *posting line* is aligned when its number is a number or a single operation in parentheses, `(a op b)`,
  with one of `+ - * /` between two plain numbers (a sign on a number is fine: `(2 * -3)`; one blank after
  `(` is fine, one before `)` is not): `(100000 * 3)` is as good as `12.50` for the column, and is the widest
  number of the probe, so it sets the width of the column for the whole file; `( 1+2)` is aligned and its
  inner text is kept. Every other expression is *not* aligned, kept as written and not counted in the
  column: two operations `(1 + 2 + 3)`, no parentheses `1 + 2`, a sign before the parentheses `-(1 + 2)`,
  a blank before the closing parenthesis `(1 + 2 )`. What follows the currency (`@ (10 / 4) EUR`,
  `{(10 / 4) EUR}`) is kept.
- A *`balance` or `price` line* is aligned on the last "number, blanks, currency" of the line. A parenthesised
  expression is that number (`(1 * 0) ~     (1 / 100) EUR`: the tolerance sits in the number column, the
  amount stays in the text before it). An expression without parentheses is cut in front of its last
  operand: `price BBB 1 + 1 EUR` becomes `price BBB 1` followed by blanks, then `+ 1 EUR` (the operand with its
  sign and the blank after it is the number), and `balance Assets:A  0 ~ 1 / 100 EUR` becomes `0 ~ 1 /`, blanks,
  `100 EUR`. Both still read as the same expression, since blanks are free inside one.
- `format()` treats an expression as the text it is, whatever its value (format.md). Today ledgermill
  returns every one of these probes unchanged because it refuses them, so its output differs from
  `bean-format`'s on all 14 (the harness lets a file ledgermill does not read differ, `designDifference`).
  Once expressions are read the text has to equal `bean-format`'s.

## What ledgermill does with it

The parser ledgermill is built on evaluates an expression as it reads it, with `rust_decimal`: a division
by zero or a product beyond 96 bits stops the program, and a quotient has 28 places, not 28 digits. So
no expression reaches it. Before it runs, `expr.rs` cuts each one out of its line and evaluates it with
`pydec.rs`; the parser sees a placeholder number (a decimal with 28 places, which no written number has: more
than 22 characters are refused) that the reader replaces by the value. What it reads:

- **Where**: the units of a posting, `@`, `@@`, both parts of a cost (`{}` and `{{ }}`), the amount and the tolerance of a
  `balance`, a `price`, a metadata value. A `custom` line is read, but its values are never evaluated (they
  change nothing). The cut-out expression is the longest run of digits, `.`, `+ - * /`, parentheses, blanks
  and a comma between two digits that starts where a number can: after a blank, a brace, `@`, `~` or a
  comma. It ends at the currency, `~`, `#`, a brace or anything else. A run with no operator (`-5`) is a number,
  and a sum or a difference without parentheses (`1 + 2 EUR`) is left to the parser, which adds safely.
- **Evaluation**: as "Evaluation" above says, left operand first, `Num::add`, `Num::sub`, `Num::mul`,
  `Num::div`. `Num` keeps the sign of a zero (`-0.00`): a product or a quotient is negative when exactly one
  operand is, a sum only when both terms are, a subtraction adds the flipped sign of the subtrahend, and
  the unary minus gives a plain zero. A posting printed with a negative zero shows it (`-0.00`).
- **Numbers kept whole**: a posting's units, a cost and a price keep the digits the arithmetic gave (the booking
  crate sees them rounded to 28 places, which only matters for matching lots). A result the crate cannot hold
  (more than 96 bits, such as `1.524157875323883675019051999E+38`) is refused as `number-too-precise`,
  like any number that needs more digits than are handled (`probes/refused/number-expression-result-too-large.beancount`).
  An operand is a number as written anywhere: more than 22 characters is `number-too-large`, so the
  28- and 29-digit operands of `probes/refused/number-expression-operands-too-large.beancount` are refused.
- **Syntax errors** are `parse-error`, on the line of the expression, or the line after when the expression
  stops where more was needed (a line break or a comment inside it: lines 154 and 163 of `-errors`). As for
  every parse error, ledgermill returns no transaction for the file (the reference loses only the entry,
  and the harness compares the first error). One error per line.
- **Division by zero**: a `division-by-zero` finding (error) on the line of the posting, the `balance` or the
  `price`; that entry is left out and everything else is read and checked (the reference crashes or reports an
  internal error; the probes stay in `refused/`).
- **Still refused**, as `unsupported-syntax`, construct `number-expression`: a sign in front of a sign
  without parentheses (`--4`, `+-6`) and a `~` tolerance that is a sum or a difference without parentheses
  (`~ 1 + 2`). The parser takes one sign and a plain tolerance there; the same inside parentheses is read
  (`probes/refused/number-expression-stacked-sign.beancount`, `-tolerance-sum`). Beyond that, a `( ) / *` left in a
  line after the expressions are cut out (a currency with a `/`) is refused the same way.
- **`format()`** returns every one of the format probe's lines as `bean-format` does
  (`crates/ledgermill/tests/format_beancount.rs`, `expressions_stay_as_written_and_align_as_bean_format_does`): the
  shapes in "bean-format" above, and the blanks after the operator (`(2 * - 3)` is one operation).

## Probe

`probes/number-expression-operators.beancount`, `probes/number-expression-division.beancount`,
`probes/number-expression-places.beancount`, `probes/number-expression-zero.beancount`,
`probes/number-expression-tolerance.beancount`, `probes/number-expression-tolerance-fails.beancount`,
`probes/number-expression-display.beancount`, `probes/number-expression-positions.beancount`,
`probes/number-expression-balance.beancount`, `probes/number-expression-price-directive.beancount`,
`probes/number-expression-metadata.beancount`, `probes/number-expression-errors.beancount`,
`probes/number-expression-format.beancount`, `probes/refused/number-expression-division-by-zero.beancount`,
`probes/refused/number-expression-division-by-zero-first-operand.beancount`,
`probes/refused/number-expression-operands-too-large.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/number-expression-operators.beancount
# and the same for -division, -places, -zero, -tolerance, -tolerance-fails, -positions, -errors
spec/beancount/show-prices.sh spec/beancount/probes/number-expression-price-directive.beancount
spec/beancount/show-format.sh spec/beancount/probes/number-expression-format.beancount
bean-check spec/beancount/probes/number-expression-balance.beancount
bean-query -f csv spec/beancount/probes/number-expression-display.beancount "SELECT account, number, sum(position) GROUP BY account, number"
bean-query -f csv spec/beancount/probes/number-expression-metadata.beancount \
  "SELECT meta['w1'], meta['w2'], meta['w3'], meta['w4'], meta['w5'] FROM entries WHERE type = 'transaction'"
```

The tables below are `show.sh`'s postings reduced to the line of the posting, the text written before the
currency, and `number`. Where a transaction has an elided posting its row is the negated sum (stated in
the text). `bean-check:` is what it printed.

## Output

number-expression-operators:

```text
bean-check: (no output, exit 0)
line  written         number
5     (7 + 2)         9
6     (7 - 2)         5
7     (7 * 2)         14
8     (7 / 2)         3.5
12    (1 + 2 * 3)     7
16    (2 - 3 - 4)     -5
20    (8 / 4 / 2)     1
24    (8 / 2 * 4)     16
28    (10 - 2 + 3)    11
32    (10 / 2 + 3)    8
36    ((1 + 2) * 3)   9
37    (2 * (3 + 4))   14
38    (((5)))         5
42    (-2 * 3)        -6
43    (2 * -3)        -6
44    (2 - -3)        5
45    (--4)           4
46    (+5)            5
47    (+-6)           -6
48    (- 7)           -7
52    -(1 + 2)        -3
53    +(1 + 2)        3
54    - (4 - 5)       1
58    (1+2)           3
59    ( 1 + 2 )       3
60    (1 +2)          3
61    (1+ 2)          3
62    (2*-3)          -6
63    (2 --3)         5
64    (1\t+\t2)       3
68    1 + 2           3
69    1+2             3
70    2 * 3           6
71    3 -1            2
72    -1 + 3          2
73    1 + 2 * 3       7
74    (1 + 2) * 3     9
75    (1 + 2)         3
76    1+2             3
80    (1,000 + 1)     1001
81    (1,000.50 * 2)  2001.00
82    (5. + 1)        6
```

number-expression-division (the elided posting of each transaction is the negated number):

```text
bean-check: (no output, exit 0)
line  written                   number
5     (10 / 3)                  3.333333333333333333333333333
9     (2 / 3)                   0.6666666666666666666666666667
13    (1 / 7)                   0.1428571428571428571428571429
17    (100 / 3.0)               33.33333333333333333333333333
21    (1 / 3.00)                0.3333333333333333333333333333
25    (1 / 3 / 1000000)         3.333333333333333333333333333E-7
29    (90.00 / 3)               30.00
33    (6.0 / 3)                 2.0
37    (6 / 3.0)                 2
41    (100 / 8)                 12.5
45    (1.0 / 4)                 0.25
49    (1 / 8)                   0.125
53    (1 / 1000)                0.001
57    (7 / 2)                   3.5
61    (15 / 6)                  2.5
65    (3.00 / 6)                0.50
69    (0.5 / 5)                 0.1
73    (1 / 400000)              0.0000025
77    (1 / 10000000)            1E-7
81    (1.00 / 100000000)        1E-8
85    (1 / 3 / 1000000 / 1000)  3.333333333333333333333333333E-10
89    (50 / 0.5)                1.0E+2
93    (500 / 0.5)               1.00E+3
97    (500 / 0.05)              1.00E+4
101   (5 / 0.25)                2E+1
105   (2 / 0.5)                 4
109   (1 / 0.5)                 2
113   (2 / 3 + 1000000)         1000000.666666666666666666667
117   (1 / 3 * 3)               0.9999999999999999999999999999
121   (10 / 4 * 4)              10.0
```

number-expression-places (every elided posting is the negated number, except the last three transactions: -4.8, -4.5 and -4.2):

```text
bean-check: (no output, exit 0)
line  written                                          number
6     (1.50 * 2)                                       3.00
10    (1.5 * 2.25)                                     3.375
14    (2.0 * 3)                                        6.0
18    (0.10 * 0.10)                                    0.0100
22    (0.001 * 0.001 * 0.001)                          1E-9
26    (1000000.0 * 1000000)                            1000000000000.0
30    (100 * 1000000)                                  100000000
34    (0.1 * 3)                                        0.3
38    (0.1 + 0.20)                                     0.30
42    (1.00 + 2)                                       3.00
46    (1 + 2.00)                                       3.00
50    (1.10 - 0.1)                                     1.00
54    (2.50 - 2.5)                                     0.00
58    (1,000.50 * 2)                                   2001.00
62    (0.1234567890123456789 * 0.1234567890123456789)  0.01524157875323883675019051999
66    (100000000000000000000 * 100)                    10000000000000000000000
70    (10 / 3)                                         3.333333333333333333333333333
71    1.5                                              1.5
75    (10 / 4)                                         2.5
76    (4 / 2)                                          2
80    (1.5 * 2)                                        3.0
81    1.25                                             1.25
```

number-expression-zero:

```text
bean-check: (no output, exit 0)
line  written                      number
4     (0.00 * -1)                  -0.00
7     (-(0.00 * -1))               0.00
10    ((0.00 * -1) * -1)           0.00
13    ((0.00 * -1) * 1)            -0.00
16    (0.00 / -5)                  -0.00
19    ((0.00 * -1) / 5)            -0.00
22    ((0.00 * -1) + (0.00 * -1))  -0.00
25    ((0.00 * -1) + 0.00)         0.00
28    ((0.00 * -1) - 0.00)         -0.00
31    (0.00 - (0.00 * -1))         0.00
34    (-0.00)                      0.00
37    (-0.00 * 1)                  0.00
40    (0 * -1)                     -0
43    (0.0 - 0.0)                  0.0
46    (-1.0 + 1.0)                 0.0
49    (0 / 5)                      0
52    (0.00 / 5)                   0.00
55    (0.00 * -0.00)               0.0000
```

number-expression-tolerance:

```text
bean-check: (no output, exit 0)
```

number-expression-tolerance-fails (the transactions are still listed by bean-query):

```text
bean-check (exit 1):
  4: Transaction does not balance: (-0.00000000000000000000000001 EUR)   <- "three shares of 100 against an integer"
  10: Transaction does not balance: (-0.000000000000000000000000001 EUR)   <- "three shares of 10 against an integer"
  16: Transaction does not balance: (-0.0000000000000000000000000001 EUR)   <- "three thirds against an integer"
  20: Transaction does not balance: (-0.0000000000000000000000000001 EUR)   <- "a third times three"
  24: Transaction does not balance: (-0.006 EUR)   <- "a quarter 0.006 away"
  28: Transaction does not balance: (0.01333333333333333333333333 EUR)   <- "a share of 100 is 0.0033 away from a rounded share"
  32: Transaction does not balance: (-0.006 EUR)   <- "a product 0.006 away"
  36: Transaction does not balance: (-1 EUR)   <- "a positive exponent against an integer allows nothing"
  40: Transaction does not balance: (-0.04 EUR)   <- "a positive exponent against two places"
```

number-expression-positions:

```text
bean-check: (no output, exit 0)
63c9af,Assets:A,,2,AAA,,,,,2.5,EUR,6
63c9af,Assets:B,,-5,EUR,,,,,,,7
9dd402,Assets:A,,4,AAA,,,,,1.25,EUR,10
9dd402,Assets:B,,-5,EUR,,,,,,,11
e13b78,Assets:A,,2,FFF,,,,,2,EUR,14
e13b78,Assets:B,,-4,EUR,,,,,,,15
45fe33,Assets:A,,2,GGG,,,,,2.5,EUR,18
45fe33,Assets:B,,-5,EUR,,,,,,,19
885fb6,Assets:Stock,,2,BBB,2.5,EUR,2026-01-06,,,,22
885fb6,Assets:B,,-5,EUR,,,,,,,23
2d5351,Assets:Stock,,4,CCC,1.25,EUR,2026-01-07,,,,26
2d5351,Assets:B,,-5,EUR,,,,,,,27
d39efe,Assets:Stock,,4,DDD,2.75,EUR,2026-01-08,,,,30
d39efe,Assets:B,,-11,EUR,,,,,,,31
0ac23e,Assets:Stock,,4,EEE,2.75,EUR,2026-01-09,,,,34
0ac23e,Assets:B,,-11,EUR,,,,,,,35
6a5dfa,Assets:Stock,,2,HHH,2,EUR,2026-01-10,,,,38
6a5dfa,Assets:B,,-4,EUR,,,,,,,39
e7eb1b,Assets:Stock,,2,JJJ,2,EUR,2026-01-10,lot,,,42
e7eb1b,Assets:B,,-4,EUR,,,,,,,43
dcb023,Assets:Stock,,2,KKK,1.5,EUR,2026-01-12,,,,46
dcb023,Assets:B,,-3.0,EUR,,,,,,,47
```

number-expression-balance (the assertions on lines 26 to 35 that are not listed pass: A, C, D, F, H, I):

```text
bean-check (exit 1):
  27: Balance failed for 'Assets:B': expected 500 EUR != accumulated 500.01 EUR (0.01 too much)   <- Assets:B  (1000 / 2) EUR
  30: Balance failed for 'Assets:E': expected 500 EUR != accumulated 500.01 EUR (0.01 too much)   <- Assets:E  500 ~ (1 / 1000) EUR
  32: Balance failed for 'Assets:G': expected 333.3333333333333333333333333 EUR != accumulated 333.33 EUR (0.0033333333333333333333333 too little)   <- Assets:G  (1000 / 3) EUR
  35: Balance failed for 'Assets:J': expected 500.0 EUR != accumulated 500.01 EUR (0.01 too much)   <- Assets:J  (500 * 1.0) ~ -(1) EUR
```

number-expression-price-directive:

```text
bean-check: (no output, exit 0)
2026-01-02,AAA,2.5,EUR
2026-01-03,BBB,2,EUR
2026-01-04,AAA,3.333333333333333333333333333,EUR
2026-01-05,BBB,3.00,EUR
2026-01-06,AAA,0.125,EUR
2026-01-07,BBB,-2,EUR
2026-01-08,AAA,1E-7,EUR
2026-01-09,BBB,1.0E+2,EUR
```

number-expression-metadata:

```text
bean-check: (no output, exit 0)
meta['w1'],meta['w2'],meta['w3'],meta['w4'],meta['w5']
3,3,0.3333333333333333333333333333,3 EUR,6 EUR
meta['p1']
0.5
```

number-expression-display:

```text
bean-check: (no output, exit 0)
account,number,sum(position)
Assets:A,33.33333333333333333333333333,33.33 EUR
Assets:B,-33.33,-33.33 EUR
Assets:A,1.25,1.25 EUR
Assets:B,-1.25,-1.25 EUR
```

number-expression-errors (bean-query lists 21 transactions, all 'good'):

```text
bean-check (exit 1):
  9: syntax error, unexpected RPAREN, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  (1 + ) EUR
  17: syntax error, unexpected CURRENCY   <- Assets:A  (1 + 2 EUR
  25: syntax error, unexpected RPAREN, expecting end of file or EOL or ATAT or AT   <- Assets:A  1 + 2) EUR
  33: syntax error, unexpected SLASH, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  (7 // 2) EUR
  41: syntax error, unexpected ASTERISK, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  (2 ** 3) EUR
  49: Invalid token: '^'   <- Assets:A  (2 ^ 3) EUR
  57: syntax error, unexpected FLAG   <- Assets:A  (7 % 2) EUR
  65: Invalid token: 'a'   <- Assets:A  (a + 1) EUR
  73: syntax error, unexpected RPAREN, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  () EUR
  81: syntax error, unexpected NUMBER   <- Assets:A  (1 2) EUR
  89: syntax error, unexpected LPAREN, expecting end of file or EOL or ATAT or AT   <- Assets:A  (1 + 2) (3) EUR
  97: Invalid token: '.2)'   <- Assets:A  (1.5.2) EUR
  105: Invalid token: '.5'   <- Assets:A  (.5 + 1) EUR
  113: syntax error, unexpected CURRENCY   <- Assets:A  (1E3) EUR
  121: syntax error, unexpected ASTERISK, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  (* 2) EUR
  129: syntax error, unexpected ASTERISK, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  (1 + * 2) EUR
  137: syntax error, unexpected RPAREN, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  (2 * 3 -) EUR
  145: syntax error, unexpected CURRENCY, expecting PLUS or MINUS or LPAREN or NUMBER   <- Assets:A  1 / EUR
  154: syntax error, unexpected EOL, expecting PLUS or MINUS or LPAREN or NUMBER   <- 2) EUR
  163: syntax error, unexpected EOL, expecting PLUS or MINUS or LPAREN or NUMBER   <- 2) EUR
```

refused/number-expression-operands-too-large (bean-check: no output, exit 0; operands of 28 and 29 digits, lines 5, 9, 13 and 17):

```text
line  written                                         number
5     (1234567890123456789012345678 + 0.5)            1234567890123456789012345678
9     (1234567890123456789012345679 + 0.5)            1234567890123456789012345680
13    (1234567890123456789012345678 + 0.51)           1234567890123456789012345679
17    (12345678901234567890123456789 + 1)             1.234567890123456789012345679E+28
```

refused/number-expression-division-by-zero and refused/number-expression-division-by-zero-first-operand:

```text
bean-check number-expression-division-by-zero.beancount: no output, killed by SIGSEGV (shell status 139)
bean-query on the same file: the same
bean-check number-expression-division-by-zero-first-operand.beancount (exit 1):
  9: SystemError: <class 'decimal.Decimal'> returned a result with an exception set
bean-query lists the transactions of lines 4 and 12 ("good: before", "good: after")
```

number-expression-format (`show-format.sh`: the lines bean-format changed, as it wrote them):

```text
    |0         1         2         3         4         5         6
    |0123456789012345678901234567890123456789012345678901234567890123
   6|  Assets:A                                     12.50 EUR$
   7|  Assets:B                                    -12.50 EUR$
  10|  Assets:A                              (100000 * 3) EUR$
  11|  Assets:B                                   -300000 EUR$
  14|  Assets:A                                    ( 1+2) EUR$
  15|  Assets:B                                        -3 EUR$
  19|  Assets:B                                        -6 EUR$
  23|  Assets:B                                        -3 EUR$
  27|  Assets:B                                         3 EUR$
  31|  Assets:B                                        -3 EUR$
  34|  Assets:A                                   (1 * 2) AAA @ (10 / 4) EUR$
  35|  Assets:Stock                               (1 * 2) BBB {(10 / 4) EUR}$
  36|  Assets:B                                       -10 EUR$
  39|  Assets:A                                  (2 * -3) EUR$
  40|  Assets:B                                         6 EUR$
  42|2026-02-10 balance Assets:A                  (1 * 0) EUR$
  43|2026-02-10 balance Assets:A  (1 * 0) ~     (1 / 100) EUR$
  44|2026-02-10 balance Assets:A  0 ~ 1 /             100 EUR$
  45|2026-02-11 price AAA                        (10 / 4) EUR$
  46|2026-02-11 price BBB 1                           + 1 EUR$
```

