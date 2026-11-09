#!/bin/sh
# Print what bean-check says and what bean-query's prices table and PRINT list for the journals
# given, with a Python traceback replaced by a marker (the same filter as show-opens.sh).
# usage: spec/beancount/show-implicit.sh FILE...
# Cleaned for reading only: CRs and the padding spaces are dropped and the directory is cut from
# the file name.
hide() {
  awk '/Traceback/ { sub(/Traceback.*/, "[traceback hidden]"); print; skip = 1; next } /^[^ \t]/ { skip = 0 } /^$/ { skip = 0 } !skip'
}
clean() {
  tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//; s#[^,"]*/([^/,"]*\.beancount)#\1#'
}
# The rows of a query on stdout. Its stderr (load errors, a crash) goes through hide, and only the
# traceback marker is kept, so a crash is not taken for an empty table.
query() {
  err=$(mktemp)
  bean-query -f csv "$1" "$2" 2>"$err" | hide | clean | tail -n +2
  hide <"$err" | grep -F '[traceback hidden]'
  rm -f "$err"
}
PRICES="SELECT date, currency, number(amount), currency(amount) FROM prices"
LINES="SELECT date, lineno, filename FROM entries WHERE type = 'price'"
for f in "$@"; do
  echo "== $f"
  echo "-- bean-check"
  bean-check "$f" 2>&1 | hide | sed -E "s#[^ :]*/##" | grep -E '^[^ ]*:[0-9]+: |^<|hidden' || echo "(no output)"
  echo "-- prices: date,currency,number(amount),currency(amount)"
  query "$f" "$PRICES"
  echo "-- price entries: date,lineno,filename"
  query "$f" "$LINES"
done
