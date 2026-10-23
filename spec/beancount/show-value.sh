#!/bin/sh
# Print what bean-check and bean-query report for one journal, in the value query Plainledger runs
# (harness/lib.ts has the same text; its posting columns are cut here to the useful ones): every
# posting valued in V at the prices of a date, then the cost query, which does not depend on either.
# usage: spec/beancount/show-value.sh FILE V YYYY MM DD
# Cleaned for reading only: CRs and the padding spaces are dropped.
f="$1"; V="$2"; Y="$3"; M="$4"; D="$5"
CONVERT="convert(position, \"$V\", date($Y, $M, $D))"
VALUE="SELECT account, number, currency, number($CONVERT) AS amount, currency($CONVERT) AS commodity, lineno FROM postings"
clean() {
  tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//'
}
echo "== $f V=$V on $Y-$M-$D"
echo "-- bean-check"
bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
echo "-- value: account,number,currency,amount,commodity,lineno"
bean-query -f csv "$f" "$VALUE" 2>&1 | clean | tail -n +2
