#!/bin/sh
# Print the amounts of each transaction in a short form, one transaction per line.
# usage: spec/hledger/show-amounts.sh [OPTION...] FILE...
# Arguments are read as in show.sh (an argument starting with "-" goes to hledger print).
# A transaction is "DATE  ACCOUNT: AMOUNT, AMOUNT | ACCOUNT: ...". An amount is COMMODITY, a
# space and the decimal number with exactly its places (so 2 places of zero show as 0.00),
# then " @ AMOUNT" or " @@ AMOUNT" for a price and " {AMOUNT}" for a lot cost ("{}" when the
# lot has none). A posting without an amount shows "-".
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
  def amt: .acommodity + " " + num;
  .[] | .tdate + "  " + ([.tpostings[] | .paccount + ": " + (
    if (.pamount | length) == 0 then "-" else
      ([.pamount[] | amt
        + (if .acost then (if .acost.tag == "UnitCost" then " @ " else " @@ " end) + (.acost.contents | amt) else "" end)
        + (if .acostbasis then " {" + (.acostbasis.cbCost | if . then amt else "" end) + "}" else "" end)] | join(", "))
    end)] | join(" | "))'
