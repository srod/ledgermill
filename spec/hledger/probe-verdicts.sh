#!/bin/sh
# Print hledger's verdict on probe journals, and for a rejected one where its first error is.
# Run from the repository root.
# usage: spec/hledger/probe-verdicts.sh [FILE | FILE+FILE...]
#   `A+B` reads the files A and B together, in that order (two -f options).
#   Default: every spec/hledger/probes/rule-*.journal, and three pairs of rule probes.
# Output, one line per journal or pair: `NAME | accepts | N transactions` or
#   `NAME | rejects | FILE:LINE[:COLUMN] | KIND`.
# KIND is `parse` (a syntax error: a column is given), `unbalanced`, `two-elided`,
# `assertion-failed`, `other`, or the text of a one-line error that names no position.
# The line is the first line of the transaction for `unbalanced` and `two-elided`.
if [ $# -eq 0 ]; then
  p=spec/hledger/probes
  # shellcheck disable=SC2046 # the probe names have no spaces
  set -- $(ls $p/rule-*.journal)
  set -- "$@" $p/rule-file-a.journal+$p/rule-file-b.journal $p/rule-file-b.journal+$p/rule-file-a.journal \
    $p/rule-file-c.journal+$p/rule-file-a.journal
fi
for spec in "$@"; do
  args=""
  name=""
  for f in $(echo "$spec" | tr '+' ' '); do
    args="$args -f $f"
    name="$name${name:++}$(basename "$f" .journal)"
  done
  # shellcheck disable=SC2086
  if out=$(hledger --no-conf $args print -O json 2>/tmp/probe-verdicts.$$); then
    echo "$name | accepts | $(printf '%s' "$out" | jq length) transactions"
    continue
  fi
  err=$(cat /tmp/probe-verdicts.$$)
  first=$(printf '%s\n' "$err" | head -1)
  where=$(printf '%s\n' "$first" | sed -nE 's#^hledger: Error: ([^ ]*/)?([^/ ]+):([0-9]+)(-[0-9]+)?(:[0-9]+)?:$#\2:\3\5#p')
  # the message follows the quoted lines of the file, so look for its own sentences
  if printf '%s\n' "$err" | grep -q '^the asserted balance is'; then kind=assertion-failed
  elif printf '%s\n' "$err" | grep -q "^There can't be more than one real posting with no amount"; then kind=two-elided
  elif printf '%s\n' "$err" | grep -qE '^This (multi-commodity )?transaction is unbalanced'; then kind=unbalanced
  elif [ -z "$where" ]; then kind=$(printf '%s\n' "$first" | sed 's/^hledger: Error: //')
  elif printf '%s\n' "$first" | grep -qE ':[0-9]+-[0-9]+:$'; then kind=other
  else kind=parse
  fi
  echo "$name | rejects | ${where:--} | $kind"
done
rm -f /tmp/probe-verdicts.$$
