# Automated and periodic rules

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

Status: not built yet. ledgermill refuses these lines today (`unsupported-syntax`, construct
`directive`); the design is to read them in full, keep none of their content and report an
`ignored-syntax` warning, with the errors below as `parse-error`.

## Rule

An **automated rule** starts with `=` in column 0 and a **periodic rule** with `~` in column 0. The
indented lines after the header are the rule's postings. hledger only uses them when it is asked to,
with `--auto` for `=` and `--forecast` (or `--budget`) for `~`. ledgermill does neither, so the rules
change nothing. What matters is what the reader does with the text: it has to read it, reject what
hledger rejects at the same line, and not let it leak into the transactions.

**No effect.** Without those flags, `print -O json` gives exactly the transactions of the file with
the rule lines left out: no transaction comes from a rule, no posting is added, and the line
numbers of the transactions are those of the file. `hledger check` is the same. A rule's amounts
set no display style or places, and no commodity, for the transactions. A rule inside a `comment`
block is a comment, valid or not.

**Where a rule is.** A header is a line that starts with `=` or `~` in column 0. The rule is that line
plus the indented lines below it. It ends at the first line that is not indented: a blank line, a
comment at the margin (`;`, `#`, `*`), a directive, a transaction or another rule. So a rule ends the
transaction above it without a blank line, and a transaction header ends the rule above it. A
`=` or `~` that is indented is an error (the position is the sign: line, column 2 after one blank).
An indented line that follows the end of a rule (a blank line or a margin comment in the middle of
it) is the error of any indented line with nothing above it: its line, column 5 for four blanks.

**The `=` header never fails.** Everything after the `=` is accepted, or nothing: a query that is not
valid (a regular expression that does not compile, `date:garbage`, `amt:abc`, `depth:x`, an unclosed
quote, an unknown term) is only an error when the rule is applied with `--auto`. A comment after
the query, `=;x`, `==`, a tab after the sign are all fine. `hledger check` does not read the query
either.

**The `~` header is parsed.** The text after the `~` (blanks around it are free; `~monthly` is
fine) is read as a period expression, up to the first of: two blanks in a row (two spaces, or a tab
and a space, or two tabs: one tab alone does not end it), a `;`, the end of the line. What follows two
blanks is a description and is not read at all (it may hold anything, including a `(code)`, a `*`
or a `|`); what follows `;` is a comment (its dates are not read, even `date:garbage`). The text
before it must be a period expression **and nothing else**: the error is at the first character that
the expression does not use (`~ monthly rent` fails at the blank before `rent`, because one blank
does not end the expression). An empty expression (`~`, `~ `, `~ ;c`) is an error at the first
character after the blanks. The grammar is in "Period expressions" below.

**The postings are read like the postings of a transaction**: the same parser, so the same errors at
the same position (a bad amount, text after an amount, a bad assertion or lot or price, an unclosed
quote, a status mark with no account, a posting comment with a bad date, a bad `date:` or
`[DATE]`). Comment lines are fine before the first posting, between postings and after them
(and, as in a transaction, one under a posting is part of its comment: its dates are read). The
differences:

- **Nothing is checked after the parse.** A rule may have a single posting, postings that do not
  balance, any number of postings without an amount, an assertion that would fail, an assignment with
  a posting date (`a  = 5 EUR ; date:...`), a lot, a price, a number with 34 digits, `1.2.3`
  or `1,2,3` (read as numbers, as in a transaction): none of it is looked at, since no rule is applied.
  Nor does a rule take part in balance assertions or assignments of the transactions.
- **`=` rules accept a multiplier amount**: `*` right before an amount in the usual form (`*2`, `*-2`,
  `*0.5`, `*1,000`, `*1e3`, `*2 EUR`, `*$2`, `*-$2`, `*EUR 2`, `*2 @ 3 EUR`, `*2 = 5`) or `*` alone,
  with a comment or at the end of the line. The `*` is glued to what follows: `* 2` and `*EUR`
  are errors, and so are `**2`, `-*2`, `1 EUR *2`, `*2 *3`. A `~` rule, like a transaction, rejects a
  multiplier at the `*`. `%account` is only text: in an account name, or as a commodity symbol after a
  number; as an amount on its own it is the error any word is.
- **A date without a year in a posting comment** (`date:02-03`, `date2:02-03`, `[02-03]`) takes the
  year of the transaction in a transaction. A `~` rule has no transaction: it takes the year of the
  `Y` directive above it, or else the **current year** (only `02-29` can depend on which). An `=`
  rule has no year at all, `Y` or not, and the date is an error at its first character ("the current
  year is unknown") unless an earlier date of the same comment line gives one: `date:2026-01-02
  date:02-03` and `date2:2026-01-02 date:02-03` are fine (the year of an earlier `date:` or `date2:`
  value, same line), `[2026-01-02=02-03]` is fine (the second date of a bracket takes the first's year),
  but `date:02-03 date2:2026-01-02` (the full date comes after), `date:2026-01-02 [02-03]` (brackets
  take no year from a tag), a full date on the line above, and `[02-03=2026-01-02]` are errors. Full
  dates are read as everywhere: a month or day that does not exist is an error in both kinds of rule.
  A comment line before the first posting, and the header's own comment, are not read for dates.

**The rest of the file is read as if the rule were not there.** `Y`, `D`, `decimal-mark`, `commodity`,
`alias` and `apply account` do not change what a rule's amount may be (no number was found that is
valid in a rule and not in a transaction, or the other way round, under any of them), and a
rule changes none of them. Line numbers after a rule are the lines of the file.

**Errors.** A parse error in a rule fails the whole journal, like any parse error. The first error of a
file is its first parse error: the rule's line when nothing earlier is wrong. Parse errors come before the
balancing and assertion errors of the same file, even those of transactions above the rule
(`rule-bad-before-unbalanced`: the error is the rule's line, not the transaction's). With several
files, a file is read to its end, balancing included, before the next one starts: an unbalanced
transaction in the first file is reported before an invalid rule in the second
(`rule-file-a` + `rule-file-b`), and the other way round when the files are swapped.

### Period expressions

Validated with `period-forms.txt`, 474 expressions with hledger's verdict and the column of its error
(`period-verdicts.sh` rebuilds it), and by a model of the rules below that agreed with hledger on
about 40,000 generated expressions; the exceptions are the two "Not settled" classes at the end.
(The model knew digit runs of up to eight digits; longer runs were probed by hand.)
Everything is case-insensitive. Blanks (spaces only) between the words are optional where it is
unambiguous, and `every2days`, `monthlyfrom 2026`, `monthly2026` and `lastquarters` are read; a tab
between words is not allowed, and two blanks in a row end the expression (above).

A period expression is **an interval, a span, or an interval and a span** (`monthly`, `from jan`,
`monthly from jan`).

- **Interval**: `daily`, `weekly`, `monthly`, `quarterly`, `yearly`, `biweekly`, `fortnightly`,
  `bimonthly`; `every day|week|month|quarter|year`; `every N days|weeks|months|quarters|years`
  (digits, any count including 0, the plural only: `every 1 day` is an error, `every 1 days` is
  fine); `every Nth day` followed by nothing, `of week` or `of month` (`of year` is an error); `every
  WEEKDAY` (`mon`..`sun` or the full name; `tues` is an error), `every weekday`, `every weekendday`;
  `every Nth WEEKDAY` with nothing or `of month`; `every M/D` with nothing or `of year`; `every MONTH
  Nth` and `every Nth MONTH`, with nothing or `of year`. `Nth` is digits and an ending as in `1st`, `2nd`, `3rd`
  or `4th`, whatever the number (`every 0th day of week`, `every 32nd day`, `every 1th day`
  are accepted); the suffix is required (`every 5` is an error). In `M/D` the month is 1-12 and the
  day exists in that month, 29 February included; `every 13/15` and `every 2/30` are errors. The
  separator of `M/D` is `/`, `-` or `.`.
- **Span**, with *DATE* below and *SEP* one of `to`, `..` or `-`:
  - `from DATE`, `since DATE`, optionally followed by *SEP* DATE (`until` cannot follow);
  - `to DATE`, `until DATE`;
  - `in DATE`: one date, no range after it (`in 2026 to 2027` is an error);
  - *SEP* DATE (a leading `..`, `-`, `to`);
  - *DATE*, optionally followed by *SEP* *DATE* (two dates with only a blank between them are an error).
- **Date** (a "smart date"):
  - `YYYY`: four or more digits, read as a year (`0000`, `12345`, `123456789` and `20261301` are
    years); fewer than four digits is an error, except that one or two digits are a day of the month,
    1-31. Eight digits that make a real date are that date, and a longer run of digits that starts with
    a real `YYYYMMDD` is an error (`202601151`; `202602301` is a year, there is no 30 February);
  - `YYYY-MM` and `YYYY-MM-DD`, separator `-`, `/` or `.`, the same one twice, one or two digits for the
    month (1-12) and the day (1-31). If the month is not 1-12 the text is read as a year, a
    range sign and a day (`2026-13` is the year 2026 up to day 13; `2026-13-01` fails because of the 13);
  - a full date whose day the calendar does not have (`2026-02-30`, `2026-04-31`) is accepted **only as
    the sole date of the expression**: alone, after `from`/`since`, or after an interval. Anywhere
    else (`to 2026-02-30`, `in ...`, in a range) it is an error. The same goes for `YYYY-qN`;
  - `YYYYqN` and `qN`, N 1-4; `M/D` (separator `-`, `/` or `.`, the day exists in that month, 29
    February allowed); `jan`..`dec` and the full month names (`sept` is an error);
  - `today`, `yesterday`, `tomorrow`;
  - `last`, `this` or `next` and then `day`, `week`, `month`, `quarter` or `year` (the plural too; `ago` or
    `ahead` may follow), or a weekday name, or a month name; `in` may come before the unit forms
    (`in next quarter to 2027` is a range, not an `in` span);
  - `N days|weeks|months|quarters|years` (the singular too), alone or followed by `ago` or `ahead`, with `in`
    in front or not; `N` is digits, 0 included (`in -1 days` is also accepted). Hours, fortnights and
    weekdays are not units.

Weekday names alone (`monday`), `sec`, `jan 2026`, `last yr`, `next 3 days`, `2026 daily` (a span before
an interval), `monthly weekly` and `daily daily` are errors.

Not settled, left to the implementation (hledger accepts all of these but they are outside the
grammar above; none of them is a plausible rule): a `-` or `..` glued after a date (`2026-01-01-`,
`feb..`, `2025-`) and, after some forms, with a blank before it (`this year -`; not after `2026`,
`jan`, `q1` or `today`); a number glued to the date that follows it (`2026-01-152026-01`, `2026-011`,
`19992026q1`).

## Probe

The probes are under `probes/`, `rule-*.journal`. Rules that are read:

- `rule-auto-basic`, `rule-periodic-basic`: a rule between two transactions;
- `rule-auto-headers`: seventeen `=` headers (empty, glued, with a comment, an invalid regex, date, amount and
  quote, a tab after the sign), none rejected;
- `rule-periodic-headers`: twenty-four period expressions; `rule-periodic-description`: what may follow
  one (two blanks and a description, a comment with one blank, with none, with two; a tab or a tab
  and a space);
- `rule-auto-postings`, `rule-periodic-postings`, `rule-auto-multiplier`: what a rule's postings may
  hold;
- `rule-auto-comment-dates`, `rule-periodic-comment-dates`: dates in posting comments, with and without
  a year;
- `rule-ends-transaction`, `rule-directives`, `rule-amount-style`, `rule-in-comment-block`,
  `rule-no-final-newline`, `rule-header-no-final-newline`, `rule-crlf`: the rest of the file;
- `rule-file-c`: a rule alone in a file.

Rules that are rejected: the table below, and the pairs `rule-file-a` + `rule-file-b`,
`rule-file-b` + `rule-file-a`, `rule-file-c` + `rule-file-a` (`specPairs` of `harness/compare.ts` does not list
them yet: add them with the engine, so the harness compares the first error across files).

Command, from the repository root:

```sh
spec/hledger/show.sh spec/hledger/probes/rule-auto-basic.journal
spec/hledger/show.sh spec/hledger/probes/rule-periodic-basic.journal
hledger --no-conf -f spec/hledger/probes/rule-auto-basic.journal print --auto
spec/hledger/probe-verdicts.sh                       # one line per rule probe, then the three pairs
cut -f2- spec/hledger/period-forms.txt | spec/hledger/period-verdicts.sh - | diff - spec/hledger/period-forms.txt
```

## Output

rule-auto-basic: the rule and its posting are not in the output; with `--auto` they would be:

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"before","comment":"","line":2,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"after","comment":"","line":9,"postings":[{"account":"expenses:food","type":"RegularPosting","comment":"","amounts":[["EUR",10,0]]},{"account":"cash","type":"RegularPosting","comment":"","amounts":[["EUR",-10,0]]}]}
```

```text
2026-01-02 after
    expenses:food          10 EUR
    (budget:food)             0.5        <- print --auto only
    cash
```

rule-periodic-basic (the second transaction is on line 10):

```text
{"date":"2026-01-01","status":"Unmarked","code":"","description":"before","comment":"","line":2,"postings":[{"account":"a","type":"RegularPosting","comment":"","amounts":[["EUR",1,0]]},{"account":"b","type":"RegularPosting","comment":"","amounts":[["EUR",-1,0]]}]}
{"date":"2026-01-02","status":"Unmarked","code":"","description":"after","comment":"","line":10,"postings":[{"account":"c","type":"RegularPosting","comment":"","amounts":[["EUR",2,0]]},{"account":"d","type":"RegularPosting","comment":"","amounts":[["EUR",-2,0]]}]}
```

`probe-verdicts.sh`: the probes that are read, with the number of transactions they give:

```text
rule-amount-style | accepts | 1 transactions
rule-auto-basic | accepts | 2 transactions
rule-auto-comment-dates | accepts | 0 transactions
rule-auto-headers | accepts | 1 transactions
rule-auto-multiplier | accepts | 1 transactions
rule-auto-postings | accepts | 1 transactions
rule-crlf | accepts | 2 transactions
rule-directives | accepts | 1 transactions
rule-ends-transaction | accepts | 3 transactions
rule-file-c | accepts | 1 transactions
rule-header-no-final-newline | accepts | 1 transactions
rule-in-comment-block | accepts | 1 transactions
rule-no-final-newline | accepts | 1 transactions
rule-periodic-basic | accepts | 2 transactions
rule-periodic-comment-dates | accepts | 0 transactions
rule-periodic-description | accepts | 1 transactions
rule-periodic-headers | accepts | 1 transactions
rule-periodic-postings | accepts | 1 transactions
```

The probes that are rejected. The first error is `LINE:COLUMN` (what hledger prints after the file
name; a tab counts to the next multiple of eight, so `\t` in column 1 is column 9):

| Probe | First error | What is wrong |
|---|---|---|
| `rule-periodic-bad-blank` | 6:3 | only a blank after the sign |
| `rule-periodic-bad-comma` | 6:10 | text after the period expression |
| `rule-periodic-bad-comment` | 6:3 | a comment cannot stand for the period expression |
| `rule-periodic-bad-comment-glued` | 6:2 | a comment cannot stand for the period expression |
| `rule-periodic-bad-day` | 6:11 | day 32 |
| `rule-periodic-bad-double-sign` | 6:2 | two signs |
| `rule-periodic-bad-empty` | 6:2 | a periodic rule needs a period expression |
| `rule-periodic-bad-every` | 6:8 | every needs what to repeat |
| `rule-periodic-bad-every-date` | 6:13 | a day the month does not have |
| `rule-periodic-bad-every-negative` | 6:9 | a negative count |
| `rule-periodic-bad-every-one-day` | 6:11 | the unit takes the plural after a count |
| `rule-periodic-bad-from` | 6:10 | from needs a date |
| `rule-periodic-bad-in` | 6:10 | in needs a date |
| `rule-periodic-bad-month` | 6:8 | month 0 |
| `rule-periodic-bad-one-space` | 6:10 | a description needs two blanks before it |
| `rule-periodic-bad-order` | 6:24 | from after to |
| `rule-periodic-bad-partial` | 6:5 | month 13 |
| `rule-periodic-bad-quarter` | 6:7 | quarter 5 |
| `rule-periodic-bad-tab` | 6:10 | a single tab is not enough before a description |
| `rule-periodic-bad-two-dates` | 6:21 | two dates need to, .. or - |
| `rule-periodic-bad-word` | 6:3 | not a period expression |
| `rule-periodic-bad-year` | 6:6 | a year needs four digits |
| `rule-auto-bad-account-amount` | 7:16 | a number is required before a commodity-less % word |
| `rule-auto-bad-account-only` | 7:12 | no number in the amount |
| `rule-auto-bad-assertion` | 7:15 | an assertion with no amount |
| `rule-auto-bad-assertion-lot` | 7:22 | a lot in an asserted amount |
| `rule-auto-bad-comment-bracket-date` | 7:17 | an invalid bracketed date in a posting comment |
| `rule-auto-bad-comment-bracket-no-year` | 7:17 | a bracketed date without a year, which no transaction supplies |
| `rule-auto-bad-comment-bracket-no-year-after-tag` | 7:33 | brackets do not take their year from a tag |
| `rule-auto-bad-comment-continued-date` | 8:12 | an invalid date in a comment line below a posting |
| `rule-auto-bad-comment-date` | 7:21 | an invalid date in a posting comment |
| `rule-auto-bad-comment-date-calendar` | 7:21 | a date that does not exist, in a posting comment |
| `rule-auto-bad-comment-date-empty` | 7:21 | date: with no date |
| `rule-auto-bad-comment-date-no-year` | 7:21 | a date without a year, which no transaction supplies |
| `rule-auto-bad-comment-date-no-year-first` | 7:21 | a date without a year comes before the full date that would give its year |
| `rule-auto-bad-comment-date-no-year-next-line` | 8:13 | a date without a year, whose full date is on another line |
| `rule-auto-bad-date-amount` | 7:12 | a date is not an amount |
| `rule-auto-bad-expression` | 7:10 | arithmetic |
| `rule-auto-bad-junk` | 7:14 | text after an amount |
| `rule-auto-bad-lot-date` | 7:19 | a lot date that does not exist |
| `rule-auto-bad-lot-open` | 7:20 | an unclosed lot |
| `rule-auto-bad-number` | 7:10 | a sign with no digits |
| `rule-auto-bad-posting-after-blank` | 9:5 | a blank line ends the rule: line 9 is an indented line with nothing above it |
| `rule-auto-bad-posting-after-margin` | 9:5 | a margin comment ends the rule: line 9 is an indented line with nothing above it |
| `rule-auto-bad-price` | 7:14 | a price with no amount |
| `rule-auto-bad-price-twice` | 7:22 | two prices |
| `rule-auto-bad-quote` | 7:10 | an unclosed quoted commodity |
| `rule-auto-bad-star-after` | 7:14 | a multiplier after an amount |
| `rule-auto-bad-star-commodity` | 7:12 | a multiplier with no number |
| `rule-auto-bad-star-end` | 7:11 | two multipliers |
| `rule-auto-bad-star-sign` | 7:9 | a sign before the star |
| `rule-auto-bad-star-space` | 7:10 | a blank between * and the number |
| `rule-auto-bad-star-twice` | 7:9 | two stars |
| `rule-auto-bad-status-alone` | 7:5 | a status mark with no account |
| `rule-periodic-bad-assertion` | 7:15 | an assertion with no amount |
| `rule-periodic-bad-comment-continued-date` | 8:12 | an invalid date in a comment line below a posting |
| `rule-periodic-bad-comment-date` | 7:21 | an invalid date in a posting comment |
| `rule-periodic-bad-comment-date-calendar` | 7:21 | a date that does not exist, in a posting comment |
| `rule-periodic-bad-comment-date-leap` | 4:22 | 02-29 in a periodic rule takes the year of the Y directive: 2025 has no 29 February (error on line 4). |
| `rule-periodic-bad-comment-date-leap-bracket` | 4:18 | 02-29 in a periodic rule takes the year of the Y directive: 2025 has no 29 February (error on line 4). |
| `rule-periodic-bad-junk` | 7:14 | text after an amount |
| `rule-periodic-bad-number` | 7:10 | a sign with no digits |
| `rule-periodic-bad-posting-after-blank` | 9:5 | a blank line ends the rule: line 9 is an indented line with nothing above it |
| `rule-periodic-bad-star` | 7:8 | a multiplier is only read in an automated rule |
| `rule-periodic-bad-star-sign` | 7:8 | a multiplier is only read in an automated rule |
| `rule-auto-indented` | 6:2 | a rule header must start in column 0 |
| `rule-periodic-indented` | 6:2 | a rule header must start in column 0 |

The ones with a balancing error or a pair, where the first error is not at a column:

| Probe | First error | What |
|---|---|---|
| `rule-bad-before-unbalanced` | 7:3 | the invalid rule, not the unbalanced transaction above it (lines 3-5) |
| `rule-bad-after-unbalanced` | 7:12 | same, the rule's posting is invalid |
| `rule-unbalanced-after-valid` | 5 | a valid rule above an unbalanced transaction: the transaction, `FIRST-LAST` lines 5-7 |
| `rule-file-a` | 2 | alone: the unbalanced transaction, lines 2-4 |
| `rule-file-b` | 3:3 | alone: the invalid rule |
| `rule-file-a` + `rule-file-b` | `rule-file-a`:2 | a file is finished before the next |
| `rule-file-b` + `rule-file-a` | `rule-file-b`:3:3 | |
| `rule-file-c` + `rule-file-a` | `rule-file-a`:2 | a valid rule in the first file changes nothing |

The `period-forms.txt` lines the grammar is about (`ok` or `err:COLUMN`, then the expression):

```text
ok	every 2 days
err:11	every 1 day
ok	every 1 days
ok	every 5th day of month
err:16	every 2nd day of year
err:13	every 2/30
ok	every 2/29
ok	monthly in 2026
err:18	monthly in 2026 to 2027
ok	in next quarter to 2027
err:10	in 2026 to 2027
err:13	2026-01-01 2026-02-01
ok	2026-01-01 to 2026-02-01
ok	2026-02-30
err:13	2026-02-30 to 2027
err:3	sec
err:6	999
ok	20261301
err:11	202601151
ok	202602301
err:7	2026 daily
err:10	every 5
```
