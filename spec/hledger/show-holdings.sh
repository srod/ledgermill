#!/bin/sh
# Print what `hledger balance --layout=tidy --real` gives, one account per line.
# usage: spec/hledger/show-holdings.sh [--csv] [OPTION...] FILE... [-- QUERY...]
# An argument that starts with "-" before the "--" goes to hledger (write --end=2026-08-01, not
# "-e 2026-08-01", as a bare word is a file; --cost and --value=end,EUR go the same way). The
# words after "--" are passed to hledger after its own "--", as Plainledger does (see holdings.md
# for what that "--" does to a query that has no account pattern).
#
# Default: the exact amounts, from `balance -O json` (the single-column report), one line per account:
#   ACCOUNT: COMMODITY NUMBER | COMMODITY NUMBER
# The amounts of one commodity are added up whatever their costs (JSON lists `10 X @ 2 EUR` and
# `5 X` apart), the number keeps the most places of the amounts added, and a zero amount is shown
# as COMMODITY 0 (JSON keeps it, CSV drops it).
# With --csv: the rows of `balance --layout=tidy -O csv`, one per line, as Plainledger reads them:
#   ACCOUNT  COMMODITY  VALUE  START..END
# VALUE is the CSV text (rounded to the commodity's display precision, with its decimal mark) and
# START..END the period hledger shows. COMMODITY is empty for a bare number.
set -e
csv=0
files=""
opts=""
query=""
in_query=0
nl='
'
for a in "$@"; do
  if [ "$in_query" = 1 ]; then
    query="$query$nl$a"
    continue
  fi
  case "$a" in
    --csv) csv=1 ;;
    --) in_query=1 ;;
    -*) opts="$opts$nl$a" ;;
    *) files="$files$nl$a" ;;
  esac
done
fmt=json
[ "$csv" = 1 ] && fmt=csv
IFS=$nl
set --
for f in $files; do set -- "$@" -f "$f"; done
set -- "$@" balance --layout=tidy -O "$fmt" --real
for o in $opts; do set -- "$@" "$o"; done
if [ -n "$query" ]; then
  set -- "$@" --
  for q in $query; do set -- "$@" "$q"; done
fi
unset IFS
if [ "$csv" = 1 ]; then
  hledger --no-conf "$@" | python3 -c '
import csv, sys
rows = list(csv.reader(sys.stdin))
for r in rows[1:]:
    print("%s  %s  %s  %s" % (r[0], r[4], r[5], r[1]))
'
else
  hledger --no-conf "$@" | python3 -c '
import json, sys
from decimal import Decimal
for name, _, _, amounts in json.load(sys.stdin)[0]:
    sums = {}
    for a in amounts:
        q = a["aquantity"]
        d = Decimal(q["decimalMantissa"]).scaleb(-q["decimalPlaces"])
        sums[a["acommodity"]] = sums.get(a["acommodity"], Decimal(0)) + d
    print(name + ": " + " | ".join("%s %s" % (c, format(v, "f")) for c, v in sums.items()))
'
fi
