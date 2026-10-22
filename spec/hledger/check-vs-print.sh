#!/bin/sh
# Compare `hledger check` with `hledger print -O json` on the corpus: the verdict, and for a rejected
# journal the first error's file, line and (for a parse error) column. Run from the repository root.
# usage: spec/hledger/check-vs-print.sh [FILE | FILE+FILE...]
#   `A+B` reads the files A and B together, in that order (two -f options).
#   Default: every journal of harness/journals/ and spec/hledger/probes/ (not probes/refused/), and
#   the multi-file cases of harness/compare.ts (its `pairs`, `fixturePairs` and `specPairs` lists,
#   read from that file).
# Output: one line `NAME | check | print` for a journal where they differ, then a summary line
#   `N compared, N differ`. Nothing differs when the summary says `0 differ`. With -v, every journal
#   is printed, with `same` instead of the verdicts.
# CHECK_ARGS adds arguments to `check` (for example `CHECK_ARGS=-s`, or `CHECK_ARGS="accounts"`), to see
# what a check that is not a default one would reject.
# The verdict of a command is `accepts` or `rejects FILE:LINE[:COLUMN]`, the position that hledger
# prints on the first line of its error.
verbose=0
if [ "$1" = "-v" ]; then
  verbose=1
  shift
fi

verdict() { # ARGS... : run `hledger --no-conf ARGS` and print the verdict
  if err=$(hledger --no-conf "$@" 2>&1 >/dev/null); then
    echo accepts
  else
    where=$(printf '%s\n' "$err" | head -1 | sed -nE 's#^hledger: Error: ([^ ]*/)?([^/ ]+):([0-9]+)(-[0-9]+)?(:[0-9]+)?:$#\2:\3\5#p')
    echo "rejects ${where:-$(printf '%s\n' "$err" | head -1)}"
  fi
}

if [ $# -eq 0 ]; then
  list=/tmp/check-vs-print.$$
  ls harness/journals/prototype/*.journal harness/journals/app/*.journal spec/hledger/probes/*.journal >"$list"
  # the pair lists of compare.ts: one line `a b` per pair, in a directory by list name
  awk '
    /^const pairs/ { dir = "harness/journals/prototype"; on = 1; next }
    /^const fixturePairs/ { dir = "harness/journals/app"; on = 1; next }
    /^const specPairs/ { dir = "spec/hledger/probes"; on = 1; next }
    on && /^\];/ { on = 0 }
    on && match($0, /\["[^"]+", "[^"]+"\]/) {
      s = substr($0, RSTART, RLENGTH); gsub(/[\["\],]/, "", s); split(s, p, " ")
      print dir "/" p[1] ".journal+" dir "/" p[2] ".journal"
    }' harness/compare.ts >>"$list"
  set -- $(cat "$list")
  rm -f "$list"
fi

total=0
differ=0
for spec in "$@"; do
  args=""
  name=""
  for f in $(echo "$spec" | tr '+' ' '); do
    args="$args -f $f"
    name="$name${name:++}$(basename "$f" .journal)"
  done
  # shellcheck disable=SC2086
  c=$(verdict $args check $CHECK_ARGS)
  # shellcheck disable=SC2086
  p=$(verdict $args print -O json)
  total=$((total + 1))
  if [ "$c" != "$p" ]; then
    differ=$((differ + 1))
    echo "$name | $c | $p"
  elif [ $verbose -eq 1 ]; then
    echo "$name | same | $c"
  fi
done
echo "$total compared, $differ differ"
