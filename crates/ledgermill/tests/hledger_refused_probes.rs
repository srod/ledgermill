//! Every file in spec/hledger/probes/refused is refused: `check` gives at least one
//! `unsupported-syntax` error for it. The directory is read when the test runs, so a new probe
//! is covered without touching this file. (The harness does not run these files: hledger reads
//! them and we say we cannot.)
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::PathBuf;
use support::*;

/// Files of the directory that are not refused on their own: the target of an `include`, and the
/// rules file that goes with a CSV file.
const COMPANIONS: [&str; 2] = ["include-target.journal", "csv.csv.rules"];

#[test]
fn every_refused_probe_is_refused() {
    let dir: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "../../spec/hledger/probes/refused",
    ]
    .iter()
    .collect();
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if COMPANIONS.contains(&name.as_str()) {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap();
        let refused = findings(&name, &content)
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "unsupported-syntax" && f["severity"] == "error");
        assert!(refused, "{name}: no unsupported-syntax error");
        checked += 1;
    }
    assert!(checked > 0, "no probe found in {}", dir.display());
}

#[test]
fn the_companions_are_there() {
    for name in COMPANIONS {
        let path: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "../../spec/hledger/probes/refused",
            name,
        ]
        .iter()
        .collect();
        assert!(path.is_file(), "{name} is listed but missing");
    }
}
