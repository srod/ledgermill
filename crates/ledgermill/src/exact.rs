//! What a load keeps besides the public model, so that `at_cost` and `value` need no second
//! parse: the amounts with every place they have (an inferred price has 255), and which postings
//! carry the tag `--cost` drops.

use crate::posting::WrittenAmount;

/// One transaction, parallel to the public `Transaction` of the same position.
#[derive(Debug, Clone)]
pub(crate) struct Exact {
    pub postings: Vec<ExactPosting>,
    /// `at_cost` cannot say what the reference does with this transaction (`spec/hledger/cost.md`,
    /// Gaps).
    pub cost_refused: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ExactPosting {
    /// The amounts as the transaction has them once balanced: elided and assigned amounts
    /// worked out, prices inferred.
    pub amounts: Vec<WrittenAmount>,
    /// Carries the tag `_conversion-posting`, written in its comment or put there by the
    /// reference tool's conversion pairs.
    pub conversion_tagged: bool,
}
