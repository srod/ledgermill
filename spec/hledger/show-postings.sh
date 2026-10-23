#!/bin/sh
# Print the rows of `hledger print --location -O csv`, one per line, for the journals given.
# usage: spec/hledger/show-postings.sh FILE...   (several files = several -f, in order)
# A row is "IDX DATE STATUS | \"DESCRIPTION\" | FILE:FIRST-LAST | ACCOUNT | AMOUNT COMMODITY", then
# " | pstatus=S" and " | pcomment=TEXT" when the posting has them, and " | comment=TEXT" when
# the transaction comment holds more than the location line (newlines written \n). IDX is the
# `txnidx` column. FILE is shown with the current directory, then `spec/hledger/probes/`, cut from
# the front when it starts with them. Needs python3 (the CSV has multi-line fields).
set -e
files=""
for a in "$@"; do files="$files -f $a"; done
# shellcheck disable=SC2086
hledger --no-conf $files print --location -O csv | python3 -c '
import csv, os, sys
cwd = os.getcwd() + "/"
for r in csv.DictReader(sys.stdin):
    lines = r["comment"].split("\n")
    loc, rest = lines[-1], lines[:-1]
    assert loc.startswith("location: "), loc
    path, span = loc[len("location: "):].rsplit(":", 1)
    if path.startswith(cwd):
        path = path[len(cwd):]
    if path.startswith("spec/hledger/probes/"):
        path = path[len("spec/hledger/probes/"):]
    head = (r["txnidx"] + " " + r["date"] + " " + r["status"]).rstrip()
    out = [head, "\"" + r["description"] + "\"", path + ":" + span, r["account"], (r["amount"] + " " + r["commodity"]).rstrip()]
    if r["posting-status"]:
        out.append("pstatus=" + r["posting-status"])
    if r["posting-comment"]:
        out.append("pcomment=" + r["posting-comment"].replace("\n", "\\n"))
    if rest:
        out.append("comment=" + "\\n".join(rest))
    print(" | ".join(out))
'
