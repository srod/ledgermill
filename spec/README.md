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
  line. Postings are `[commodity, decimalMantissa, decimalPlaces]` triples.
- `hledger/probes/` – the probe journals.
- `hledger/prototype-observations.md` – the notes of our earlier TypeScript prototype, kept as a
  starting point. They are our own text, written from running hledger; they cover far more than
  this slice.

## Index (hledger 1.52.4)

| Note | Behaviour |
|---|---|
| [amount-forms](hledger/amount-forms.md) | where the commodity goes, signs, spaces, symbol characters |
| [decimal-places](hledger/decimal-places.md) | quantities keep their written places; limits |
| [number-forms-refused](hledger/number-forms-refused.md) | number forms hledger accepts that ledgermill refuses |
| [transaction-header](hledger/transaction-header.md) | status, description, comment on the header line, date checks |
| [comments](hledger/comments.md) | where comments attach, trimming, what ends a transaction |
| [source-lines](hledger/source-lines.md) | the reported line, CRLF, BOM, tabs, missing final newline |
| [posting-lines](hledger/posting-lines.md) | account/amount separator, elided amounts, markers |
| [balancing](hledger/balancing.md) | what must sum to zero, single posting, no postings |
| [ordering](hledger/ordering.md) | output order, several files |
| [big-numbers](hledger/big-numbers.md) | JSON form of large mantissas |

## ledgermill findings in this slice

`unsupported-syntax` (error): the line uses something this slice does not read. `details.construct`
names it (`directive`, `periodic-or-auto-transaction`, `date`, `header-syntax`, `transaction-code`,
`control-character`, `indented-line`, `elided-amount`, `account-name`, `amount`,
`multiple-commodities`, `number-too-large`). A transaction with a refused line is not returned.

`unbalanced-transaction` (error): the postings of a transaction do not sum to zero. hledger fails
the whole command here; ledgermill reports the finding and still returns the transaction.
