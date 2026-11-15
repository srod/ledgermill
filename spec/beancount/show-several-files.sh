#!/bin/sh
# Print what bean-check says about the files of probes/several-files/ read as ONE ledger: the first file
# of a set is the root, the others are `include`d at its end, in the order given (what a manifest of
# several Beancount files stands for). Then, for a few sets, the postings, prices or the option that the
# load ends with. A last part runs hand-made roots that only include, and the name_assets option each
# load ends with.
# usage: spec/beancount/show-several-files.sh
# Cleaned for reading only: the directory is cut from the file names.
cd "$(dirname "$0")/probes/several-files" || exit 1
HERE=$(pwd)
PY="$(dirname "$(readlink "$(command -v bean-check)")")/python"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
clean() { sed -E "s#[^ :,]*/##" | grep -v '^ *$'; }

# root FILE...: the set as one file in $TMP: the first file, then an include line for each other.
root() {
  name=$(basename "$1")
  cp "$1" "$TMP/$name"
  printf '\n' >> "$TMP/$name"
  shift
  for f in "$@"; do printf 'include "%s/%s"\n' "$HERE" "$f" >> "$TMP/$name"; done
  echo "$TMP/$name"
}

check() { # check FILE...
  echo "== $*"
  out=$(bean-check "$(root "$@")" 2>&1 | clean)
  echo "bean-check: ${out:-(no output)}"
}

postings() { # postings FILE...
  echo "== $* (postings)"
  bean-query -f csv "$(root "$@")" \
    "SELECT date, filename, lineno, account, number, narration FROM postings ORDER BY date, filename, lineno" 2>&1 \
    | tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//' | clean
}

prices() { # prices FILE...: the prices table, then the date, line and file of each price entry
  echo "== $* (prices)"
  set -- "$(root "$@")"
  bean-query -f csv "$1" "SELECT date, currency, number(amount), currency(amount) FROM prices" 2>&1 \
    | tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//' | clean
  bean-query -f csv "$1" "SELECT date, lineno, filename FROM entries WHERE type = 'price'" 2>&1 \
    | tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//' | clean
}

sums() { # sums FILE...: the holdings queries' units of Assets:Sum (show-holdings.sh has the full texts)
  echo "== $* (sums)"
  set -- "$(root "$@")"
  bean-check "$1" 2>&1 | clean
  bean-query -f csv "$1" "SELECT account, currency, str(sum(units(position))) AS units, str(sum(convert(position, 'EUR'))) AS value WHERE account ~ 'Assets' GROUP BY account, currency" 2>&1 \
    | tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//' | clean
}

echo "##### accounts, balances, pads and the order of the walk"
check a.beancount b.beancount
check bal-a.beancount bal-b.beancount
check bal-a.beancount bal-wrong-b.beancount
check open-a.beancount open-b.beancount
check order-open-a.beancount order-open-b.beancount
check order-open-b.beancount order-open-a.beancount
check pad-a.beancount pad-b.beancount
postings pad-a.beancount pad-b.beancount

echo "##### the order of the sums: by date and line, not by file"
sums sum-order-a.beancount sum-order-b.beancount
sums sum-order-b.beancount sum-order-a.beancount

echo "##### options and plugins: the root's apply, an included file's do not"
check fifo-a.beancount sale-b.beancount
check sale-b.beancount fifo-a.beancount
check auto-a.beancount auto-b.beancount
check auto-b.beancount auto-a.beancount
prices implicit-a.beancount implicit-b.beancount
prices implicit-b.beancount implicit-a.beancount

echo "##### files that hold nothing, and settings in a file that is not the root"
check a.beancount empty.beancount b.beancount
check empty.beancount a.beancount b.beancount
check a.beancount options-only.beancount b.beancount
check options-only.beancount a.beancount b.beancount
check lots-a.beancount settings-only.beancount
check settings-only.beancount lots-a.beancount
prices settings-only.beancount lots-a.beancount

echo "##### roots that only include"
for f in include-open include-rename; do
  echo "== $f.beancount"
  out=$(bean-check "$f.beancount" 2>&1 | clean)
  echo "bean-check: ${out:-(no output)}"
  "$PY" -c "
import sys
from beancount import loader
entries, errors, options = loader.load_file(sys.argv[1])
print('name_assets:', options['name_assets'], '| entries:', len(entries), '| errors:', len(errors))
" "$f.beancount"
done
