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

    /// JSON array of flat postings (Beancount only; `[]` for hledger).
    pub fn postings(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.postings())?)
    }

    /// JSON array of findings.
    pub fn check(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.check())?)
    }
}
