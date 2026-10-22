#!/bin/sh
# Print hledger's verdict on journal lines that are directives (or anything at the margin).
# usage: spec/hledger/directive-verdicts.sh FILE     one line per input line (`-` reads stdin)
# Each input line is put first in a journal, followed by a small valid transaction, so an error on
# the line itself is reported on line 1. `\n` in an input line is a new line and `\t` a tab (so
# an indented line can follow a directive); a lone `\` has no other meaning.
# Output, one line per input line: `ok<TAB>LINE` or `err:LINE:COLUMN<TAB>LINE`. An error that is
# not on line 1 (a following line was rejected) shows its own line number. An error that names no
# position (a missing file) shows `err:?`.
# Check mode: spec/hledger/directive-forms.txt holds the output of this script. From the repository
# root, prints nothing when hledger still agrees:
#   cut -f2- spec/hledger/directive-forms.txt | spec/hledger/directive-verdicts.sh - | diff - spec/hledger/directive-forms.txt
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cat "$1" | while IFS= read -r line; do
  printf '%b\n\n2026-01-02 t\n    a  1\n    b\n' "$line" >"$tmp/d.journal"
  if hledger --no-conf -f "$tmp/d.journal" print >/dev/null 2>"$tmp/err"; then
    printf 'ok\t%s\n' "$line"
  else
    pos=$(head -1 "$tmp/err" | sed -nE 's#^hledger: Error: .*d\.journal:([0-9]+(-[0-9]+)?(:[0-9]+)?):$#\1#p')
    printf 'err:%s\t%s\n' "${pos:-?}" "$line"
  fi
done
