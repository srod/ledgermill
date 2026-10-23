//! The flat posting rows of an hledger transaction: one per posting and commodity, as
//! `hledger print --location -O csv` lists them (`spec/hledger/postings-flat.md`).

use crate::exact::Exact;
use crate::model::{FlatPosting, Status, Transaction};
use crate::posting::WrittenAmount;

/// The rows of `tx`. `lines` are the lines of its postings, in order, and `exact` has their
/// amounts with every place they have; a posting's rows are never rounded, and are padded to the
/// places their amount is shown with.
pub(crate) fn rows(tx: &Transaction, exact: &Exact, lines: &[u32]) -> Vec<FlatPosting> {
    let flag = match tx.status {
        Status::Unmarked => "",
        Status::Pending => "!",
        Status::Cleared => "*",
    };
    let mut out = Vec::new();
    for ((posting, written), &line) in tx.postings.iter().zip(&exact.postings).zip(lines) {
        for (number, currency) in amounts_of(&written.amounts) {
            out.push(FlatPosting {
                date: tx.date.clone(),
                flag: flag.into(),
                payee: None,
                narration: tx.description.clone(),
                account: posting.account.clone(),
                kind: posting.kind,
                number,
                currency,
                file: tx.source.file.clone(),
                line,
            });
        }
    }
    out
}

/// `(quantity, commodity)` of each row of one posting, commodities in code point order (the empty
/// one first). Zeros are left out when another amount is not zero; when all are zero (or there
/// are none) one row `0` is left, with the first named commodity, else none.
fn amounts_of(amounts: &[WrittenAmount]) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = amounts
        .iter()
        .filter(|a| !a.quantity.is_zero())
        .map(|a| {
            (
                a.quantity.padded_to(a.display_places).to_string(),
                a.commodity.clone(),
            )
        })
        .collect();
    if rows.is_empty() {
        let commodity = amounts
            .iter()
            .map(|a| a.commodity.as_str())
            .filter(|c| !c.is_empty())
            .min()
            .unwrap_or("");
        return vec![("0".into(), commodity.into())];
    }
    rows.sort_by(|a, b| a.1.cmp(&b.1));
    rows
}
