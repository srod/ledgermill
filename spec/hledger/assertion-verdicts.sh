#!/bin/sh
# Print hledger's verdict on every journal of the corpus that has a balance assertion or assignment
# (and on the multi-file cases of harness/compare.ts that have one), and for a rejected one where
# the first error is and what kind it is. Run from the repository root.
# usage: spec/hledger/assertion-verdicts.sh [FILE | FILE+FILE...]
#   `A+B` reads the files A and B together, in that order (two -f options).
#   Default: every journal under harness/journals/ and spec/hledger/probes/ with a `=` after a
#   posting (a `{=` lot cost does not count), and the eight pairs below.
# Output: `NAME | accepts |` or `NAME | rejects | FILE:LINE[:COLUMN] | KIND`, where KIND is
# assertion-failed, unbalanced, parse, or the text of a one-line error that names no position.
# For assertion-failed, `: SCOPE; asserted A; calculated B` follows (hledger's own words, with the
# spaces squeezed): the commodity or commodities compared, and the two amounts.
if [ $# -eq 0 ]; then
  set -- $(grep -lE '^[ 	]+[^ 	;][^;{]*=' harness/journals/prototype/*.journal \
      harness/journals/app/*.journal spec/hledger/probes/*.journal | sort)
  p=harness/journals/prototype
  set -- "$@" $p/m1a.journal+$p/m1b.journal $p/m2a.journal+$p/m1b.journal \
    $p/m3a.journal+$p/m3b.journal $p/h1.journal+$p/h2.journal $p/h3.journal+$p/h4.journal \
    $p/h5.journal+$p/h6.journal $p/y8a.journal+$p/y8b.journal $p/x13.journal+$p/x13b.journal
fi
for spec in "$@"; do
  args=""
  name=""
  for f in $(echo "$spec" | tr '+' ' '); do
    args="$args -f $f"
    name="$name${name:++}$(basename "$f" .journal)"
  done
  # shellcheck disable=SC2086
  err=$(hledger --no-conf $args print 2>&1 >/dev/null)
  if [ $? -eq 0 ]; then
    echo "$name | accepts |"
    continue
  fi
  first=$(printf '%s\n' "$err" | head -1)
  where=$(printf '%s\n' "$first" | sed -nE 's#^hledger: Error: ([^ ]*/)?([^/ ]+):([0-9]+)(-[0-9]+)?(:[0-9]+)?:$#\2:\3\5#p')
  if printf '%s\n' "$err" | grep -q 'Balance assertion failed'; then
    scope=$(printf '%s\n' "$err" | sed -nE 's/^(In commodity [^ ]+|Across all commodities) at this point, (excluding|including) subaccounts.*/\1, \2 subaccounts/p')
    asserted=$(printf '%s\n' "$err" | sed -nE 's/^the asserted balance is: +//p')
    calculated=$(printf '%s\n' "$err" | sed -nE 's/^but the calculated balance is: +//p')
    kind="assertion-failed: $scope; asserted $asserted; calculated $calculated"
  elif printf '%s\n' "$err" | grep -q 'unbalanced'; then kind=unbalanced
  elif [ -z "$where" ]; then kind=$(printf '%s\n' "$first" | sed 's/^hledger: Error: //')
  else kind=parse
  fi
  echo "$name | rejects | ${where:--} | $kind"
done
