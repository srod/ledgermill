#!/bin/sh
# Print what bean-check and bean-query report for the journals given, in the prices query Plainledger
# runs (harness/lib.ts has the same text), then the file and line of each price entry, in the order
# the entries come.
# usage: spec/beancount/show-prices.sh FILE...
# Cleaned for reading only: CRs and the padding spaces are dropped and the directory is cut from the
# file name.
PRICES="SELECT date, currency, number(amount), currency(amount) FROM prices"
LINES="SELECT date, lineno, filename FROM entries WHERE type = 'price'"
clean() {
  tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//; s#[^,"]*/([^/,"]*\.beancount)#\1#'
}
for f in "$@"; do
  echo "== $f"
  echo "-- bean-check"
  bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
  echo "-- prices: date,currency,number(amount),currency(amount)"
  bean-query -f csv "$f" "$PRICES" 2>&1 | clean | tail -n +2
  echo "-- price entries: date,lineno,filename"
  bean-query -f csv "$f" "$LINES" 2>&1 | clean | tail -n +2
done
