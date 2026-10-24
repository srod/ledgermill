#!/bin/sh
# For each journal: run bean-format, then say whether it only changed whitespace, whether
# bean-check/bean-query read the formatted copy as the same ledger (the postings and prices queries of
# show-postings.sh and show-prices.sh, errors included), and how many lines it changed.
# usage: spec/beancount/format-same.sh FILE...
here=$(dirname "$0")
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
for f in "$@"; do
  out="$dir/$(basename "$f")"
  if ! bean-format "$f" > "$out" 2>/dev/null; then
    echo "$f | bean-format fails"
    continue
  fi
  if [ "$(tr -d ' \t\r\n' < "$f")" = "$(tr -d ' \t\r\n' < "$out")" ]; then ws=yes; else ws=NO; fi
  before=$("$here/show-postings.sh" "$f"; "$here/show-prices.sh" "$f")
  after=$("$here/show-postings.sh" "$out"; "$here/show-prices.sh" "$out")
  # the files have the same base name; the directory (in the `==` headers and in some messages) is masked
  src=$(cd "$(dirname "$f")" && pwd)
  mask() { grep -v '^== ' | sed -e "s#$1#DIR#g"; }
  if [ "$(printf '%s\n' "$before" | mask "$src")" = "$(printf '%s\n' "$after" | mask "$dir")" ]; then same=yes; else same=NO; fi
  changed=$(diff "$f" "$out" | grep -c '^>')
  echo "$f | whitespace only: $ws | same ledger: $same | lines changed: $changed"
done
