# Syntax: what ledgermill reads, ignores, refuses

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

Status: **specified, not built**. The classes below are the design; "Where ledgermill differs today"
lists the rows ledgermill does not yet do as the table says.

Every construct of the hledger journal format and what ledgermill does with it, with the probe that
shows what hledger does. The list of constructs comes from the format's documentation (the directives,
the parts of a transaction and of a posting, the amount, price, lot and assertion forms, the comments);
the behaviour comes from running hledger.

## Rule

Three classes, as for Beancount ([syntax](../beancount/syntax.md)).

- **supported**: read, and the result is what hledger gives, errors included: `check()` reports what
  hledger rejects, with the file and line of its first error ([check-vs-print](check-vs-print.md): that
  is what a plain `hledger check` rejects, nothing more). A directive that changes nothing ledgermill
  reports (`payee`, `P`, `python`, ...) is supported: there is nothing to ignore and no finding.
- **ignored (warning)**: read in full, kept nowhere, with an `ignored-syntax` warning
  (`details.construct` names it). The rest of the file is read as if the lines were not there. Only the
  `=` and `~` rules are in this class, by design ([rules-ignored](rules-ignored.md)): hledger applies
  them only with `--auto` and `--forecast`, which ledgermill does not offer.
- **unsupported (error)**: `unsupported-syntax` (`details.construct`). For what hledger accepts and could
  change a transaction or a posting that ledgermill does not reproduce. The lines that cannot be read are
  not returned as transactions. It is never used for what hledger rejects: that is a `parse-error` (or
  another finding) on the line hledger blames.

Probes are under `probes/` (run by the harness), or `probes/refused/` for the unsupported ones (not run:
hledger accepts them, ledgermill reports an error). `rule-*` probes belong to
[rules-ignored](rules-ignored.md). A probe name is a file of `probes/` without its `.journal`; `refused/NAME` is in `probes/refused/`
and `app/NAME` is in `../../harness/journals/app/`.

| Construct | Class | Probe |
|---|---|---|
| transaction: date `YYYY-MM-DD`, with `/` or `.`, leading zeros optional | supported | `header-code-and-dates`, `header-date-mixed-separators`, `date-invalid`, `date-leap`, `date-year-zero-and-five-digits` |
| secondary date `DATE=DATE` (a year-less one takes the primary's year) | supported | `header-code-and-dates`, `directive-year` |
| date with no year after `Y`, `year` or `apply year` | supported | `directive-year` |
| date with no year and no `Y` directive (transaction, secondary primary, `P`) | unsupported | `refused/date-without-year-header`, `refused/date-without-year-before-y`, `refused/date-without-year-price`, `directive-date-without-year` |
| status `*` or `!`, code `(...)`, description, `payee \| note` | supported | `header-forms`, `header-status-glued-to-date`, `header-unusual-accepted-by-hledger` |
| transaction comment: on the header, on indented lines below | supported | `comments`, `header-forms` |
| transaction with no postings | supported | `syntax-no-postings` |
| posting indented with spaces or a tab | supported | `tab-indent`, `posting-forms`, `posting-indented-header-line` |
| posting status `*` or `!` | supported | `posting-status`, `posting-status-and-virtual` |
| virtual `(a)` and balanced virtual `[a]` postings | supported | `posting-virtual`, `posting-status-and-virtual` |
| account names: spaces, unicode, `:`, the two-blank delimiter | supported | `posting-account-separators`, `posting-single-space-before-amount` |
| amounts: commodity before or after, spaced or not, quoted symbols, signs | supported | `amount-forms`, `posting-forms` |
| numbers: digit groups, decimal marks, exponents, trailing point | supported | `numbers-forms`, `numbers-marks`, `numbers-groups-and-exponent`, `numbers-group-spaces`, `numbers-invalid-separators`, `numbers-big` |
| a number of up to 255 decimal places | supported | `numbers-places-limit`, `numbers-many-places` |
| a number of more than 255 decimal places (an error) | supported | `numbers-too-many-places`, `numbers-too-many-places-exponent` |
| an exponent above 4096 | unsupported | `refused/number-too-large-exponent` |
| cost `@` and `@@` | supported | `prices`, `posting-price`, `app/unit-cost-spelling` |
| virtual cost `(@)` and `(@@)` | supported | `syntax-virtual-cost` |
| lot `{}`, `{{}}`, `{=}`, `[DATE]`, `(LABEL)` | supported | `lots`, `app/repeated-account-annotations` |
| valuation expression `((...))` after an amount (ignored by hledger) | supported | `syntax-valuation-expression` |
| elided amount, and the places it gets | supported | `elided`, `elided-two`, `elided-grouping`, `posting-elided-amount` |
| balancing: groups, tolerance, inferred prices | supported | `balance-ok-mixed-places`, `balance-unbalanced`, `tolerance`, `tolerance-fails`, `inferred-prices`, `inferred-prices-same-sign` |
| conversion postings (`equity:conversion` and the like, `type:` tags) | supported | `conversion`, `conversion-declared`, `conversion-type-tags`, `conversion-names`, `conversion-leading-run`, `conversion-several` |
| conversion postings: a priced posting on a conversion account; several priced postings after an odd run | unsupported | `refused/conversion-postings-priced-conversion`, `refused/conversion-postings-several-priced` |
| balance assertions `=`, `==`, `=*`, `==*` | supported | `posting-assertion`, `assert-forms`, `assert-compare`, `assert-fail-basic` and the other `assert-*` |
| balance assignments | supported | `assign-basic`, `assign-total`, `assign-inclusive` and the other `assign-*` |
| posting comments and tags | supported | `comments`, `comment-tags-do-nothing` |
| posting dates: `date:`, `date2:`, `[DATE]`, `[DATE=DATE]` | supported | `comment-dates`, `comment-date-invalid`, `posting-comment-date-tag-invalid`, `posting-comment-bracket-date-invalid` |
| comment lines starting with `;`, `#` or `*` | supported | `comments`, `comments-col0-ends-transaction`, `app/proj006-comment-block` |
| `comment` ... `end comment` block, also unterminated | supported | `directive-comment-block`, `directive-comment-unterminated`, `directive-comment-end-text` |
| a blank line, or a line of blanks, ends a transaction | supported | `comments-blank-ends-transaction`, `comments-spaces-only-line-ends-transaction` |
| BOM, CRLF, no final newline, a last line of blanks | supported | `bom`, `crlf`, `no-final-newline`, `spaces-only-last-line-with-newline`, `spaces-only-last-line-no-newline` |
| an empty file | supported | `empty` |
| a carriage return inside a line | unsupported | `refused/control-character-in-line`, `refused/control-character-line-ends` |
| several files read together | supported | `order-file-a` with `order-file-b`, `assert-file-a` with `assert-file-b` |
| `account` (a `type:` tag, indented lines, a comment) | supported | `directive-ignored`, `conversion-type-tags`, `conversion-type-invalid`, `directive-account-virtual-name`, `directive-account-balanced-virtual-name` |
| `alias`, regex alias, `end aliases` | supported | `directive-alias`, `directive-alias-order`, `directive-alias-no-equals`, `directive-alias-empty-pattern` |
| `apply account`, `end apply account` | supported | `directive-apply-account` |
| `commodity` (a symbol, a sample amount, `format` and other indented lines) | supported | `directive-commodity-format`, `directive-commodity-needs-mark`, `directive-ignored`, `directive-commodity-lone-mark` |
| `decimal-mark` | supported | `directive-decimal-mark` |
| `D` | supported | `directive-default-commodity`, `directive-default-commodity-text` |
| `payee`, `tag` | supported | `directive-ignored`, `directive-tag-two-words` |
| `P` | supported | `directive-ignored`, `directive-price-invalid`, `directive-price-no-amount` |
| `Y`, `year`, `apply year`, `end apply year` | supported | `directive-year`, `directive-year-glued`, `directive-end-apply-year-text` |
| `apply fixed`, `apply tag`, `assert`, `bucket`, `A`, `capture`, `check`, `define`, `eval`, `expr`, `value`, `N`, `C`, `tag`, and their `end` lines (read and ignored by hledger) | supported | `syntax-ledger-directives`, `directive-end-apply-fixed-text`, `directive-end-apply-tag-text`, `directive-bare-bucket`, `directive-bare-a`, `directive-bare-n`, `directive-n-number`, `directive-bare-c`, `directive-c-one-amount`, `directive-c-no-number` |
| `python` with its indented code, and lines that start with `--` | supported | `directive-python-block`, `directive-flag-line` |
| `apply` with nothing, or with an unknown word | supported | `directive-apply-bare`, `directive-apply-unknown` |
| automated rule `=` with its postings | ignored (warning) | `rule-auto-basic`, `rule-auto-headers`, `rule-auto-postings`, `rule-auto-multiplier` |
| periodic rule `~` with its postings | ignored (warning) | `rule-periodic-basic`, `rule-periodic-headers`, `rule-periodic-description`, `rule-periodic-postings` |
| `include` (a file that exists; a missing one is an error) | unsupported | `refused/include` |
| a timeclock, timedot or CSV file (read by hledger by its extension) | unsupported | `refused/timeclock.timeclock`, `refused/timedot.timedot`, `refused/csv.csv` |
| a timeclock line (`i`, `o`) in a journal, `!include`, an unknown word at the margin (hledger rejects them) | supported | `directive-forms.txt` (lines `i ...`, `o ...`, `!include`, `x`) |

"supported" for a line that hledger rejects means that ledgermill rejects it too, with a `parse-error` on
the line hledger names (the whole file, for a rejected file, gives no transaction).

## Probe

The probes named above. Commands and outputs are in the notes of each behaviour (see the index in the
README); for what this note adds:

```sh
spec/hledger/probe-verdicts.sh spec/hledger/probes/syntax-virtual-cost.journal spec/hledger/probes/syntax-valuation-expression.journal
spec/hledger/show.sh spec/hledger/probes/syntax-no-postings.journal
spec/hledger/show.sh spec/hledger/probes/syntax-ledger-directives.journal
cut -f2- spec/hledger/directive-forms.txt | spec/hledger/directive-verdicts.sh - | diff - spec/hledger/directive-forms.txt
for f in spec/hledger/probes/refused/*.journal; do spec/hledger/probe-verdicts.sh $f; done
for f in refused/timeclock.timeclock refused/timedot.timedot refused/csv.csv; do hledger --no-conf -f spec/hledger/probes/$f print; done
```

`directive-forms.txt` is hledger's verdict on 200 lines of the margin: the arguments each directive takes,
bare, glued, with an extra word, a comment or an indented line below it (`ok` or `err:LINE:COLUMN`, the
line being the line of the file the error is on: `err:2:5` is an indented line after a directive that takes
none). The rows that name a `directive-*` probe are the lines of this file where ledgermill differs today.
The reader needs the forms of the file, not only those probes.

## Output

`probe-verdicts.sh` on the probes that are new in this note: hledger accepts every probe of `refused/`
(`include-target` is the file `include` includes):

```text
$ for f in spec/hledger/probes/refused/*.journal; do spec/hledger/probe-verdicts.sh $f; done
control-character-in-line | accepts | 1 transactions
control-character-line-ends | accepts | 1 transactions
conversion-postings-priced-conversion | accepts | 1 transactions
conversion-postings-several-priced | accepts | 1 transactions
date-without-year-before-y | accepts | 2 transactions
date-without-year-header | accepts | 1 transactions
date-without-year-price | accepts | 1 transactions
include-target | accepts | 1 transactions
include | accepts | 2 transactions
number-too-large-exponent | accepts | 1 transactions
```

```text
$ spec/hledger/probe-verdicts.sh spec/hledger/probes/syntax-virtual-cost.journal spec/hledger/probes/syntax-valuation-expression.journal
syntax-virtual-cost | accepts | 2 transactions
syntax-valuation-expression | accepts | 2 transactions
```

`refused/timeclock.timeclock`, `refused/timedot.timedot` and `refused/csv.csv` (with `csv.csv.rules`) each print one
transaction with `hledger -f FILE print`: `2026-01-01 * desc` with `(work:a) 8.00h`, `2026-01-01 *` with two
postings of 4.00 and 2.50, and `2026-01-01 coffee` with `assets:bank EUR5` and `expenses:x EUR-5`.

`directive-forms.txt`, the lines that matter for the gaps (`ok` or `err:LINE:COLUMN`, then the line; `\n`
is a new line and `\t` a tab):

```text
ok	python\n    print(1)
ok	--strict
ok	--foo bar
ok	end apply fixed x
ok	end apply tag x
ok	end apply year x
ok	commodity ,
ok	D 1.00 EUR junk
ok	year2026
err:1:9	account (a)
err:1:9	account [a]
err:1:7	tag a b
err:6:1	alias a
err:1:7	alias =b
err:3:13	comment\nx\nend comment x
err:1:1	bucket
err:1:1	A
err:1:2	N
err:1:3	N 1
err:1:2	C
err:1:8	C 1 EUR
err:1:4	C a
err:1:1	apply
err:1:1	apply x
err:1:1	include x.journal
err:1:2	!include x.journal
err:1:1	i 2026-01-01 09:00:00 a
err:1:1	x
```

## Where ledgermill differs today

Built from `bun harness/real.ts` and the harness on every probe above, with the wasm of this branch
(`cargo build --release --target wasm32-unknown-unknown -p ledgermill-wasm`, then `wasm-bindgen`). Every
journal of the corpus (`harness/journals/`, `probes/`) agrees with hledger today except the ones below. Before
the probes of this change the harness said, for hledger, 968 pass, 0 fail, 2 skip (`prototype/w7`, which has
rules, and `directive-date-without-year`); with the `rule-*` probes, 974 pass, 0 fail, 86 skip. The 27 probes
added for the rows below fail today: this list is the engine's to-do list, and the probes are its tests.

Silent: ledgermill returns the transactions, with no finding, of a journal that hledger rejects (16 probes).
These are the ones that make figures differ from hledger's without an error or a warning:

| Construct | Probe | hledger | ledgermill today |
|---|---|---|---|
| a number with more than 255 decimal places (also through an exponent: `1e-256`) | `numbers-too-many-places`, `numbers-too-many-places-exponent` | parse error at the number | reads it |
| `account (a)`, `account [a]` | `directive-account-virtual-name`, `directive-account-balanced-virtual-name` | parse error at the parenthesis | accepts |
| `tag a b` (a blank in the name) | `directive-tag-two-words` | parse error at the second word | accepts |
| `alias =b` (nothing before the `=`) | `directive-alias-empty-pattern` | parse error | accepts |
| text after `end comment`, inside a comment block | `directive-comment-end-text` | parse error on that line | accepts |
| `bucket` or `A` with no account | `directive-bare-bucket`, `directive-bare-a` | parse error at the word | accepts |
| `N` with no symbol, or with a number | `directive-bare-n`, `directive-n-number` | parse error | accepts |
| `C` with nothing, one amount, or no number | `directive-bare-c`, `directive-c-one-amount`, `directive-c-no-number` | parse error | accepts |
| `apply` with nothing, or an unknown word | `directive-apply-bare`, `directive-apply-unknown` | parse error at the word | accepts |

Another line: `alias a` (no `=`), `directive-alias-no-equals`. hledger blames the line after the last line of
the file; ledgermill reports a `parse-error` on the alias's own line. Both reject, but the first errors
differ, which the harness counts as a failure. (The line is the position at which hledger found out that
no `=` came; to match it, report the line after the last one.)

Refused, where hledger reads the file (a `parse-error` finding for a line hledger accepts, so the
transaction it belongs to is dropped):

| Construct | Probe | ledgermill today |
|---|---|---|
| `python` and its indented code | `directive-python-block` | parse-error on the first code line |
| a line starting with `--` and a word (`--strict`, `--foo=bar`, `--foo bar`; `--` alone is fine) | `directive-flag-line` | parse-error |
| text after `end apply fixed`, `end apply tag`, `end apply year` | `directive-end-apply-fixed-text`, `directive-end-apply-tag-text`, `directive-end-apply-year-text` | parse-error |
| `commodity ,` | `directive-commodity-lone-mark` | parse-error |
| text after the amount of `D` | `directive-default-commodity-text` | parse-error |
| `year2026` (glued) | `directive-year-glued` | parse-error |
| virtual costs `(@)`, `(@@)` | `syntax-virtual-cost` | parse-error on the line |
| a valuation expression `((...))` | `syntax-valuation-expression` | parse-error on the line |

Not an error of the verdict, a different kind of finding to decide:

- A timeclock, timedot or CSV file (`refused/*`) gets `parse-error`s (line by line) and, for the timedot one, a
  transaction made of a date line; hledger reads the file by its extension. The class says
  `unsupported-syntax`, a construct such as `timeclock`, `timedot` or `csv`, on line 1, and no transaction.
- `!include` is an error in hledger (at the `!`); ledgermill says `unsupported-syntax` (`include`) on the
  same line. The verdict and the line match; hledger's kind is a parse error.
- `include` of a file that does not exist is an error in hledger (line of the include); ledgermill
  says `include` too: same line, same verdict.
- The rules `=` and `~` are `unsupported-syntax` (`directive`) today; every `rule-*` probe is a SKIP, and the
  probes that hledger rejects are SKIPs too, because the first error of ours is the refusal. Each line of
  [rules-ignored](rules-ignored.md) is a case for the harness once rules are read.
- A file that is not valid UTF-8 is an error in hledger with no position (`hGetContents`); ledgermill gets a
  string, so the caller decides how to decode: nothing to do.
- A positive exponent up to 4096 and a number of up to 255 decimal places are read exactly as hledger does;
  an exponent above 4096 is `number-too-large`, as the table says. hledger reads `1e100000` (a mantissa of a
  hundred thousand digits) and does not finish on `1e9999999999999999999`: not probed further.

Probes that live in `probes/` and that ledgermill refuses today: `directive-date-without-year`
(`date-without-year`, SKIP; `crates/ledgermill/tests/findings.rs` includes it, so it was left where it
is; the same case is in `refused/date-without-year-*`). Nothing under `harness/journals/` is refused,
except `prototype/w7` (rules).
