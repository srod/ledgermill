#!/bin/sh
# Run a Beancount reference CLI (bean-check, bean-query, bean-format) without ever printing
# Python tracebacks, which quote upstream source lines (CONTRIBUTING.md clean-room rules).
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

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
set +e
"$tool" "$@" >"$tmp" 2>&1
status=$?
set -e

if grep -q '^Traceback (most recent call last):' "$tmp"; then
  sed '/^Traceback (most recent call last):/,$d' "$tmp"
  echo "[run-ref.sh: Python traceback stripped to preserve clean-room isolation; exit $status]" >&2
else
  cat "$tmp"
fi
exit "$status"
