#!/bin/sh
# Print the tags hledger puts on the postings of each transaction, one transaction per line.
# usage: spec/hledger/show-tags.sh [OPTION...] FILE...   (arguments as in show.sh)
# A posting is "ACCOUNT AMOUNT" followed by [conversion] when it has the tag _conversion-posting,
# and [cost] when it has the tag _cost-posting (hledger adds both when a transaction has
# conversion postings; `--cost` drops the first kind, see cost.md). An amount is the decimal number
# and its commodity; several amounts are joined by commas; no amount shows as "-".
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
hledger --no-conf $files print -O json "$@" | jq -r '
  def num: (.aquantity.decimalPlaces) as $p
    | (.aquantity.decimalMantissa | if . < 0 then -. else . end | tostring) as $m
    | (if .aquantity.decimalMantissa < 0 then "-" else "" end) as $s
    | if $p == 0 then $s + $m
      else ($m | if length <= $p then ("0" * ($p + 1 - length)) + . else . end) as $d
        | $s + $d[:($d | length) - $p] + "." + $d[($d | length) - $p:] end;
  def tags: ([.ptags[][0]] ) as $t
    | (if ($t | index("_conversion-posting")) then "[conversion]" else "" end)
      + (if ($t | index("_cost-posting")) then "[cost]" else "" end);
  .[] | .tdate + "  " + ([.tpostings[] | .paccount + " "
    + (if (.pamount | length) == 0 then "-" else ([.pamount[] | num + " " + .acommodity
        + (if .acost then (if .acost.tag == "UnitCost" then " @ " else " @@ " end) + (.acost.contents | num + " " + .acommodity) else "" end)] | join(",")) end)
    + (tags | if . == "" then "" else " " + . end)] | join(" | "))'
