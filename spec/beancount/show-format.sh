#!/bin/sh
# Print what bean-format does to the journals given: a column ruler, then the lines it changed, as it
# wrote them, each with its line number and a `|` where the text starts (so column 0 of the text is
# the column after the `|`). Tabs and trailing blanks stay visible: a tab shows as `^I` and the end of
# a line as `$`.
# usage: [BEAN_FORMAT_ARGS='-w 30'] spec/beancount/show-format.sh FILE...
# BEAN_FORMAT_ARGS holds options for bean-format (-w, -W, -c).
for f in "$@"; do
  echo "== $f"
  out=$(mktemp)
  if ! bean-format $BEAN_FORMAT_ARGS "$f" > "$out" 2>/dev/null; then
    echo "bean-format failed"
    rm -f "$out"
    continue
  fi
  echo "    |0         1         2         3         4         5         6"
  echo "    |0123456789012345678901234567890123456789012345678901234567890123"
  # line N of the output is line N of the input (bean-format keeps the line count); print the ones that differ
  awk 'NR==FNR { a[FNR] = $0; next } { gsub(/\r/, "", a[FNR]); if ($0 != a[FNR]) printf "%4d|%s\n", FNR, $0 }' "$f" "$out" | cat -vet
  rm -f "$out"
done
