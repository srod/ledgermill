#!/bin/sh
# Print what bean-check and bean-query report for the journals given, in the cost query Plainledger
# runs (harness/lib.ts has the same text; its posting columns are cut here to the useful ones).
# usage: spec/beancount/show-cost.sh FILE...
# Cleaned for reading only: CRs and the padding spaces are dropped.
COST="SELECT account, number, currency, cost_number, cost_currency, number(cost(position)) AS amount, currency(cost(position)) AS commodity, lineno FROM postings"
for f in "$@"; do
  echo "== $f"
  echo "-- bean-check"
  bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
  echo "-- cost: account,number,currency,cost_number,cost_currency,amount,commodity,lineno"
  bean-query -f csv "$f" "$COST" 2>&1 | tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//' | tail -n +2
done
