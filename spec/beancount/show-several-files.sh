#!/bin/sh
# Print what bean-check says about each file of probes/several-files/ alone and about the hand-made
# roots that `include` them, and the name_assets option each load ends with.
# usage: spec/beancount/show-several-files.sh
# Cleaned for reading only: the directory is cut from the file names.
cd "$(dirname "$0")/probes/several-files" || exit 1
PY="$(dirname "$(readlink "$(command -v bean-check)")")/python"
for f in open-a open-b include-open rename-a include-rename; do
  echo "== $f.beancount"
  out=$(bean-check "$f.beancount" 2>&1 | sed -E "s#[^ :]*/##" | grep -v '^ *$')
  echo "bean-check: ${out:-(no output)}"
  "$PY" -c "
import sys
from beancount import loader
entries, errors, options = loader.load_file(sys.argv[1])
print('name_assets:', options['name_assets'], '| entries:', len(entries), '| errors:', len(errors))
" "$f.beancount"
done
