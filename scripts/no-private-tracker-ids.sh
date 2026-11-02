#!/bin/sh
# ledgermill is public; ids from a private issue tracker (PLA- and a number) lead nowhere for its readers.
# With no argument: checks every tracked file. With a file (the commit-msg hook): checks that file.
pattern='PLA-[0-9]+'
if [ $# -gt 0 ]; then
  # What git drops from the message: the diff under the scissors line (commit.verbose) and # comments.
  if sed '/^# -\{24\} >8 -\{24\}$/,$d; /^#/d' "$1" | grep -nE "$pattern"; then
    echo "commit message names a private tracker id: say what changed, or link a GitHub issue (#12)" >&2
    exit 1
  fi
  exit 0
fi
if git grep -nE "$pattern"; then
  echo "tracked files name a private tracker id: say what changed, or link a GitHub issue (#12)" >&2
  exit 1
fi
