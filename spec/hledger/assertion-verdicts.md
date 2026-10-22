# hledger's verdict on the journals with assertions and assignments

Reference: hledger 1.52.4 (`hledger --version`: `hledger 1.52.4, mac-aarch64`), 2026-10-22.

Status: built.

## Rule

Which of the harness journals that have a balance assertion or assignment (the ones
`bun harness/compare.ts` used to skip with `unsupported: balance-assertion`) hledger accepts, and for the
rejected ones where its first error is, by file and line. ledgermill reads assertions: the
accepted ones are compared transaction by transaction and, for the rejected ones, our first error finding
must be on the same file and line as the first error of hledger (the harness does it). The rule for the line is in
[assertion-failures](assertion-failures.md): the posting's line for a failing assertion, the first
line of the transaction for an unbalanced one.

The corpus: 99 journals (`harness/journals/prototype/`, `harness/journals/app/`,
`spec/hledger/probes/posting-assertion.journal`) and 8 pairs of files read together.
**Accepted: 77** (72 journals, 5 pairs). **Rejected: 30** (27 journals, 3 pairs): 17 assertions
that fail (the list below, `assertion-failed`), 13 transactions that do not balance once the
assigned amounts are in (`unbalanced`). None is rejected for a parse error. The probes of the
assertion and assignment notes (`assert-*`, `assign-*`) are not in this table; the script lists
them too when run without arguments.

## Probe

All the journals of the corpus above, and the 8 pairs.

Command, from the repository root:

```sh
spec/hledger/assertion-verdicts.sh
```

It prints one line per journal or pair (`A+B` is `-f A -f B`): `NAME | accepts |` or
`NAME | rejects | FILE:LINE[:COLUMN] | KIND`. This note leaves out the lines of the `assert-` and
`assign-` probes.

## Output

Accepted journals: `proj004-balance-assertion`, `d4`, `fuzz46-282`, `h2`, `h4`, `h56`, `ia31-incl`, `ia32-incl`, `ia33-incl`, `ia34-incl`, `ia35-incl`, `ia36-incl`, `lb11-basis`, `lb12-basis`, `lb13-basis`, `lb21-basis`, `lb22-basis`, `lb25-basis`, `lb3-basis`, `lb4-basis`, `lb41-basis`, `lb42-basis`, `lb43-basis`, `lb44-basis`, `lb45-basis`, `lb5-basis`, `lb6-basis`, `lb7-basis`, `m12`, `m1a`, `m3a`, `m4`, `m5`, `m6`, `m7`, `m9`, `pi61-infer`, `pi62-infer`, `pi63-infer`, `pi65-infer`, `pi72-infer`, `q1023`, `q1025`, `q1026`, `q1027`, `q11`, `q1114`, `q1201`, `q1202`, `q1203`, `q1204`, `q1205`, `q1413`, `q1514`, `q1606`, `q1811`, `q1812`, `q1813`, `q701`, `q702`, `v6`, `v7`, `v8`, `val-m7-assign`, `val-z26`, `x10`, `x11`, `x12`, `x13b`, `y6`, `y7`, `posting-assertion`

Pairs: `m1a+m1b`, `m3a+m3b`, `h1+h2`, `h3+h4`, `x13+x13b`

Rejected (the position is `FILE:LINE:COLUMN` for an assertion and `FILE:LINE` of the transaction's
first line for an unbalanced one:

| Journal | First error | Kind | Cause |
|---|---|---|---|
| `fuzz43-172` | `fuzz43-172.journal:5:17` | assertion-failed | `a -176.8 USD = -176.8 USD` after a posting of 1165 USD, 3 January vs 2 January: the balance is 988.2 |
| `fuzz45-368` | `fuzz45-368.journal:11:14` | assertion-failed | `a 0.108 Y = 0.108 Y` (line 11) on an account that holds 954 Y from the transaction written first, same date |
| `h6` | `h6.journal:2:13` | assertion-failed | `(e) 0 EUR = 1 EUR`: the 1 EUR is in h5, another file |
| `ia37-incl` | `ia37-incl.journal:3:13` | assertion-failed | `(e) 1 EUR == 2.4 EUR` while the account holds 2.44 EUR |
| `lb23-basis` | `lb23-basis.journal:4` | unbalanced | `a == 3 USD` also zeroes `1 X {2 EUR}`: `a` holds two amounts, so no price is inferred; 3 USD and -1 X |
| `lb24-basis` | `lb24-basis.journal:4` | unbalanced | the same, with `b -3 USD` beside it (all postings of one sign) |
| `m11` | `m11.journal:3:17` | assertion-failed | `(f) 1.004 EUR = 1.00 EUR`: exact comparison, the balance is 1.004 |
| `m2a` | `m2a.journal:2:13` | assertion-failed | `(e) 1 EUR = 2 EUR`: the other 1 EUR is in m1b, another file |
| `m8` | `m8.journal:3:13` | assertion-failed | `(e) 1 USD == 1 USD` while the account also holds 1 EUR |
| `pi51-infer` | `pi51-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c 0 USD`) |
| `pi52-infer` | `pi52-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c 0 Z`) |
| `pi53-infer` | `pi53-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c 0 USD` first) |
| `pi54-infer` | `pi54-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c -1 USD`) |
| `pi55-infer` | `pi55-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c 1 X`) |
| `pi71-infer` | `pi71-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c 1 X`, `d -6 Y`) |
| `pi73-infer` | `pi73-infer.journal:4` | unbalanced | `a == 3 USD` also zeroes the old 1 X: `a` holds two amounts, so no price is inferred and the sum is not zero (with `c 1 X`, `d -6 Y`, `c` first) |
| `pi91-infer` | `pi91-infer.journal:3` | unbalanced | `a == 3 USD` also zeroes the `(a) 1 Z` of the day before: 3 USD, -1 Z, `c -6 Y`: three commodities |
| `pi92-infer` | `pi92-infer.journal:3` | unbalanced | `a == 3 Y` also zeroes the `(a) 1 Z` of the day before: 3 Y, -1 Z, `c -6 USD`: three commodities |
| `q1020` | `q1020.journal:2:11` | assertion-failed | `e 1 EUR = 1.004 EUR` |
| `q1021` | `q1021.journal:2:15` | assertion-failed | `e 1.004 EUR = 1 EUR` |
| `q1022` | `q1022.journal:2:15` | assertion-failed | `e 1.004 EUR = 1.00 EUR` |
| `q1024` | `q1024.journal:3:11` | assertion-failed | `e 1 USD == 1 USD` while the account also holds 1 EUR |
| `q1028` | `q1028.journal:2:7` | assertion-failed | `e 0 = 3 EUR`: the account holds no EUR (the 0 has no commodity) |
| `q517` | `q517.journal:1` | unbalanced | `b = 5 EUR` assigns 5 EUR beside `e 1 EUR`: both positive |
| `q518` | `q518.journal:4` | unbalanced | `b = -3 EUR` after `b -1 EUR = -1 EUR` assigns -2 EUR: 1 EUR and -2 EUR |
| `v9` | `v9.journal:4:13` | assertion-failed | `(e) 0 EUR = 11 EUR` after a posting of 10 EUR |
| `y8b` | `y8b.journal:2:13` | assertion-failed | `(e) 0 EUR = 1 EUR`: the 1 EUR is in y8a, another file |
| `m2a+m1b` | `m2a.journal:2:13` | assertion-failed | `(e) 1 EUR = 2 EUR`: the other 1 EUR is in m1b, another file |
| `h5+h6` | `h6.journal:2:13` | assertion-failed | `(e) 0 EUR = 1 EUR`: the 1 EUR is in h5, another file |
| `y8a+y8b` | `y8b.journal:2:13` | assertion-failed | `(e) 0 EUR = 1 EUR`: the 1 EUR is in y8a, another file |
