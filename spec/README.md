# Spec notes

One note per behaviour of the reference tools. Code is written from these notes, not from the
reference tools' source (see [CONTRIBUTING.md](../CONTRIBUTING.md)).

Each note has:

- **Rule**: what the tool does, in our own words;
- **Probe**: the journal we wrote (under `probes/`), run unchanged;
- **Command**: exactly what was run;
- **Reference**: the tool and version;
- **Output**: the part of the output that shows the rule.

## Layout

- `hledger/` – notes on hledger. The commands use `hledger/show.sh`, which runs
  `hledger -f FILE print -O json` and keeps the fields ledgermill compares, one transaction per
  line. An amount is `[commodity, decimalMantissa, decimalPlaces]`, then its price as
  `["@"|"@@", commodity, mantissa, places]` and its lot as `{cost, date, label}` when it has them.
- `hledger/probes/` – the probe journals.
- `hledger/prototype-observations.md` – the notes of our earlier TypeScript prototype, kept as a
  starting point. They are our own text, written from running hledger.

## Index (hledger 1.52.4)

| Note | Behaviour |
|---|---|
| [amount-forms](hledger/amount-forms.md) | where the commodity goes, signs, spaces, symbol characters |
| [numbers](hledger/numbers.md) | decimal marks, digit groups, exponents, exactness, big values |
| [decimal-places](hledger/decimal-places.md) | quantities keep their written places; ten-place output |
| [directives-numbers](hledger/directives-numbers.md) | `decimal-mark`, `commodity`, `D`: which mark a lone separator is |
| [directives-other](hledger/directives-other.md) | `account`, `P`, `comment` blocks and the directives read and ignored |
| [aliases-and-parents](hledger/aliases-and-parents.md) | `alias`, regex aliases, `apply account` |
| [dates](hledger/dates.md) | date forms, `Y`, secondary dates |
| [transaction-header](hledger/transaction-header.md) | status, code, description, comment on the header line |
| [comments](hledger/comments.md) | where comments attach, trimming, what ends a transaction |
| [comment-dates](hledger/comment-dates.md) | `date:` tags and bracketed dates in posting comments |
| [source-lines](hledger/source-lines.md) | the reported line, CRLF, BOM, tabs, missing final newline |
| [posting-lines](hledger/posting-lines.md) | account/amount separator, elided amounts, markers |
| [postings](hledger/postings.md) | posting status, virtual and balanced virtual postings, account names |
| [prices-and-lots](hledger/prices-and-lots.md) | `@`, `@@`, `{}`, `{{}}`, `[date]`, `(label)` |
| [balancing](hledger/balancing.md) | groups, what must sum to zero, tolerance |
| [elided-amounts](hledger/elided-amounts.md) | what an amount-less posting gets, and its places |
| [inferred-prices](hledger/inferred-prices.md) | the price given when two commodities do not balance |
| [conversion](hledger/conversion.md) | conversion accounts, `type:` tags, pairs that balance by quantity |
| [ordering](hledger/ordering.md) | output order, several files |

## ledgermill findings

`parse-error` (error): hledger rejects this line too. ledgermill drops the transaction it belongs
to.

`unsupported-syntax` (error): the line uses something ledgermill does not read yet.
`details.construct` names it: `directive` (periodic and automated rules), `include`,
`balance-assertion`, `date-without-year`, `control-character`, `number-too-large`,
`conversion-postings`. A transaction with a refused line is not returned.

`unbalanced-transaction` (error): a group does not sum to zero (`details.group`,
`details.residual`), two postings of a group leave out their amount (`details.reason:
two-elided-amounts`), or a conversion pair carries a price (`conversion-posting-with-cost`).
hledger fails the whole command here; ledgermill reports the finding and still returns the
transaction.
