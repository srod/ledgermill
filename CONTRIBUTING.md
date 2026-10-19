# Contributing

Thank you for helping. Two things matter more here than in most projects: the clean-room rules,
and evidence for every behaviour.

## Clean-room rules

ledgermill reproduces what hledger and Beancount compute, without their code. That only holds
if everyone follows these rules.

### Never

1. **Never read the source code** of hledger, Beancount, rustledger or Ledger: not their
   repositories, not vendored copies, not patches or diffs against them, not stack traces that
   show their source lines, not AI output that quotes them. This includes browsing them on the
   web.
2. **Never copy a file** from those projects: no test journal, no script, no manual page, no
   `--help` text, no screenshot.
3. **Never decompile or disassemble** their programs.
4. **Never paste their text** into our code, tests, docs, comments or commit messages.

### Allowed

5. **Run the reference tools** (hledger, `bean-check`, `bean-query`, `bean-format`) on journals
   we wrote, and record what they print. That is how we learn behaviour.
6. **Read the hledger manual and the Beancount documentation** to learn rules, option names and
   field names. Do not copy their wording, tables or examples, and do not turn their examples
   into test journals: write your own, even to test the same rule.
7. **Match the observable interface**: option names, field names, file syntax.
8. **Read permissively licensed code**, such as the `beancount-parser-lima` and
   `limabean` crates.

### How work is done

9. **One spec note per behaviour** in `spec/`: the rule in our own words, a journal we wrote,
   the exact command, the reference tool's version and its output. Code is written from spec
   notes.
10. **Record exposure.** If you have read any of the forbidden material, or do so by accident,
    add it to [EXPOSURE.md](EXPOSURE.md) in the same pull request, and do not write code in that
    area. If code may have been copied, it is deleted and rewritten from the spec notes by
    someone (or an agent session) without that exposure.
11. **Don't mirror their structure.** Our module layout and type names follow our own design,
    not theirs.

## AI coding agents

Agents follow the same rules, and a few more:

- No upstream source or manual text in a prompt. A link to a manual page is fine.
- Every commit containing code an agent wrote carries an `Assisted-by: <tool and model>`
  trailer.
- A human reviews every agent change before it is merged.
- Before each release, a script compares our source with the pinned upstream sources and the
  score is recorded. Nobody reads the upstream tree to do this.

## Sign-off (DCO)

Every commit must be signed off (`git commit -s`), certifying the
[Developer Certificate of Origin 1.1](https://developercertificate.org/): you wrote the change or
have the right to submit it under the project's licences. There is no contributor licence
agreement.

## Pull requests

- Explain what changes and which spec notes back it.
- Run `cargo test` and the comparison harness (see `harness/README.md`) before opening.
- Fill in the clean-room line in the pull request template.
