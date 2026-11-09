# `plugin "beancount.plugins.auto_accounts"`, and the plugin lines ledgermill refuses

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-11-09.

Status: **built**. `auto_accounts` is the one plugin ledgermill runs. Every other `plugin` line is refused,
with the module name in `details.name`.

## Rule

### The plugin line

- A well-formed line is `plugin "MODULE"`, or `plugin "MODULE" "CONFIG"`, at the margin. Blanks or a tab between
  the keyword and the string, none at all (`plugin"MODULE"`), and a `;` comment after it are fine. Anything else
  is a syntax error of the reference, on that line: an unquoted name, a number or a third string after the
  module, text glued after the closing quote, an indented line. (A keyword with nothing after it is a syntax
  error too, but the reference puts it on the line after.)
- **A module name written with a backslash is refused**, whatever it spells. The reference reads the escapes of
  the string: a backslash makes the next character itself (`\o` is `o`, `\\` a backslash, `\"` a quote), except
  `\n`, `\t`, `\r`, `\b` and `\f`, which are control characters (`\x6f`, `\157` and `\u006f` are not escapes:
  `x6f`, `157`, `u006f`). So `plugin "beancount.plugins.auto_acc\ounts"` runs `auto_accounts` and
  `"beancount.plugins.auto_accou\nts"` is another module. ledgermill compares the module as written, so both are
  refused by name (as written); the parser behind it does not read these escapes in a `plugin` line either.
  The same holds for `implicit_prices`.
- `beancount.plugins.auto_accounts` with **no** configuration string is built. With one the reference fails
  to apply the plugin (an error on `<load>:0`), and ledgermill refuses it.
- Any other module is refused: `unsupported-syntax`, `details.construct = "plugin"`,
  `details.name` = the module as written (with or without a configuration string). The file lists no transaction.
  The reference runs the plugin, so a journal that is fine for it may be rejected or changed by it. A module
  that does not exist is an error of the reference on `<load>:0` (`plugin-unknown-module`); ledgermill cannot
  tell it from one it does not know, and refuses it by name as well.
- A refused plugin line next to an `auto_accounts` one still refuses the file.
- Naming `auto_accounts` twice runs it once. The line may stand anywhere in the file: first, last, between
  directives. A plugin belongs to its file only, as options do (several-files.md): in a manifest of several
  Beancount files, the others do not get it.

### What `auto_accounts` does

It adds an `open` for every account that is used and has no `open` directive.

- **Which accounts.** One is *used* by a posting of a transaction, by the account **and** the source of a `pad`,
  and by a `balance`, a `note`, a `document` or a `close`. A `price`, `event`, `commodity`, `query` and `custom`
  name no account for it (an account in the values of a `custom`, or in the value of a metadata key, is not opened).
- **Which date.** The date of the account's earliest use, whichever line comes first in the file. The `open` has
  no currency constraint and the default booking method, so any currency is fine in it and a lot is booked as
  everywhere.
- **Which accounts it leaves alone.** One with an `open` anywhere in the file, whatever its date. A use before
  that `open` stays the error it is without the plugin (`account-not-open`, `reason: not-yet-open`), and an
  explicit `open` keeps its currencies (`currency-not-allowed`). A duplicate `open` is still a duplicate.
- **`close`.** A `close` of an account that has no `open` opens it on the day of the `close` (or earlier, if it
  was used before), so there is no *close without open* error. A use after the `close` is still an error
  (`reason: closed`), and so is a second `close` (`duplicate-close`). An account whose first use is after
  its `close` is opened on the day of the `close` and used when closed.
- **Transactions booking rejects.** The plugin does not see them: an account used only by such a transaction
  is not opened by it, and a later use of that account decides the date. (No visible error depends on the
  difference, since the rejected transaction is not checked; the `open` dates are not output.)
- **What goes away, what stays.** The errors that need an `open` and no more go away: `account-not-open` with
  `reason: never-opened` (for a transaction, a `pad`, a `note`, a `document`), a `balance` on an account never
  opened, and a `close` without an `open`. Every other error stays, and `check()` gives them in the same
  order as without the plugin (check-order.md): the plugin adds nothing to the steps. The first error of a
  journal is the first of what is left.
- Pads, balances and booking work as without the plugin: the padding transaction is made as usual, its two
  accounts opened on the pad's date.
- **Output.** `transactions()` and `postings()` are the same as for the journal with the `open` lines written
  by hand. The `open` directives the plugin adds are not in any list ledgermill returns.

## Probe

Accepted, each with no `open`, or only some:

- `probes/auto-accounts.beancount`: two accounts, no `open`.
- `probes/auto-accounts-directives.beancount`: a `balance`, a `pad` and its balance, a `note`, a `document`,
  a `close`; a `price`, `event`, `commodity`, `custom` and a metadata value that name accounts and open none.
- `probes/auto-accounts-first-use.beancount`: the use that comes first in time is written second.
- `probes/auto-accounts-mixed.beancount`: one `open` by hand, the plugin line at the end.
- `probes/auto-accounts-close-ok.beancount`: a `close` of an account used before, and of one never used.
- `probes/auto-accounts-twice.beancount`: the plugin three times, with a tab and a comment.

Rejected, with the error that stays:

- `probes/auto-accounts-open-after-use.beancount`: an `open` dated after a use.
- `probes/auto-accounts-use-after-close.beancount`: a use after a `close`, a double `close`.
- `probes/auto-accounts-currencies.beancount`: a currency constraint of an explicit `open`, a duplicate `open`.
- `probes/auto-accounts-booking-error.beancount`: a transaction booking rejects.
- `probes/auto-accounts-errors-order.beancount`: eight errors of different steps next to accounts that would
  be unknown, the plugin line last.

Refused (`probes/refused/`, not run by the harness): `plugin-check-commodity.beancount`,
`plugin-auto-accounts-config.beancount`, `plugin-unknown-module.beancount`,
`plugin-escaped-name.beancount`, `plugin-escaped-control.beancount`.

Command:

```sh
spec/beancount/show-opens.sh spec/beancount/probes/auto-accounts-*.beancount spec/beancount/probes/refused/plugin-*.beancount
spec/beancount/show.sh spec/beancount/probes/auto-accounts.beancount
```

`show-opens.sh` prints what `bean-check` says (one line per error, a Python traceback replaced by a marker)
and the `open` and `close` directives `bean-query`'s `PRINT` lists, the plugin's among them. The
`order` probe's full list was read with `bean-check` itself, its lines in the order given below.

## Output

auto-accounts-first-use (the earliest use, not the first written):

```text
bean-check: (no output)
2026-01-02 open Assets:B
2026-01-02 open Assets:C
2026-01-05 open Assets:A
```

auto-accounts-directives (`Assets:Meta` and `Expenses:Custom` are not opened):

```text
bean-check: (no output)
2026-03-01 open Assets:Bal
2026-03-02 open Assets:PadTo
2026-03-02 open Equity:PadFrom
2026-03-04 open Assets:Note
2026-03-05 open Assets:Doc
2026-03-10 open Assets:Closed
2026-03-10 close Assets:Closed
2026-03-11 open Assets:A
2026-03-11 open Assets:B
```

auto-accounts-mixed (the explicit `open` is left as it is, with its currency):

```text
2026-01-01 open Assets:A                                        EUR
2026-03-02 open Assets:B
```

auto-accounts-close-ok:

```text
2026-03-01 open Assets:A
2026-03-01 open Assets:B
2026-03-05 close Assets:A
2026-03-06 open Assets:NeverUsed
2026-03-06 close Assets:NeverUsed
```

auto-accounts-open-after-use (no `open` added for `Assets:B`, which has one):

```text
bean-check: auto-accounts-open-after-use.beancount:5: Invalid reference to inactive account 'Assets:B'
```

auto-accounts-use-after-close (`Assets:NoUse` is opened on 03-20, its close day):

```text
bean-check: auto-accounts-use-after-close.beancount:8: Duplicate close directive for Assets:A
bean-check: auto-accounts-use-after-close.beancount:10: Invalid reference to inactive account 'Assets:A'
bean-check: auto-accounts-use-after-close.beancount:15: Invalid reference to inactive account 'Assets:NoUse'
```

auto-accounts-currencies (`Assets:Auto` takes USD and a `balance` in JPY):

```text
bean-check: auto-accounts-currencies.beancount:11: Invalid currency 'JPY' for Balance directive: 
bean-check: auto-accounts-currencies.beancount:5: Duplicate open directive for Assets:Dup
bean-check: auto-accounts-currencies.beancount:7: Invalid currency USD for account 'Assets:Eur'
```

auto-accounts-booking-error (only the second transaction's accounts are opened):

```text
bean-check: auto-accounts-booking-error.beancount:5: You may not have more than one auto-posting per currency
2026-03-05 open Assets:Q
2026-03-05 open Assets:R
```

auto-accounts-errors-order (booking, unused pad, balance, duplicate open, inactive account, currency,
duplicate commodity, unbalanced; the unknown `Assets:Never`, `Assets:Nowhere` and the rest give no error):

```text
bean-check: auto-accounts-errors-order.beancount:31: You may not have more than one auto-posting per currency
bean-check: auto-accounts-errors-order.beancount:26: Unused Pad entry
bean-check: auto-accounts-errors-order.beancount:23: Balance failed for 'Assets:Opening': expected 9 EUR != accumulated -3 EUR (12 too little)
bean-check: auto-accounts-errors-order.beancount:3: Duplicate open directive for Assets:Dup
bean-check: auto-accounts-errors-order.beancount:19: Invalid reference to inactive account 'Assets:LaterStill'
bean-check: auto-accounts-errors-order.beancount:15: Invalid currency USD for account 'Assets:Eur'
bean-check: auto-accounts-errors-order.beancount:9: Duplicate commodity directives for 'ZZZ'
bean-check: auto-accounts-errors-order.beancount:11: Transaction does not balance: (-1 EUR)
```

The refused ones:

```text
plugin-check-commodity:        bean-check: (no output)   [the plugin runs; with a commodity directive it has nothing to say]
plugin-auto-accounts-config:   bean-check: <load>:0: Error applying plugin "beancount.plugins.auto_accounts": [traceback hidden]
                               bean-check: plugin-auto-accounts-config.beancount:3: Invalid reference to unknown account 'Assets:B'
                               bean-check: plugin-auto-accounts-config.beancount:3: Invalid reference to unknown account 'Assets:A'
plugin-unknown-module:         bean-check: <load>:0: Error importing "no.such.module": [traceback hidden]
plugin-escaped-name:           bean-check: (no output)   [the plugin runs: both accounts are opened]
plugin-escaped-control:        bean-check: <load>:0: Error importing "beancount.plugins.auto_accou<line feed>ts": [traceback hidden]
```

The same journal with other plugin lines, for the forms (a line `plugin LINE` in front of a journal with
two opens): `plugin beancount.plugins.auto_accounts` is `Invalid token` on line 1; a number as the second argument, a
third string and `plugin "x" foo` are syntax errors on line 1; `plugin` alone is a syntax error on line 2; an
indented plugin line is a syntax error (`unexpected INDENT`); `plugin ""` is the import error `Empty module name`.

## What ledgermill does

Built. `check()` and the lists are those of the reference for all of the above, compared by the harness on the
probes. The plugin line is read before the parser runs (as `option` and `include` are), so `details.name` is the
module as written between the quotes. A malformed plugin line is left to the parser, which gives a `parse-error`
on the line, as the reference does, except a keyword alone, which is refused as `plugin` with no name (the
reference's line is the next one). An indented `plugin` line, which the reference rejects, is not noticed by the
parser we build on; that is a gap of every indented directive, not of this plugin.
