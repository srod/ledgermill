# hledger `check` and `print`: the same verdict

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

## Rule

`check()` of ledgermill has to say what hledger says about a journal: what the harness compares is
`hledger print -O json` (`harness/lib.ts`), so this note records that `hledger check`, the command a
user runs to ask "is my journal valid?", gives the same answer, and which of its checks are on by default.

**A plain `hledger check` runs three checks**: the file parses, every transaction balances (with the
amounts hledger can infer), and every balance assertion and assignment holds. These are the errors
`print` reports too, from the same code that reads the journal: on every journal of the corpus, `check`
and `print` accept or reject together, and a rejected journal has the same first error, file and line
(and column for a parse error). So the first error of `check` is the first error of `print`; there is
nothing to compare twice.

**The other checks are not on by default.** A journal that passes a plain `check` may still be rejected
by one of them, asked for by name (`hledger check accounts`) or with `-s`/`--strict`, which adds the
accounts and commodities checks. They look at things the journal can leave out (declarations), not at
the transactions' numbers, and ledgermill does not report them:

- `accounts`: an account used in a posting must be declared with `account` (`-s`);
- `commodities`: a commodity used in an amount must be declared with `commodity` (`-s`);
- `payees`: the payee (the text before `|` in a description) must be declared with `payee`;
- `tags`: a tag name used in a comment must be declared with `tag`;
- `ordereddates`: transactions must be in date order (within a file);
- `uniqueleafnames`: no two accounts share their last name part;
- `recentassertions`: an account with an assertion must have one in the last week of the day the command
  runs (the verdict depends on today's date);
- `balanced`: accepts the journal below, which has a posting with no amount; it rejects an unbalanced
  transaction, as reading the journal does;
- `parseable`, `autobalanced`, `assertions`: the three default ones, by name.

In the journal below a plain `check` and `print` accept; each of those checks, asked for by name,
rejects it except the default ones and `balanced`. The position is the first error's line.

| Command | Verdict |
|---|---|
| `check` | accepts |
| `check parseable`, `check autobalanced`, `check assertions`, `check balanced` | accept |
| `check -s` | rejects, line 5 (the commodity `EUR`) |
| `check accounts` | rejects, line 5 (`expenses:food:milk`) |
| `check commodities` | rejects, line 5 (`EUR`) |
| `check payees` | rejects, line 4 (the payee `shop`) |
| `check tags` | rejects, line 4 (the tag `flavour`) |
| `check ordereddates` | rejects, line 8 (the second transaction is older) |
| `check uniqueleafnames` | rejects, line 9 (the leaf `milk` twice) |
| `check recentassertions` | rejects, line 6 (`assets:cash`: its newest assertion is from January) |

An error of a default check on a journal with a declaration problem comes first, as always: with the
journal unbalanced, `check -s` and `check accounts` report the unbalanced transaction, not the missing
declaration, because the journal is not read at all when it is invalid.

Over the whole corpus (the journals of `harness/journals/`, the probes of `spec/hledger/probes/`, and the
39 multi-file cases of `harness/compare.ts`: 1088 in all) `check` and `print` never differ. With `-s`,
768 of the 1088 journals are rejected by `check` and accepted by `print`, nearly all for an undeclared
account or commodity: these are not errors `check()` reports, and the harness does not compare them.

## Probe

`probes/check-default-passes.journal`, and the whole corpus for the comparison.

Command, from the repository root:

```sh
spec/hledger/check-vs-print.sh                 # prints the journals where check and print differ, then a count
CHECK_ARGS=-s spec/hledger/check-vs-print.sh   # the same with check -s
f=spec/hledger/probes/check-default-passes.journal
for c in "" -s accounts commodities payees tags ordereddates uniqueleafnames recentassertions; do
  hledger --no-conf -f $f check $c; echo "check $c: $?"
done
hledger --no-conf -f $f print -O json > /dev/null; echo "print: $?"
```

## Output

```text
1088 compared, 0 differ
```

(the same command with `CHECK_ARGS=-s`: `768 differ`, the first of them
`combined-lot-price | rejects combined-lot-price.journal:1 | accepts`.)

check-default-passes: exit status 0 for `check`, `check parseable`, `check autobalanced`,
`check assertions`, `check balanced` and for `print`; 1 for the others, which print the first of
these lines each (an error `check-default-passes.journal:LINE:` and the rule broken, for example `Strict
account checking is enabled, and account "expenses:food:milk" has not been declared`).
