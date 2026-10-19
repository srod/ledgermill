#!/bin/sh
# Print the fields ledgermill compares, one transaction per line, for the journals given.
# usage: spec/hledger/show.sh FILE...   (several files = several -f, in order)
set -e
args=""
for f in "$@"; do args="$args -f $f"; done
# shellcheck disable=SC2086
hledger $args print -O json | jq -c '.[] | {
  date: .tdate, status: .tstatus, code: .tcode, description: .tdescription, comment: .tcomment,
  line: .tsourcepos[0].sourceLine,
  postings: [.tpostings[] | {account: .paccount, type: .ptype, comment: .pcomment,
    amounts: [.pamount[] | [.acommodity, .aquantity.decimalMantissa, .aquantity.decimalPlaces]]}]}'
