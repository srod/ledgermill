#!/bin/sh
# Print what bean-check says about each journal (one line per error, the copy of the directive after
# it dropped, a Python traceback replaced by a marker) and the `open` and `close` directives
# bean-query's PRINT lists, with the plugin's own ones among them.
# usage: spec/beancount/show-opens.sh FILE...
hide() {
  awk '/Traceback/ { sub(/Traceback.*/, "[traceback hidden]"); print; skip = 1; next } /^[^ \t]/ { skip = 0 } /^$/ { skip = 0 } !skip'
}
for f in "$@"; do
  echo "== $f"
  echo "-- bean-check"
  bean-check "$f" 2>&1 | hide | sed -E "s#[^ :]*/##" | grep -E '^[^ ]*:[0-9]+: |^<|^\[traceback hidden\]' || echo "(no output)"
  echo "-- open and close directives"
  bean-query "$f" "PRINT" 2>&1 | hide | grep -E '^[0-9-]+ (open|close) |^\[traceback hidden\]' || echo "(no output)"
done
