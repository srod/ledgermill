//! Thin wasm-bindgen binding over the ledgermill core. Values cross as JSON strings in the
//! core's own shape; all ledger logic stays in the core.

use ledgermill::SourceFile;
use wasm_bindgen::prelude::*;

/// A loaded ledger held in wasm memory. Call `free()` when done.
#[wasm_bindgen]
pub struct Ledger(ledgermill::Ledger);

/// Load `files`, a JSON array of `{ "name", "content" }`, in order.
#[wasm_bindgen]
pub fn load(files: &str) -> Result<Ledger, JsError> {
    let files: Vec<SourceFile> = serde_json::from_str(files)?;
    Ok(Ledger(ledgermill::load(&files)))
}

#[wasm_bindgen]
impl Ledger {
    /// JSON array of transactions.
    pub fn transactions(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.transactions())?)
    }

    /// JSON array of flat postings (`Ledger::postings`).
    pub fn postings(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.postings())?)
    }

    /// JSON array of the `P` directives (`Ledger::prices`); `[]` for Beancount.
    pub fn prices(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.prices())?)
    }

    /// JSON array of findings.
    pub fn check(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.check())?)
    }

    /// JSON array of transactions as `hledger print --cost` gives them (`Ledger::at_cost`).
    ///
    /// When the core refuses, this throws an `Error` whose `message` is the finding as JSON
    /// (`code`, `severity`, `file`, `line`, `details`, `message`), as it does for any other failure.
    #[wasm_bindgen(js_name = atCost)]
    pub fn at_cost(&self) -> Result<String, JsError> {
        answer(self.0.at_cost())
    }

    /// JSON array of transactions as `hledger print --value=DATE,COMMODITY` gives them
    /// (`Ledger::value`); `date` is `YYYY-MM-DD`. A refusal or an invalid date throws an `Error`
    /// whose `message` is the finding as JSON, as for `atCost`.
    pub fn value(&self, date: &str, commodity: &str) -> Result<String, JsError> {
        answer(self.0.value(date, commodity))
    }

    /// JSON array of holdings (`Ledger::holdings`). `query` is a JSON object
    /// `{ "commodity": "EUR", "end": "2026-05-01", "today": "2026-10-23" }`; `end` may be left out
    /// or `null`. A refusal or an invalid date throws an `Error` whose `message` is the finding as
    /// JSON, as for `atCost`; so does a `query` that is not that object.
    pub fn holdings(&self, query: &str) -> Result<String, JsError> {
        let query: ledgermill::HoldingsQuery = serde_json::from_str(query)?;
        answer(self.0.holdings(&query))
    }
}

/// The answer as JSON, or the finding as the message of the error.
fn answer<T: serde::Serialize>(result: Result<T, ledgermill::Finding>) -> Result<String, JsError> {
    match result {
        Ok(answer) => Ok(serde_json::to_string(&answer)?),
        Err(finding) => Err(JsError::new(&serde_json::to_string(&finding)?)),
    }
}
