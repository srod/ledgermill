#!/bin/sh
# Print what bean-check and bean-query report for the journals given, in the two queries the
# comparison harness uses (harness/lib.ts): the transactions, then their postings.
# usage: spec/beancount/show.sh FILE...
# Cleaned for reading only: CRs and the padding spaces are dropped, the id is cut to 6 characters
# (postings share their transaction's id) and the directory is cut from the file name.
# A multi-line field shows over several lines.
ENTRIES="SELECT id, type, filename, lineno, date, flag, payee, narration FROM entries WHERE type = 'transaction'"
POSTINGS="SELECT id, account, posting_flag, number, currency, cost_number, cost_currency, cost_date, cost_label, number(price), currency(price), filename, lineno FROM postings"
clean() {
  tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//; s/^([0-9a-f]{6})[0-9a-f]{26},/\1,/; s#[^,"]*/([^/,"]*\.beancount)#\1#'
}
for f in "$@"; do
  echo "== $f"
  echo "-- bean-check"
  bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
  echo "-- transactions: id,type,filename,lineno,date,flag,payee,narration"
  bean-query -f csv "$f" "$ENTRIES" 2>&1 | clean | tail -n +2
  echo "-- postings: id,account,posting_flag,number,currency,cost_number,cost_currency,cost_date,cost_label,price_number,price_currency,filename,lineno"
  bean-query -f csv "$f" "$POSTINGS" 2>&1 | clean | tail -n +2
done
