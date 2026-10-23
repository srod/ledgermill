#!/bin/sh
# Print the P lines `hledger prices` lists, the way Plainledger reads them.
# usage: spec/hledger/show-prices.sh [--first] FILE...   (several files = several -f, in order)
# Run 1 is `hledger -f F... prices --color=n`. Run 2 adds `-c "1.000000000000000000 COMM"` for every
# commodity in the rate column of run 1 (full-precision numbers, no digit groups) and is what is
# printed. --first prints run 1 instead. The script stops with an error if run 2 lists other
# rows than run 1 (a different count, or another date or first commodity on a row).
# The rate column of a row is "RATE COMM" or "COMM RATE"; a quoted COMM keeps its quotes.
set -e
first=0
if [ "$1" = "--first" ]; then first=1; shift; fi
files=""
for a in "$@"; do files="$files -f $a"; done
# shellcheck disable=SC2086
run1=$(hledger --no-conf $files prices --color=n)
if [ "$first" = 1 ]; then printf '%s\n' "$run1"; exit 0; fi
# the rate column: drop "P DATE " and the first commodity (quoted or not)
rates=$(printf '%s\n' "$run1" | sed -E -e 's/^P [^ ]+ //' -e 's/^("[^"]*"|[^ ]*) ?//')
comms=$(printf '%s\n' "$rates" | while IFS= read -r r; do
  if printf '%s' "$r" | grep -Eq '^[-+0-9.,]'; then # rate first, commodity after
    printf '%s\n' "$r" | sed -E 's/^[-+0-9., ]*[0-9.,] ?//'
  else # commodity first, rate after
    printf '%s\n' "$r" | sed -E 's/ ?[-+]? ?[0-9][-+0-9.,eE ]*$//'
  fi
done | sort -u)
set --
OLDIFS=$IFS
IFS='
'
for c in $comms; do
  set -- "$@" -c "1.000000000000000000 $c"
done
# a row whose rate has no commodity gives an empty line (the loop skips it)
if printf '%s\n' "$comms" | grep -q '^$'; then set -- "$@" -c "1.000000000000000000"; fi
IFS=$OLDIFS
# shellcheck disable=SC2086
run2=$(hledger --no-conf $files prices --color=n "$@")
head2() { printf '%s\n' "$1" | sed -E 's/^(P [^ ]+ ("[^"]*"|[^ ]*)).*/\1/'; }
[ "$(head2 "$run1")" = "$(head2 "$run2")" ] || { echo "run 2 lists other rows than run 1" >&2; exit 1; }
printf '%s\n' "$run2"
