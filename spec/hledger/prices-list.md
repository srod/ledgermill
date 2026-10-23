# `hledger prices`: the P lines, in order, with their exact rates

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-23.

Status: built as `Ledger::prices()`; the harness compares it with the second run described above.

`prices()` answers what two runs of hledger list:

1. `hledger -f F1 -f F2 ... prices --color=n`;
2. the same with `-c "1.000000000000000000 COMM"` added for every commodity in the rate column of run 1.

Each row of run 2 is `P DATE FROM RATE TO`. The engine returns, for each row, the date, the commodity
`FROM`, the rate (commodity `TO` and an exact decimal quantity), and the file and line of the `P` directive.
This note says which `P` directives are listed, in which order, what the quantity really is, and what
hledger does not give (the file and the line, which are ours to define).

The rows below are made by `spec/hledger/show-prices.sh`, which does the two runs, checks that run 2 lists the
same rows as run 1 (same count, same dates, same `FROM`), and prints run 2; `--first` prints run 1. The
directory `spec/hledger/probes/` is left out of the paths.

## Rule: what is listed

Only the `P` directives of the files given. Nothing is added from the transactions: not the price of an
`@` or `@@` amount, not a price hledger could infer from two commodities that balance. A `P` dated in the
future is listed. A journal with no `P` line gives no row and succeeds.

Two flags add rows (not built, not needed): `--infer-market-prices` adds a row per `@`/`@@` price (and per
inferred price) of the transactions, dated on the transaction, and `--show-reverse` adds the reverse of every
price (`P d EUR 0.5 ZZZ` next to `P d ZZZ 2 EUR`).

`prices` checks the journal like `print`: a `P` line that does not parse, an unbalanced transaction, a failed
balance assertion (`-I` would skip that one) or a missing file is an error, and nothing is listed. The `P`
line errors are those of [directives-other](directives-other.md); the ones met here are in the last section.

### Probe

`probes/prices-not-from-costs.journal`, `probes/prices-empty.journal`

### Command and output

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-not-from-costs.journal
P 2026-01-05 ZZZ 2 EUR
P 2099-01-01 FUT 2 EUR
$ hledger --no-conf -f spec/hledger/probes/prices-not-from-costs.journal prices --color=n --infer-market-prices
P 2026-01-05 ZZZ 2 EUR
P 2026-01-10 USD 0.9 EUR
P 2026-01-11 GBP 1.2 EUR
P 2026-01-12 AAA 0.9 BBB
P 2099-01-01 FUT 2 EUR
$ hledger --no-conf -f spec/hledger/probes/prices-not-from-costs.journal prices --color=n --show-reverse
P 2026-01-05 ZZZ 2 EUR
P 2026-01-05 EUR 0.5 ZZZ
P 2099-01-01 FUT 2 EUR
P 2099-01-01 EUR 0.5 FUT
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-empty.journal
$
```

## Rule: order

The rows are sorted by date, and nothing else: rows with the same date keep the order in which hledger read
them, which is the files in the order given (`-f a -f b`: all of `a`, then all of `b`) and, within a file,
top to bottom. Where a `P` stands relative to the transactions does not matter. Two rows with the same date
and the same pair stay as they are written: a repeated price is kept (even an identical one), the last one
written is the last listed, and the rate is not compared. A time after the date is ignored by the sort (see
dates, below).

### Probe

`probes/prices-order-a.journal`, `probes/prices-order-b.journal`

### Command and output

order-a alone: the lines are written 03-01, 01-01, a transaction, then six lines dated 02-01, the
last three of them USD to EUR at `0.5`, `0.5` and `0.50`:

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-order-a.journal
P 2026-01-01 USD 0.8 EUR
P 2026-02-01 EUR 1.1 USD
P 2026-02-01 EUR 1.2 USD
P 2026-02-01 EUR 1.15 USD
P 2026-02-01 AAA 1 EUR
P 2026-02-01 USD 0.5 EUR
P 2026-02-01 USD 0.5 EUR
P 2026-02-01 USD 0.50 EUR
P 2026-03-01 USD 0.9 EUR
```

Two files, both with `P` lines dated 2026-02-01 (b has `GBP 7` then `USD 0.6` on that date, and 01-15, 04-01):

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-order-a.journal spec/hledger/probes/prices-order-b.journal
P 2026-01-01 USD 0.8 EUR
P 2026-01-15 USD 0.85 EUR
P 2026-02-01 EUR 1.1 USD
P 2026-02-01 EUR 1.2 USD
P 2026-02-01 EUR 1.15 USD
P 2026-02-01 AAA 1 EUR
P 2026-02-01 USD 0.5 EUR
P 2026-02-01 USD 0.5 EUR
P 2026-02-01 USD 0.50 EUR
P 2026-02-01 GBP 7 EUR
P 2026-02-01 USD 0.6 EUR
P 2026-03-01 USD 0.9 EUR
P 2026-04-01 GBP 1.2 EUR
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-order-b.journal spec/hledger/probes/prices-order-a.journal
P 2026-01-01 USD 0.8 EUR
P 2026-01-15 USD 0.85 EUR
P 2026-02-01 GBP 7 EUR
P 2026-02-01 USD 0.6 EUR
P 2026-02-01 EUR 1.1 USD
P 2026-02-01 EUR 1.2 USD
P 2026-02-01 EUR 1.15 USD
P 2026-02-01 AAA 1 EUR
P 2026-02-01 USD 0.5 EUR
P 2026-02-01 USD 0.5 EUR
P 2026-02-01 USD 0.50 EUR
P 2026-03-01 USD 0.9 EUR
P 2026-04-01 GBP 1.2 EUR
```

## Rule: the rate is what the digits mean under the number marks in force, and the printed text can mislead

The rate is an amount written after the first commodity, read like any other amount
([numbers](numbers.md), [directives-numbers](directives-numbers.md)) **at that line**. So the marks come from
what was declared **before the `P` line**, in the same file:

- `decimal-mark` decides for every commodity;
- else a `commodity` directive **for the rate's commodity** (the one written after the digits; a directive
  for the commodity written first, `FROM`, does not touch the rate), as a sample amount or as an indented
  `format` line;
- else `D`, for every commodity;
- else a lone `.` or `,` is the decimal mark.

With `commodity 1.000,00 EUR`, `P 2026-07-31 USD 0.8 EUR` has the rate **8**: the point is a digit group. The
rate is not 0.8 shown oddly: valuing one USD at the last price on or before the date gives 9333 EUR for
`USD 0.9333 EUR` (second probe below). A directive written **after** the `P` line, or in another file, changes
nothing for it.

Run 1 shows the rate in a display style (the commodity's marks, digit groups, side), so its text cannot be
read back as a number: `9333` is shown as `9.333`, `0.9333` as `0,9333`. Run 2's `-c` replaces the style by
`1.000000000000000000 COMM` (point, no groups, no more than the places the rate had) and shows the digits
the parse gave: `8`, `9333`, `0.9333`, `1234.5`, `1234`. That is why the style must be reset for each commodity;
the engine has no style to reset and returns the quantity.

The display style of a commodity in run 1 is also taken from the `P` amounts themselves (all of them, so a later
odd one can change how earlier rows print: `probes/prices-rate-style-last.journal`). Amounts in transactions play
no part (`probes/prices-rate-style-txn.journal`).

### Probe

`probes/prices-rate-commodity-format.journal`, `probes/prices-rate-commodity-value.journal`, `probes/prices-rate-commodity-after.journal`, `probes/prices-rate-commodity-from.journal`, `probes/prices-rate-commodity-block.journal`, `probes/prices-rate-decimal-mark-comma.journal`, `probes/prices-rate-decimal-mark-point.journal`, `probes/prices-rate-scope-a.journal` + `probes/prices-rate-scope-b.journal`, `probes/prices-rate-style-last.journal`, `probes/prices-rate-style-txn.journal`

### Command and output

`commodity 1.000,00 EUR` on line 1, then `0.8`, `0.9333`, `0,9333`, `1.234,5`, `1.234` EUR, `EUR 1.1 USD` and `5 EUR`:

```text
$ spec/hledger/show-prices.sh --first spec/hledger/probes/prices-rate-commodity-format.journal
P 2026-07-31 USD 8 EUR
P 2026-07-31 USD 9.333 EUR
P 2026-07-31 USD 0,9333 EUR
P 2026-07-31 USD 1.234,5 EUR
P 2026-07-31 USD 1.234 EUR
P 2026-07-31 EUR 1.1 USD
P 2026-07-31 USD 5 EUR
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-commodity-format.journal
P 2026-07-31 USD 8 EUR
P 2026-07-31 USD 9333 EUR
P 2026-07-31 USD 0.9333 EUR
P 2026-07-31 USD 1234.5 EUR
P 2026-07-31 USD 1234 EUR
P 2026-07-31 EUR 1.1 USD
P 2026-07-31 USD 5 EUR
```

The value of one USD on 2026-08-01 (prices `0.8` on 07-30 and `0.9333` on 07-31, same directive):

```text
$ spec/hledger/show-amounts.sh --value=2026-08-01,EUR spec/hledger/probes/prices-rate-commodity-value.journal
2026-08-01  assets:cash: EUR 9333 | equity:open: EUR -9333
```

An indented `format` line does the same (`commodity EUR` / `  format 1.000,00 EUR`: `0.8` is `8`). The directive after the
first of two lines changes only the second (run 2):

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-commodity-after.journal
P 2026-01-01 USD 0.8 EUR
P 2026-01-02 USD 8 EUR
```

A directive for the commodity written first (`commodity 1.000,00 USD`): `P USD 0.8 EUR` is `0.8`, `P EUR 0.8 USD` is
`8` (its rate is in USD), and `P USD 1.000 EUR` is `1.000` (EUR is undeclared: a lone mark is the decimal mark):

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-commodity-from.journal
P 2026-01-01 USD 0.8 EUR
P 2026-01-02 EUR 8 USD
P 2026-01-03 USD 1.000 EUR
```

`decimal-mark ,` (lines `0.8`, `0,8`, `1.234`, `1.234,5`) and `decimal-mark .` (lines `0,8`, `1,234`, `1,234.5`):

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-decimal-mark-comma.journal
P 2026-01-01 USD 8 EUR
P 2026-01-02 USD 0.8 EUR
P 2026-01-03 USD 1234 EUR
P 2026-01-04 USD 1234.5 EUR
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-decimal-mark-point.journal
P 2026-01-01 USD 8 EUR
P 2026-01-02 USD 1234 EUR
P 2026-01-03 USD 1234.5 EUR
```

`decimal-mark ,` in file a (rows `0,5` and `1.234`), nothing in file b (`0.7`, `1,234`): a's directive does not reach b:

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-scope-a.journal spec/hledger/probes/prices-rate-scope-b.journal
P 2026-01-01 USD 0.5 EUR
P 2026-01-02 USD 1234 EUR
P 2026-01-03 USD 0.7 EUR
P 2026-01-04 USD 1.234 EUR
```

Run 1 on the style probes. Rows are `0.8`, `1.50E1`, `1.2.3` EUR: the last amount's groups style the others
(`0,8` and `1.5,0` are the same numbers as run 2's `0.8` and `15.0`; `1.2.3` is `123`):

```text
$ spec/hledger/show-prices.sh --first spec/hledger/probes/prices-rate-style-last.journal
P 2026-01-01 USD 0,8 EUR
P 2026-01-02 USD 1.5,0 EUR
P 2026-01-03 USD 1.2.3 EUR
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-style-last.journal
P 2026-01-01 USD 0.8 EUR
P 2026-01-02 USD 15.0 EUR
P 2026-01-03 USD 123 EUR
```

## Rule: the places and the form of the quantity

The quantity keeps the places it was written with, whatever the declared format has: `0.8` stays one place,
`0.80` two (`commodity 1,000.00 EUR` does not pad `7 EUR` and does not round `0.123456`; `-c` with two or eighteen
places neither). So `0.8` and `0.80` are two different strings with the same value; the engine keeps both as
written. The number grammar is that of [numbers](numbers.md): a leading `+` or `-`, `1.` (is `1`), `.5`, an
exponent (`E` or `e`, signed), digit groups of `,`, `.` or a space. Seen here:

- exponent: `1e-3` is `0.001`, `1.5E2` is `150`, `1.50E1` is `15.0` (the places are the written ones minus the
  exponent, not below zero), `2.5e+3` is `2500`, `1E28` is `1` and twenty-eight zeros, `1E-30` is
  `0.` and twenty-nine zeros, `1`; under a decimal comma `1,5E2` is `150`;
- groups: `1,234.56` and `1 234.5` are `1234.56` and `1234.5`; `1.2.3,4` is `123.4`; `12,34.5` is `1234.5`
  (groups need not be of three);
- sign and zero: `-0.5` is `-0.5`, `+0.5` is `0.5`, `0` is `0`, `0.00` stays `0.00`, `-0` is `0`, `-0.00` is
  `0.00` (negative zero is not kept);
- a rate of a commodity in itself is listed as is (`P d EUR 1 EUR`); a negative or zero rate is not an error;
- size: 20 integer digits, 24 or 31 places and a 60-digit integer part with 30 places are all kept exactly.

A lone mark without a declaration is a decimal mark: `1,5` and `1.5` are `1.5`, and `1,234` and `1.234` are
`1.234`.

### Probe

`probes/prices-rate-forms.journal`

### Command and output

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-forms.journal
P 2026-01-01 USD 0.8 EUR
P 2026-01-02 USD 0.80 EUR
P 2026-01-03 USD 0.123456789012345678901234 EUR
P 2026-01-04 USD 0.001 EUR
P 2026-01-05 USD 150 EUR
P 2026-01-06 USD 1234.56 EUR
P 2026-01-07 USD 1234.5 EUR
P 2026-01-08 USD -0.5 EUR
P 2026-01-09 USD 0 EUR
P 2026-01-10 USD 0.00 EUR
P 2026-01-11 USD 12345678901234567890.12345 EUR
P 2026-01-12 EUR 1 EUR
P 2026-01-13 USD 1 EUR
P 2026-01-14 USD 0.5 EUR
P 2026-01-15 USD 1.5 EUR
P 2026-01-16 USD 1.234 EUR
P 2026-01-17 USD 1.234 EUR
P 2026-01-18 USD 0.5 EUR
P 2026-01-19 USD 0.3333333333333333333333333333333 EUR
P 2026-01-20 USD 15.0 EUR
P 2026-01-21 USD 0.000000000000000000000000000001 EUR
P 2026-01-22 USD 2500 EUR
P 2026-01-23 USD 10000000000000000000000000000 EUR
P 2026-01-24 USD 0 EUR
P 2026-01-25 USD 0.00 EUR
P 2026-01-26 USD 150 EUR
P 2026-01-27 USD 123.4 EUR
P 2026-01-28 USD 1234.5 EUR
P 2026-01-29 USD 123456789012345678901234567890123456789012345678901234567890.123456789012345678901234567890 EUR
```

## Rule: a rate with no commodity

`P d USD 3` is accepted (also `P d USD 3 ""`). The rate's commodity is then empty, the number is read by the
same marks, and run 2 needs `-c "1.000000000000000000"` (no commodity) to show digits.

With a `D` directive in force, the rate gets D's commodity and the **display pads it to D's places**: the
digits are read by D's marks (`0,8` is `0.8`; `0.8` is `8`) and a rate written with fewer places than D's
sample is shown with D's (`7` under `D 1,000.00 EUR` is `7.00`; under `D 1,000.000 EUR`, `1.5` is `1.500`, while
`1.2345` is kept). A rate that names its commodity is not padded, nor is one under `commodity` (`7 EUR` stays `7`).
This is the one case where run 2 shows places that were not written. Whether the quantity itself has the padded
places cannot be seen (valuation through it gives the same digits either way), so the engine should
reproduce the padded text, as run 2 prints it, and the question stays open in Gaps.

### Probe

`probes/prices-rate-bare.journal`, `probes/prices-rate-default-commodity.journal`

### Command and output

Bare rates `0.8`, `1,5`, `1,234`, `1.234,5`, `1 234.5`, `3`, `3 ""` (run 2 is made with the bare `-c`):

```text
$ spec/hledger/show-prices.sh --first spec/hledger/probes/prices-rate-bare.journal
P 2026-01-03 USD 0,8
P 2026-01-04 USD 1,5
P 2026-01-05 USD 1,234
P 2026-01-06 USD 1.234,5
P 2026-01-07 USD 1.234,5
P 2026-01-08 USD 3
P 2026-01-09 USD 3
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-bare.journal
P 2026-01-03 USD 0.8
P 2026-01-04 USD 1.5
P 2026-01-05 USD 1.234
P 2026-01-06 USD 1234.5
P 2026-01-07 USD 1234.5
P 2026-01-08 USD 3
P 2026-01-09 USD 3
```

`D 1.000,00 EUR`, then `0.8 EUR`, `0,8 EUR`, `0,8 GBP`, and the bare `1.234,5`:

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-rate-default-commodity.journal
P 2026-01-01 USD 8 EUR
P 2026-01-02 USD 0.8 EUR
P 2026-01-03 USD 0.8 GBP
P 2026-01-04 USD 1234.50 EUR
```

## Rule: commodity names

Both commodities follow the symbol rules of [amount-forms](amount-forms.md): letters and symbols (`$`, `€`, `_`),
or a quoted name that holds anything but `"`. An unquoted name cannot hold a digit (`A1` is an error at the
digit); a name ends at the first character that is not allowed (`AB-C` is `AB`). The name is the same
quoted or not (`"EUR"` is `EUR`). The empty name `""` is accepted for `FROM` (the row reads `P 2026-01-17  3 EUR`:
two spaces) and for the rate (previous section).

The rate may be written with its commodity before or after the number, with or without a space, and a sign
before the commodity or after it: `5 EUR`, `5EUR`, `EUR5`, `EUR 5`, `$1.25`, `$ 1.5`, `-$3`, `$-3`, `EUR - 3`. The row
shows `RATE COMM` (run 2 style, suffix, one space) or, in run 1, the side and spacing the commodity was first
seen with: this is a display style and carries no data (`probes/prices-names-prefix-side.journal`: run 1 shows
`USD EUR 3`, `USD EUR 4`, `GBP EUR 5`; run 2 `3 EUR`, `4 EUR`, `5 EUR`).

hledger quotes a name in its output when it holds a space or a digit (`"ACME INC"`, `"A1"`, `"1x"`, `"a b"`, not `EUR`,
`€`, `$`, `C_D`); the quotes are not part of the name. The engine returns names without quotes, as
`print -O json` does (`["ACME INC", 1, 0]`), and the quoting is ours to redo when a text is wanted.

### Probe

`probes/prices-names.journal`, `probes/prices-names-prefix-side.journal`

### Command and output

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-names.journal
P 2026-01-01 $ 0.9 €
P 2026-01-02 € 1.1 $
P 2026-01-03 EUR 1.25 $
P 2026-01-04 EUR 1.5 $
P 2026-01-05 "ACME INC" 12.5 "Big Bank Ltd"
P 2026-01-06 "ACME INC" 12.5 USD
P 2026-01-07 ACME 3 €
P 2026-01-08 ACME -3 $
P 2026-01-09 ACME -3 $
P 2026-01-10 "A1" 3 "B2"
P 2026-01-11 "1x" 3 EUR
P 2026-01-12 VTI 3 EUR
P 2026-01-13 VTI 3 EUR
P 2026-01-14 EUR 3 EUR
P 2026-01-15 USD 5 €
P 2026-01-16 USD 5 C_D
P 2026-01-17  3 EUR
P 2026-01-18 USD 3 EUR
P 2026-01-19 USD -3 EUR
P 2026-01-20 USD -3 EUR
P 2026-01-21 USD 3 "a b"
P 2026-01-22 USD 3 a
```

(the probe's lines in order: `$ 0.9 €`, `€ 1.1 $`, `EUR $1.25`, `EUR $ 1.5`, `"ACME INC" 12.5 "Big Bank Ltd"`,
`"ACME INC" 12.5 USD`, `ACME €3`, `ACME -$3`, `ACME $-3`, `"A1" 3 "B2"`, `"1x" 3 EUR`, `VTI 3 "EUR"`, `"VTI" 3 EUR`,
`EUR 3 "EUR"`, `USD 5 "€"`, `USD 5 C_D`, `"" 3 EUR`, `USD EUR 3`, `USD -EUR 3`, `USD EUR - 3`, `USD 3 "a b"`,
`USD 3 "a""b"`: a doubled quote ends the name at the first quote and the rest is dropped.)

## Rule: the line

`P`, one or more blanks (spaces or tabs), the date, an optional time, the commodity, the rate; `P` may be glued to
the date (`P2026-01-10`). Blanks around and between the parts do not matter and trailing blanks are dropped.

After the rate, anything separated from it by a blank is dropped (`5 EUR junk after the amount`, `5 EUR ; a comment`,
`5 EUR 3`, `5 E U R` is `5 E`); a `;` comment with no blank is dropped too. A name glued to junk is one name
(`5 EURjunk`). `P` must be in column 0 and upper case; between `comment` and `end comment` or after a `;` it is
not read.

A rate is read as an amount, so a price, a lot or a label may follow it, and `prices` lists them in the text.
The **first character after the rate, blanks aside, decides**: `@`, `{`, `[` and `(` start such an annotation
(`5 EUR @ 3 X`, `5 EUR@3 X`, `5 EUR {3 X}`, `5 EUR [2026-01-05]`, `5 EUR (a)`, `5 EUR @@ 3 X`); anything else is junk
and is dropped (`=`, `==`, `^`, `"q"`, a date, and text after other junk: `5 EUR junk @ 3 X` is `5 EUR`). Two of the
annotation forms are errors (`[junk]`, a lone `(`). An annotation is not a form to support: the engine **refuses** a
`P` line whose first character after the rate is one of those four: an `unsupported-syntax` finding on its line
(`details.construct` is `price-annotation`), and the line is not listed. This also refuses the few that hledger
reads as plain (`5 EUR @`, `5 EUR {}`), which is the safe side.

### Probe

`probes/refused/prices-rate-annotations.journal`, `probes/refused/prices-error-annotation.journal` (apart from the rest: ledgermill refuses them)

### Command and output

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/refused/prices-rate-annotations.journal
P 2026-01-01 USD 5 EUR @ 3 X
P 2026-01-02 USD 5 EUR @@ 3 X
P 2026-01-03 USD 5 EUR @ 3 X
P 2026-01-04 USD 5 EUR {3 X}
P 2026-01-05 USD 5 EUR {1 X}
P 2026-01-06 USD 5 EUR [2026-01-05]
P 2026-01-07 USD 5 EUR @ 3 X
P 2026-01-08 USD 5 EUR ()
P 2026-01-09 USD 5 EUR (a)
P 2026-01-10 USD 5 EUR @ 3 X
P 2026-01-11 USD 5 EUR
P 2026-01-12 USD 5 EUR
P 2026-01-13 USD 5 EUR
P 2026-01-14 USD 5 EUR
P 2026-01-15 USD 5 EUR
P 2026-01-16 USD 5 EUR
P 2026-01-17 USD 5 EUR
P 2026-01-18 USD 5 EUR
$ hledger --no-conf -f spec/hledger/probes/refused/prices-error-annotation.journal prices --color=n
prices-error-annotation.journal:1:25: (an error at the `[`)
```

(The 2026-01-03 line is `5 EUR@3 X`, 01-07 is `(@) 3 X`, 01-11 `{}`, 01-12 a lone `@`.)

### Probe

`probes/prices-line-forms.journal`

### Command and output

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-line-forms.journal
P 2026-01-01 USD 5 EUR
P 2026-01-02 USD 5 EUR
P 2026-01-03 USD 5 EUR
P 2026-01-04 USD 5 EURjunk
P 2026-01-05 USD 5 EUR
P 2026-01-06 USD 5 AB
P 2026-01-07 USD 5 EUR
P 2026-01-08 USD 5 EUR
P 2026-01-09 USD 5 E
P 2026-01-10 USD 1 EUR
P 2026-01-11 USD 1 EUR
P 2026-01-12 USD 1 EUR
```

(The 2026-01-13 line is inside a `comment` block and the 2026-01-14 line is behind a `;`: neither is listed.)

## Rule: the date

The column is `YYYY-MM-DD`, always, whatever was written. The forms are those of [dates](dates.md): `2026/1/9`,
`2026.2.3`, `2026-3-4` and `2026/01/05` are `2026-01-09`, `2026-02-03`, `2026-03-04` and `2026-01-05`. A date with no
year takes the year of the last `Y` directive **above it** in the file, and the current year without one (so the
result depends on the clock; the engine has no clock). A time (`12:00`, `08:30:15`) after the date is read and
dropped, and it does not order rows of the same day. A secondary date (`2026-01-09=2026-01-10`) is an error, and so
is a date that does not exist (`2026-02-30`). Dates are sorted as dates (`2099-01-01` after the rest).

### Probe

`probes/prices-dates.journal`, `probes/prices-dates-year.journal`, `probes/prices-dates-time.journal`

### Command and output

```text
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-dates.journal
P 2026-01-05 USD 4 EUR
P 2026-01-09 USD 1 EUR
P 2026-02-03 USD 2 EUR
P 2026-03-04 USD 3 EUR
P 2099-01-01 USD 5 EUR
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-dates-year.journal
P 2024-12-31 USD 4 EUR
P 2025-01-09 USD 1 EUR
P 2025-02-03 USD 2 EUR
P 2026-01-01 USD 3 EUR
$ spec/hledger/show-prices.sh spec/hledger/probes/prices-dates-time.journal
P 2026-01-08 USD 4 EUR
P 2026-01-09 USD 1 EUR
P 2026-01-09 USD 2 EUR
P 2026-01-09 USD 3 EUR
```

(year: `Y 2025`, `1/9`, `2/3`, a full date, `Y 2024`, `12/31`. time: `12:00` is written first, then `08:30:15`, then no time: the rows stay in that order.)

## Rule: what -c run 2 changes

Only the style of the rate: the marks, the digit groups, the side and the space of the commodity. It never
changes digits or places (see above), never drops, adds or reorders a row, and the first commodity, the date
and any text after the rate (`@ 3 X`) are unchanged. A `-c` for a commodity that no row uses does nothing.
`show-prices.sh` checks the count, the dates and the first commodities on every run; it was run on all
the probes here.

## Rule: errors

A `P` line that hledger rejects stops `prices` with the line and column; the engine returns its first error
the same way ([directives-other](directives-other.md) has the rest). Here: a date that does not exist, a
secondary date, a lower-case `p`, an indented `P`, an unquoted name with a digit, a rate that is only a sign, a
number with a misplaced separator (`1,234.567,8`); and from the rest of the journal an unbalanced transaction and
a failed balance assertion.

### Probe

`probes/prices-error-date.journal`, `probes/prices-error-secondary-date.journal`, `probes/prices-error-lowercase.journal`, `probes/prices-error-indented.journal`, `probes/prices-error-digit-name.journal`, `probes/prices-error-no-rate.journal`, `probes/prices-error-number.journal`, `probes/prices-error-unbalanced.journal`, `probes/prices-error-assertion.journal`

### Command and output

First line of each error (path cut):

```text
prices-error-date.journal:2:3: This is not a valid date, please fix it.
prices-error-secondary-date.journal:1:13: unexpected '=', expecting digit
prices-error-lowercase.journal:1:1: unexpected 'p', expecting end of input or transaction or directive
prices-error-indented.journal:1:3: unexpected 'P', expecting newline
prices-error-digit-name.journal:1:15: unexpected '1'
prices-error-no-rate.journal:1:23: unexpected newline, expecting '+', '-', or number
prices-error-number.journal:1:27: invalid number (invalid use of separator)
prices-error-unbalanced.journal:2-4: This transaction is unbalanced.
prices-error-assertion.journal:3:19: Balance assertion failed in assets:a
```

## Command

```sh
spec/hledger/show-prices.sh spec/hledger/probes/prices-order-a.journal spec/hledger/probes/prices-order-b.journal
spec/hledger/show-prices.sh --first spec/hledger/probes/prices-rate-commodity-format.journal
hledger --no-conf -f spec/hledger/probes/prices-not-from-costs.journal prices --color=n --infer-market-prices
hledger --no-conf -f spec/hledger/probes/prices-error-date.journal prices --color=n
```

(The `--no-conf` is ours, so the user's configuration file cannot change the run; Plainledger's runs may differ in
that, and the rules above do not depend on it.)

## Gaps

- **File and line.** `prices` gives neither. The engine fills them: the path as given for the `P` directive's file
  and the line of the `P` (1-based, as [source-lines](source-lines.md) numbers lines), not read from hledger.
- **Year-less dates** with no `Y` take the current year; the engine needs a year from its caller or must refuse.
- **A bare rate under `D`** shows D's places (`7.00`); whether the quantity has them is not observable. `prices()` lists
  the padded text (`7.00`), and `value()` goes on using the written digits, which gives the same values.
- **A rate with a price, a lot or a label**: refused by the engine (see "the line"); what hledger lists for them is not
  specified further.
- `prices` with a query, a period, `-b`/`-e`, `--forecast`, `--auto`, aliases: not probed (not used by Plainledger).
- `include`d files are refused by ledgermill, so `P` lines there are not listed.
- A reader of the rows by whitespace cannot read `P d "" 3 EUR` (two spaces) or a quoted name with spaces: it
  needs the quoting rules above.
