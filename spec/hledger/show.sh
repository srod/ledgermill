#!/bin/sh
# Print the fields ledgermill compares, one transaction per line, for the journals given.
# usage: spec/hledger/show.sh [OPTION...] FILE...   (several files = several -f, in order)
# An argument that starts with "-" is passed to hledger as it is, after "print" (for example
# "--cost" or "--value=2026-01-01,EUR"); every other argument is a journal file.
# An amount is [commodity, mantissa, places], followed by its price as
# ["@"|"@@", commodity, mantissa, places] and its lot as {cost, date, label} when it has them.
set -e
files=""
n=$#
while [ "$n" -gt 0 ]; do
  a=$1
  shift
  n=$((n - 1))
  case "$a" in
    -*) set -- "$@" "$a" ;; # kept, in order, for hledger (it may hold a space)
    *) files="$files -f $a" ;;
  esac
done
# shellcheck disable=SC2086
hledger --no-conf $files print -O json "$@" | jq -c '
  def q: [.acommodity, .aquantity.decimalMantissa, .aquantity.decimalPlaces];
  .[] | {
  date: .tdate, status: .tstatus, code: .tcode, description: .tdescription, comment: .tcomment,
  line: .tsourcepos[0].sourceLine,
  postings: [.tpostings[] | {account: .paccount, type: .ptype, comment: .pcomment,
    amounts: [.pamount[] | q
      + (if .acost then [[(if .acost.tag == "UnitCost" then "@" else "@@" end)] + (.acost.contents | q)] else [] end)
      + (if .acostbasis then [{cost: (.acostbasis.cbCost | if . then q else null end),
                               date: .acostbasis.cbDate, label: .acostbasis.cbLabel}] else [] end)]}]}'
