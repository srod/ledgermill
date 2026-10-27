//! `Ledger::at_cost`: every cost replaced by an amount in the cost's commodity, and the
//! conversion postings dropped. See `spec/hledger/cost.md`.

use crate::balance::{
    Draft, conversion_exempt, conversion_pairs, pair_fits, priced, within_half_unit,
};
use crate::decimal::{DIVISION_PLACES, Decimal};
use crate::model::{Finding, Posting, Transaction};
use crate::posting::{CostKind, WrittenAmount};
use crate::problem::unsupported_finding;
use crate::valuation::{Term, merge};
use crate::{Exact, Ledger};

/// Which postings of a transaction the reference tool tags `_conversion-posting`.
pub(crate) struct Plan {
    pub tagged: Vec<bool>,
    /// The transaction is one `spec/hledger/cost.md` leaves open.
    pub refused: bool,
}

/// An amount on its way to the output of `--cost`: with a cost it becomes an amount in the
/// cost's commodity (no cost, no lot); without one it is left as it is, lot kept.
pub(crate) fn at_cost_term(a: &WrittenAmount) -> Term<'_> {
    match &a.cost {
        None => Term {
            commodity: a.commodity.as_str(),
            value: a.quantity.clone(),
            original: Some(a),
        },
        Some(cost) => Term {
            commodity: cost.commodity.as_str(),
            value: match cost.kind {
                CostKind::Unit => a
                    .quantity
                    .mul(&cost.value)
                    .rounded(DIVISION_PLACES)
                    .normalized(),
                CostKind::Total => cost.value.clone(),
            },
            original: None,
        },
    }
}

impl Ledger {
    /// The transactions as `hledger print --cost` gives them: each amount with a cost is replaced
    /// by an amount in the cost's commodity, without its cost or lot, and the conversion
    /// postings are dropped; same shape and order as [`Ledger::transactions`].
    ///
    /// A unit cost multiplies, a total cost is the amount; the result keeps its places, up to
    /// ten (rounded half to even beyond that), after trailing zeros of a product are dropped.
    /// A transaction left with no posting stays. The amounts of a posting that holds several (an
    /// elided or assigned amount) are added per commodity once converted, as for
    /// [`Ledger::value`].
    ///
    /// As [`Ledger::transactions`], a ledger with error findings answers for the transactions it
    /// lists: a transaction left out there is left out here (and `check()` says why).
    ///
    /// The conversion postings dropped are those tagged `_conversion-posting` that have no
    /// cost: by the reference tool when a transaction holds pairs of conversion-account
    /// postings, and by a `_conversion-posting:` tag in the posting's comment.
    ///
    /// # Errors
    ///
    /// A finding with code `unsupported-syntax` (and, as its details' `construct`,
    /// `cost-conversion-postings`) for the first transaction that has pairs of conversion
    /// postings together with several priced postings or with a balance assignment: the spec
    /// does not say which postings the reference drops there.
    ///
    /// Beancount (`cost(position)`: spec/beancount/cost-of-position.md): a posting with a cost becomes
    /// its units times the cost per unit, in the cost's currency, rounded to 28 digits and printed as
    /// `number()` does; the other postings stay whole. Every transaction is answered; the one error is
    /// the first `unsupported-syntax` finding of [`Ledger::check`], when the file had one.
    ///
    /// A set of several ledgers (several Beancount files, or hledger and Beancount; see
    /// [`load`](crate::load)) is answered by each ledger on its own and put in the order of
    /// [`Ledger::transactions`], so result `i` is for transaction `i`. The error is the first
    /// ledger's, ledgers being ordered by the position of their first file.
    pub fn at_cost(&self) -> Result<Vec<Transaction>, Finding> {
        if !self.parts.is_empty() {
            return self.ask_parts(Ledger::at_cost);
        }
        if self.beancount {
            return match self.beancount_refusal() {
                Some(refusal) => Err(refusal),
                None => Ok(crate::beancount::at_cost(&self.transactions)),
            };
        }
        let mut out = Vec::with_capacity(self.transactions.len());
        for (t, exact) in self.transactions.iter().zip(&self.exact) {
            if exact.cost_refused {
                return Err(unsupported_finding(
                    "cost-conversion-postings",
                    "Which postings --cost drops is not known for conversion postings with several priced postings or an assignment.",
                    &t.source.file,
                    t.source.line,
                ));
            }
            out.push(Transaction {
                postings: at_cost_postings(t, exact),
                ..t.clone()
            });
        }
        Ok(out)
    }
}

fn at_cost_postings(t: &Transaction, exact: &Exact) -> Vec<Posting> {
    t.postings
        .iter()
        .zip(&exact.postings)
        .filter(|(_, e)| !(e.conversion_tagged && e.amounts.iter().all(|a| a.cost.is_none())))
        .map(|(p, e)| Posting {
            amounts: merge(e.amounts.iter().map(at_cost_term)),
            ..p.clone()
        })
        .collect()
}

/// The number a posting stands for when two postings are compared: its written quantity, or
/// zero with no places when it has none.
fn number(p: &Draft) -> Decimal {
    p.amounts
        .as_deref()
        .and_then(<[WrittenAmount]>::first)
        .map_or_else(Decimal::zero, |a| a.quantity.clone())
}

/// Work out which postings get the tag. `drafts` are as written, before balancing: elided
/// amounts are `None` and no price is inferred yet. `assignment`: a posting works out its
/// amount from an assertion.
pub(crate) fn plan(drafts: &[Draft], assignment: bool) -> Plan {
    let untagged = || vec![false; drafts.len()];
    let pairs = conversion_pairs(drafts);
    if pairs.is_empty() {
        return Plan {
            tagged: untagged(),
            refused: false,
        };
    }
    let priced_at: Vec<usize> = (0..drafts.len())
        .filter(|&k| drafts.get(k).is_some_and(priced))
        .collect();
    // The rules were checked with one priced posting and no assignment.
    if priced_at.len() > 1 || assignment {
        return Plan {
            tagged: untagged(),
            refused: true,
        };
    }
    // A shape that cannot be balanced is reported as such, and has nothing to convert.
    let Ok(exempt) = conversion_exempt(drafts) else {
        return Plan {
            tagged: untagged(),
            refused: false,
        };
    };
    let is_conv = |i: usize| drafts.get(i).is_some_and(|p| p.conversion_account);
    let one = |i: usize| match drafts.get(i).and_then(|p| p.amounts.as_deref()) {
        Some([a]) => Some(a),
        _ => None,
    };
    // Which pairs get the tag: all of them, or only those that fit the priced posting.
    let tag_pair: Vec<bool> = match priced_at.first() {
        Some(&k) => {
            let run = (0..k).rev().take_while(|&i| is_conv(i)).count();
            let by_quantity = exempt.get(k).copied().unwrap_or(false);
            if run % 2 == 1 && run == k {
                // An odd run opens the transaction: balanced by quantity, the pairs are tagged;
                // not by quantity, nothing is.
                vec![by_quantity; pairs.len()]
            } else {
                // Behind an odd run that does not open the transaction, the priced posting
                // can repeat the anchor too. A pair that makes the priced posting balance by
                // its quantity is tagged whatever the anchor; the others follow the anchor.
                let repeater = (run % 2 == 1).then_some(k);
                let all = first_free_is_unique(drafts, repeater);
                let fits = |q: usize| {
                    let (Some(a), Some(x), Some(y)) = (one(k), one(q), one(q + 1)) else {
                        return false;
                    };
                    a.cost.as_ref().is_some_and(|c| pair_fits(a, c, x, y))
                };
                pairs
                    .iter()
                    .map(|&q| all || (by_quantity && fits(q)))
                    .collect()
            }
        }
        None => vec![first_free_is_unique(drafts, None); pairs.len()],
    };
    let mut tagged = untagged();
    for (&p, _) in pairs.iter().zip(&tag_pair).filter(|(_, t)| **t) {
        for i in [p, p + 1] {
            if let Some(flag) = tagged.get_mut(i) {
                *flag = true;
            }
        }
    }
    Plan {
        tagged,
        refused: false,
    }
}

/// The first posting that is neither a conversion posting nor priced is the anchor; the pairs
/// are tagged when no other such posting repeats its number (to half a unit at the anchor's
/// places). With no anchor nothing is tagged. `priced_repeater`: the index of a priced posting
/// that also counts as a posting that can repeat the anchor.
fn first_free_is_unique(drafts: &[Draft], priced_repeater: Option<usize>) -> bool {
    let mut free = drafts
        .iter()
        .enumerate()
        .filter(|(i, p)| !p.conversion_account && (!priced(p) || priced_repeater == Some(*i)))
        .map(|(_, p)| p);
    let Some(anchor) = free.next() else {
        return false;
    };
    let target = number(anchor);
    !free.any(|p| within_half_unit(&number(p), &target, target.places()))
}
