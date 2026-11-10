//! Times `load` plus the JSON of `transactions()`, and `format`, on a file:
//! `cargo run --release --example bench -- FILE [RUNS] [--print-format]`. `--print-format` writes the
//! formatted text to stdout instead, to compare two builds byte for byte.
//! Not packaged (the crate's `include` leaves `examples/` out).

use ledgermill::SourceFile;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_default();
    // The flag is read wherever it stands, so `FILE --print-format` works without a run count.
    let rest: Vec<String> = args.collect();
    let print_format = rest.iter().any(|a| a == "--print-format");
    let runs: usize = rest.iter().find_map(|a| a.parse().ok()).unwrap_or(15);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let files = [SourceFile {
        name: path,
        content,
    }];
    if print_format {
        if let Some(file) = files.first() {
            print!("{}", ledgermill::format(file));
        }
        return;
    }
    let mut times = Vec::new();
    let mut format_times = Vec::new();
    for _ in 0..runs {
        let start = Instant::now();
        let ledger = ledgermill::load(&files);
        let loaded = start.elapsed();
        let json = serde_json::to_string(&ledger.transactions()).unwrap_or_default();
        let total = start.elapsed();
        times.push((loaded, total, json.len()));
        if let Some(file) = files.first() {
            let start = Instant::now();
            let formatted = ledgermill::format(file);
            format_times.push((start.elapsed(), formatted.len()));
        }
    }
    times.sort();
    format_times.sort();
    if let Some((load, total, bytes)) = times.get(times.len() / 2) {
        println!("median: load {load:?}, load+json {total:?}, json {bytes} bytes");
    }
    if let Some((time, bytes)) = format_times.get(format_times.len() / 2) {
        println!("median: format {time:?}, formatted {bytes} bytes");
    }
}
