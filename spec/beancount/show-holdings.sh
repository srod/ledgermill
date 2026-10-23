#!/bin/sh
# Print what bean-check and bean-query report for one journal, in the units and holdings queries
# Plainledger runs (harness/lib.ts has the same texts), valued in V, as of D when given.
# usage: spec/beancount/show-holdings.sh FILE V [DATE [ACCOUNT-REGEX]]
# DATE may be - for none. With a regex, only the accounts it matches are listed.
# Cleaned for reading only: CRs and the padding spaces are dropped. The row order is the order
# bean-query gives, which this note set does not rely on.
f="$1"; V="$2"; D="$3"; ONLY="$4"
[ "$D" = "-" ] && D=""
WHERE=""
[ -n "$D" ] && WHERE="WHERE date <= $D"
if [ -n "$ONLY" ]; then
  [ -n "$WHERE" ] && WHERE="$WHERE AND account ~ '$ONLY'" || WHERE="WHERE account ~ '$ONLY'"
fi
UNITS="SELECT account, currency, str(sum(units(position))) AS units $WHERE GROUP BY account, currency"
HOLDINGS="SELECT account, currency, str(sum(units(position))) AS units, str(sum(cost(position))) AS cost_basis, str(sum(convert(cost(position), \"$V\"))) AS cost_basis_value, str(sum(convert(position, \"$V\"))) AS market_value $WHERE GROUP BY account, currency"
clean() {
  tr -d '\r' | sed -E 's/ +,/,/g; s/, +/,/g; s/ +$//'
}
echo "== $f V=$V${D:+ date <= $D}${ONLY:+ accounts $ONLY}"
echo "-- bean-check"
bean-check "$f" 2>&1 | sed -E "s#[^ :]*/##" || true
echo "-- units: account,currency,units"
bean-query -f csv "$f" "$UNITS" 2>&1 | clean | tail -n +2
echo "-- holdings: account,currency,units,cost_basis,cost_basis_value,market_value"
bean-query -f csv "$f" "$HOLDINGS" 2>&1 | clean | tail -n +2
