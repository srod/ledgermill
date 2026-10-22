#!/bin/sh
# Print bean-check's verdict on every Beancount journal of the corpus, and for a rejected one the
# first error it printed (its file name, line and message). Run from the repository root.
# usage: spec/beancount/verdicts.sh [FILE...]    (default: the whole corpus)
if [ $# -eq 0 ]; then
  set -- harness/journals/app/*.beancount spec/beancount/probes/*.beancount \
    spec/beancount/probes/refused/*.beancount
fi
for f in "$@"; do
  err=$(bean-check "$f" 2>&1 >/dev/null)
  if [ $? -eq 0 ]; then
    echo "$f | accepts |"
  else
    first=$(printf '%s\n' "$err" | grep -m1 -E '^[^ ]*:[0-9]+: |^<load>:[0-9]+: ' | sed -E 's#^[^ :]*/##')
    echo "$f | rejects | $first"
  fi
done
