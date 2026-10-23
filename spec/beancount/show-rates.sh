#!/bin/sh
# Print what bean-check and bean-query report for one journal, as the rate from commodity FROM to commodity TO on each
# of the days 2026-01-01 to 2026-01-06: Plainledger's value query (harness/lib.ts) for the one posting of one unit of
# FROM, whose account is Assets:FROM, with the number of convert(position, "TO", date) on six dates in one row.
# usage: spec/beancount/show-rates.sh FILE FROM TO...   (several TO: one row each)
# Cleaned for reading only: CRs and the padding spaces are dropped.
f="$1"; from="$2"; shift 2
clean() {
  tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//'
}
echo "== $f"
echo "-- bean-check"
bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
echo "-- rate: FROM>TO, then the number of convert(position, TO, date) on 2026-01-01 to 2026-01-06 (the commodity is TO unless said)"
for to in "$@"; do
  cols=""
  for d in 1 2 3 4 5 6; do
    cols="$cols, number(convert(position, \"$to\", date(2026, 01, 0$d))) AS n$d, currency(convert(position, \"$to\", date(2026, 01, 0$d))) AS c$d"
  done
  bean-query -f csv "$f" "SELECT account$cols FROM postings WHERE account ~ '^Assets:$from\$'" 2>&1 | clean | tail -n +2 |
    awk -F, -v from="$from" -v to="$to" '{ out = from ">" to ":"; for (i = 2; i <= 12; i += 2) { out = out "  " ($(i+1) == to ? $i : "-") } print out }'
done
