//! Test helpers: everything goes through `load` -> `transactions()` / `check()`.
#![allow(dead_code)]

use ledgermill::{SourceFile, load};
use serde_json::Value;

pub fn file(name: &str, content: &str) -> SourceFile {
    SourceFile { name: name.into(), content: content.into() }
}

pub fn transactions(name: &str, content: &str) -> Value {
    serde_json::to_value(load(&[file(name, content)]).transactions()).unwrap()
}

pub fn findings(name: &str, content: &str) -> Value {
    serde_json::to_value(load(&[file(name, content)]).check()).unwrap()
}
