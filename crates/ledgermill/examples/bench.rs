//! Times `load` plus the JSON of `transactions()` on a file: `cargo run --release --example bench -- FILE [RUNS]`.
//! Not packaged (the crate's `include` leaves `examples/` out).

use ledgermill::SourceFile;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_default();
    let runs: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(15);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let files = [SourceFile {
        name: path,
        content,
    }];
    let mut times = Vec::new();
    for _ in 0..runs {
        let start = Instant::now();
        let ledger = ledgermill::load(&files);
        let loaded = start.elapsed();
        let json = serde_json::to_string(&ledger.transactions()).unwrap_or_default();
        let total = start.elapsed();
        times.push((loaded, total, json.len()));
    }
    times.sort();
    if let Some((load, total, bytes)) = times.get(times.len() / 2) {
        println!("median: load {load:?}, load+json {total:?}, json {bytes} bytes");
    }
}
