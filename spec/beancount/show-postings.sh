#!/bin/sh
# Print what bean-check and bean-query report for the journals given, in the postings query the
# comparison harness uses (harness/lib.ts): one row per posting, in the order bean-query gives.
# usage: spec/beancount/show-postings.sh FILE...
# Cleaned for reading only: CRs and the padding spaces are dropped and the directory is cut from the
# file name. A multi-line field shows over several lines.
QUERY="SELECT date, flag, payee, narration, account, number, currency, filename, lineno ORDER BY date, filename, lineno"
for f in "$@"; do
  echo "== $f"
  echo "-- bean-check"
  bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
  echo "-- postings: date,flag,payee,narration,account,number,currency,filename,lineno"
  bean-query -f csv "$f" "$QUERY" 2>&1 | tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//; s#[^,"]*/([^/,"]*\.beancount)#\1#' | tail -n +2
done
