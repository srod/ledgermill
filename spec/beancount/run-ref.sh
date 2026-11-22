#!/bin/sh
# Run a Beancount reference CLI (bean-check, bean-query, bean-format) without ever printing the
# frames of a Python traceback, which quote upstream source lines (CONTRIBUTING.md clean-room
# rules). Standard output and standard error stay separate, each passed through the same filter,
# and the exit status is the tool's. The last line of a traceback, the exception that was raised,
# is kept; so is everything outside a traceback.
# usage: spec/beancount/run-ref.sh <bean-check|bean-query|bean-format> [args...]
set -eu

if [ $# -lt 1 ]; then
  echo "usage: $0 <bean-check|bean-query|bean-format> [args...]" >&2
  exit 2
fi

tool=$1
shift
case "$tool" in
  bean-check|bean-query|bean-format) ;;
  *)
    echo "run-ref.sh: refused tool '$tool' (expected bean-check, bean-query, or bean-format)" >&2
    exit 2
    ;;
esac

out=$(mktemp)
err=$(mktemp)
trap 'rm -f "$out" "$err"' EXIT
set +e
# A Python warning prints the source line that raised it, with no traceback around it.
PYTHONWARNINGS=ignore "$tool" "$@" >"$out" 2>"$err"
status=$?
set -e

# A traceback runs from its header to the first line that does not start with a blank: the
# exception. awk exits 10 when it dropped frames.
stripped=0
strip() {
  rc=0
  awk '
    /^Traceback \(most recent call last\):$/ { tracing = 1; dropped = 1; next }
    tracing && /^[^[:space:]]/ { print; tracing = 0; next }
    !tracing { print }
    END { exit dropped ? 10 : 0 }
  ' "$1" || rc=$?
  if [ "$rc" -eq 10 ]; then
    stripped=1
  elif [ "$rc" -ne 0 ]; then
    exit "$rc"
  fi
}

strip "$out"
strip "$err" >&2
if [ "$stripped" -eq 1 ]; then
  echo "[run-ref.sh: Python traceback frames stripped to preserve clean-room isolation; exit $status]" >&2
fi
exit "$status"
