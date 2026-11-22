#!/bin/sh
# Run a Beancount reference CLI (bean-check, bean-query, bean-format) without ever printing the
# frames of a Python traceback, which quote upstream source lines (CONTRIBUTING.md clean-room
# rules). Standard output and standard error stay separate, each passed through the same filter,
# and the exit status is the tool's. The last line of a traceback, the exception that was raised,
# is kept; so is everything outside a traceback. A warning's message is kept and the source line
# under it dropped. `bean-python` is the Python that runs bean-check (from its shebang), for the
# few helpers that call the loader themselves.
# usage: spec/beancount/run-ref.sh <bean-check|bean-query|bean-format|bean-python> [args...]
set -eu

if [ $# -lt 1 ]; then
  echo "usage: $0 <bean-check|bean-query|bean-format|bean-python> [args...]" >&2
  exit 2
fi

tool=$1
shift
case "$tool" in
  bean-check|bean-query|bean-format) ;;
  bean-python)
    interp=$(sed -n '1s/^#![[:space:]]*//p' "$(command -v bean-check)")
    case "$interp" in */env\ *) interp=${interp#* } ;; esac
    tool=${interp%% *}
    ;;
  *)
    echo "run-ref.sh: refused tool '$tool' (expected bean-check, bean-query, bean-format or bean-python)" >&2
    exit 2
    ;;
esac

out=$(mktemp)
err=$(mktemp)
trap 'rm -f "$out" "$err"' EXIT
set +e
"$tool" "$@" >"$out" 2>"$err"
status=$?
set -e

# A traceback runs from its header to the first line that does not start with a blank: the
# exception. An exception group's header and frames are indented (`  + Exception Group Traceback`,
# `  | ...`), so its whole report is dropped. A warning (`FILE:LINE: SomeWarning: message`) quotes
# its source line on the indented line under it; PYTHONWARNINGS can't silence it, as bean-check runs
# Python with -E. awk exits 10 when it dropped anything.
stripped=0
strip() {
  rc=0
  awk '
    /^[[:space:]+|]*(Exception Group )?Traceback \(most recent call last\):$/ { tracing = 1; dropped = 1; next }
    tracing && /^[^[:space:]]/ { print; tracing = 0; next }
    warned && /^[[:space:]]/ { warned = 0; dropped = 1; next }
    { warned = 0 }
    !tracing && /^[^[:space:]].*:[0-9]+: [A-Za-z]*Warning: / { print; warned = 1; next }
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
  echo "[run-ref.sh: Python source lines stripped to preserve clean-room isolation; exit $status]" >&2
fi
exit "$status"
