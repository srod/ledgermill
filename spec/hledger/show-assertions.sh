#!/bin/sh
# Print the balance assertions hledger read, one per line: where (line:column of the first `=`),
# the account, the asserted amount (and its price when it has one), and the two flags. For an
# assignment the amount the posting got (after the assignment was worked out) is listed too.
# usage: spec/hledger/show-assertions.sh FILE...   (several files = several -f, in order)
set -e
args=""
for f in "$@"; do args="$args -f $f"; done
# shellcheck disable=SC2086
hledger --no-conf $args print -O json | jq -c '
  def q: [.acommodity, .aquantity.decimalMantissa, .aquantity.decimalPlaces]
    + (if .acost then [[(if .acost.tag == "UnitCost" then "@" else "@@" end)] + (.acost.contents | [.acommodity, .aquantity.decimalMantissa, .aquantity.decimalPlaces])] else [] end);
  .[] | .tpostings[] | select(.pbalanceassertion) | .pbalanceassertion as $b
  | {at: "\($b.baposition.sourceLine):\($b.baposition.sourceColumn)", account: .paccount,
     asserted: ($b.baamount | q), total: $b.batotal, inclusive: $b.bainclusive,
     amounts: [.pamount[] | q]}'
