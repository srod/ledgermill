#!/bin/sh
# Print hledger's verdict on period expressions, as the header of a periodic rule (`~ EXPRESSION`).
# usage: spec/hledger/period-verdicts.sh FILE      one expression per line (`-` reads stdin)
# Output, one line per expression: `ok<TAB>EXPRESSION` or `err:COLUMN<TAB>EXPRESSION`, where COLUMN
# is the column hledger reports on line 1 of the probe it writes (the `~` is column 1).
# The expressions are written to a scratch journal, so they must have no tab and no leading blank.
# Check mode: the corpus spec/hledger/period-forms.txt holds the output of this script. To see that
# hledger still agrees, from the repository root (prints nothing when it does):
#   cut -f2- spec/hledger/period-forms.txt | spec/hledger/period-verdicts.sh - | diff - spec/hledger/period-forms.txt
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cat "$1" | while IFS= read -r expr; do
  printf '~ %s\n    (x)  1\n' "$expr" >"$tmp/p.journal"
  if hledger --no-conf -f "$tmp/p.journal" print >/dev/null 2>"$tmp/err"; then
    printf 'ok\t%s\n' "$expr"
  else
    col=$(head -1 "$tmp/err" | sed -nE 's#^hledger: Error: .*p\.journal:1:([0-9]+):$#\1#p')
    printf 'err:%s\t%s\n' "${col:-?}" "$expr"
  fi
done
