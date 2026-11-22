//! Thin wasm-bindgen binding over the ledgermill core. Values cross as JSON strings in the
//! core's own shape; all ledger logic stays in the core.

use ledgermill::SourceFile;
use wasm_bindgen::prelude::*;

/// Format a journal or a Beancount ledger (`ledgermill::format`): an hledger journal gets postings
/// indented by 4 spaces and the amounts of a transaction ending in one column; a Beancount ledger
/// gets the amounts of the whole file in one column and its usual indent. Only whitespace changes;
/// if the result would not read as the same ledger, or ledgermill refuses the file, `content` comes
/// back unchanged. `name` decides the format.
#[wasm_bindgen]
pub fn format(name: &str, content: &str) -> String {
    ledgermill::format(&SourceFile {
        name: name.into(),
        content: content.into(),
    })
}

/// A loaded ledger held in wasm memory. Call `free()` when done.
#[wasm_bindgen]
pub struct Ledger(ledgermill::Ledger);

/// Load `files`, a JSON array of `{ "name", "content" }`, in order. All the Beancount files are one
/// ledger and all the hledger files are one; the answers merge them (see `ledgermill::load`).
/// `format` stays one file.
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

    /// JSON array of the `P` directives, or for Beancount the `price` directives (`Ledger::prices`).
    pub fn prices(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.prices())?)
    }

    /// JSON array of findings.
    pub fn check(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.check())?)
    }

    /// JSON array of transactions as `hledger print --cost` gives them, or for Beancount with every cost
    /// as `cost(position)` (`Ledger::at_cost`).
    ///
    /// When the core refuses, this throws an `Error` whose `message` is the finding as JSON
    /// (`code`, `severity`, `file`, `line`, optional `column`, `details`, `message`), as it does
    /// for any other failure.
    #[wasm_bindgen(js_name = atCost)]
    pub fn at_cost(&self) -> Result<String, JsError> {
        answer(self.0.at_cost())
    }

    /// JSON array of transactions as `hledger print --value=DATE,COMMODITY` gives them, or for
    /// Beancount with every posting as `convert(position, COMMODITY, DATE)` (`Ledger::value`);
    /// `date` is `YYYY-MM-DD`. A refusal or an invalid date throws an `Error`
    /// whose `message` is the finding as JSON, as for `atCost`.
    pub fn value(&self, date: &str, commodity: &str) -> Result<String, JsError> {
        answer(self.0.value(date, commodity))
    }

    /// JSON array of holdings (`Ledger::holdings`). `query` is a JSON object
    /// `{ "commodity": "EUR", "end": "2026-05-01", "today": "2026-10-23" }`; `end` may be left out
    /// or `null`. A refusal or an invalid date throws an `Error` whose `message` is the finding as
    /// JSON, as for `atCost`; so does a `query` that is not that object.
    pub fn holdings(&self, query: &str) -> Result<String, JsError> {
        answer(parse_holdings_query(query).and_then(|q| self.0.holdings(&q)))
    }
}

/// Decode the JSON query for `holdings()`, turning invalid JSON or a missing field into an
/// `invalid-argument` finding so callers always receive a finding as JSON on error.
fn parse_holdings_query(query: &str) -> Result<ledgermill::HoldingsQuery, ledgermill::Finding> {
    serde_json::from_str(query).map_err(|error| ledgermill::Finding {
        code: "invalid-argument".into(),
        severity: ledgermill::Severity::Error,
        file: String::new(),
        line: 0,
        column: None,
        details: serde_json::json!({ "argument": "query", "value": query }),
        message: format!("The holdings query is not valid: {error}"),
    })
}

/// The answer as JSON, or the finding as the message of the error.
fn answer<T: serde::Serialize>(result: Result<T, ledgermill::Finding>) -> Result<String, JsError> {
    match result {
        Ok(answer) => Ok(serde_json::to_string(&answer)?),
        Err(finding) => Err(JsError::new(&serde_json::to_string(&finding)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_holdings_query;

    #[test]
    fn malformed_holdings_query_becomes_an_invalid_argument_finding() {
        for bad in ["{", "{}", r#"{"commodity":"EUR"}"#] {
            let finding = match parse_holdings_query(bad) {
                Ok(_) => panic!("expected error for {bad}"),
                Err(finding) => finding,
            };
            assert_eq!(finding.code, "invalid-argument");
            assert_eq!(finding.severity, ledgermill::Severity::Error);
            assert_eq!(finding.details["argument"], "query");
            assert_eq!(finding.details["value"], bad);
        }
    }

    #[test]
    fn the_message_does_not_call_a_valid_json_query_invalid_json() {
        for (bad, reason) in [("{", "EOF"), ("{}", "missing field `commodity`")] {
            let Err(finding) = parse_holdings_query(bad) else {
                panic!("expected error for {bad}");
            };
            assert!(
                finding
                    .message
                    .starts_with("The holdings query is not valid: ")
            );
            assert!(finding.message.contains(reason), "{}", finding.message);
        }
    }
}
