# Header and flags

Reference: Beancount 3.2.3 / beanquery 0.2.0 (`bean-query --version`: `beanquery 0.2.0, beancount 3.2.3`), 2026-10-21.

## Rule

The transaction header is the date, one flag, then up to two strings. Tags, links, comments and
metadata may follow; ledgermill does not report them (a Beancount `comment` is always empty).

Flag: `*` and `!` mean cleared and pending (ledgermill's `status`). Any other flag still reads as
a transaction and is kept as written: `#`, `&`, `?`, `%`, and a single capital letter such as `P`
or `S` followed by a space. The word `txn` reads as `*`. ledgermill returns the flag as written in
`flag`, and the status unmarked for every flag other than `*` and `!`.

Strings: one string is the narration. Two strings are the payee, then the narration. A header
with no string has an empty narration. An empty payee (`""`) is no payee. A payee followed by an
empty narration keeps the payee. Inside a string `\"` is a quote and `\\` a backslash. A string
may run over several lines, and spaces at its ends are kept (`show.sh` trims line ends and
squeezes spaces around commas; the raw query below shows `  padded  ` unchanged). Non-ASCII text is
kept. The reported line of a transaction is the line its header starts on; its postings have their
own lines.

Posting flag: a posting may start with one flag character before the account (`!`, `*`, `#`,
`P`, `&`). It is reported on that posting alone, also on a posting without an amount, and is
empty otherwise. It does not change the transaction's flag.


## Probe

`probes/header-flags.beancount`, `probes/header-strings.beancount`, `probes/posting-flags.beancount`

Command:

```sh
spec/beancount/show.sh spec/beancount/probes/header-flags.beancount
spec/beancount/show.sh spec/beancount/probes/header-strings.beancount
spec/beancount/show.sh spec/beancount/probes/posting-flags.beancount
```

Raw check of the padded narration (`cat -e` shows the line ends):

```sh
bean-query -f csv spec/beancount/probes/header-strings.beancount "SELECT lineno, payee, narration FROM entries WHERE type = 'transaction'" | tail -1 | cat -e
```

`show.sh` prints one row per posting: `id` (the first 6 characters of its transaction's id), account, posting flag, number, currency, cost number, cost currency, cost date, cost label, price number, price currency, line (here without the file name). A transaction row is `id,type,line,date,flag,payee,narration`. `bean-check:` is the first line it printed, if any.

## Output

header-flags:

```text
bean-check: (no output, exit 0)
7fa0f2,transaction,4,2026-01-02,*,,star
afefe9,transaction,7,2026-01-02,!,,bang
14f37a,transaction,10,2026-01-02,*,,txn word reads as a star
e5e825,transaction,13,2026-01-02,#,,hash
073744,transaction,16,2026-01-02,P,,letter P
0595b1,transaction,19,2026-01-02,S,,letter S
793c46,transaction,22,2026-01-02,&,,ampersand
c744fb,transaction,25,2026-01-02,?,,question mark
251b8c,transaction,28,2026-01-02,%,,percent
```

header-strings:

```text
bean-check: (no output, exit 0)
803531,transaction,4,2026-01-02,*,"Payee,with comma","Narration with a "" quote"
532531,transaction,7,2026-01-03,*,,only narration
658bba,transaction,10,2026-01-04,*,payee only has an empty narration,
d62c71,transaction,13,2026-01-05,*,,empty payee
54adc9,transaction,16,2026-01-06,*,,
36b906,transaction,19,2026-01-07,*,,with tag and link
25f9a7,transaction,22,2026-01-08,*,,"escaped "" quote and \ backslash"
185e12,transaction,25,2026-01-09,*,é ünï 💶,日本語
a9fd3f,transaction,28,2026-01-10,*,"two
lines","narration over
three lines"
ff3384,transaction,33,2026-01-11,*,,padded
803531,Expenses:Food,,1,EUR,,,,,,,5
803531,Assets:Bank,,-1,EUR,,,,,,,6
532531,Expenses:Food,,1,EUR,,,,,,,8
532531,Assets:Bank,,-1,EUR,,,,,,,9
658bba,Expenses:Food,,1,EUR,,,,,,,11
658bba,Assets:Bank,,-1,EUR,,,,,,,12
d62c71,Expenses:Food,,1,EUR,,,,,,,14
d62c71,Assets:Bank,,-1,EUR,,,,,,,15
54adc9,Expenses:Food,,1,EUR,,,,,,,17
54adc9,Assets:Bank,,-1,EUR,,,,,,,18
36b906,Expenses:Food,,1,EUR,,,,,,,20
36b906,Assets:Bank,,-1,EUR,,,,,,,21
25f9a7,Expenses:Food,,1,EUR,,,,,,,23
25f9a7,Assets:Bank,,-1,EUR,,,,,,,24
185e12,Expenses:Food,,1,EUR,,,,,,,26
185e12,Assets:Bank,,-1,EUR,,,,,,,27
a9fd3f,Expenses:Food,,1,EUR,,,,,,,31
a9fd3f,Assets:Bank,,-1,EUR,,,,,,,32
ff3384,Expenses:Food,,1,EUR,,,,,,,34
ff3384,Assets:Bank,,-1,EUR,,,,,,,35
```

posting-flags:

```text
bean-check: (no output, exit 0)
90cfcc,transaction,4,2026-01-02,*,,a flag on one posting
ee0ab6,transaction,7,2026-01-03,*,,flags of several kinds
6e129b,transaction,12,2026-01-04,*,,an elided posting with a flag
90cfcc,Assets:A,!,1,EUR,,,,,,,5
90cfcc,Assets:B,,-1,EUR,,,,,,,6
ee0ab6,Assets:A,*,1,EUR,,,,,,,8
ee0ab6,Assets:B,#,1,EUR,,,,,,,9
ee0ab6,Assets:A,P,1,EUR,,,,,,,10
ee0ab6,Assets:B,&,-3,EUR,,,,,,,11
6e129b,Assets:A,,2,EUR,,,,,,,13
6e129b,Assets:B,!,-2,EUR,,,,,,,14
```

Raw check:

```text
33,,  padded  ^M$
```
