# Device bench

Checks the **device targets** on a phone before a release (`RELEASING.md`, step 2). It runs the wasm in a real Web
Worker on 10,000 generated transactions per format (hledger and Beancount, `harness/bigjournal.ts`, seed 1 as
`harness/bench.ts`), 3 runs each, and prints **met / not met** against each target, the worst of the 3 runs deciding:

| metric | target |
|---|---|
| start (fetch + compile + instantiate the wasm) | ≤ 300 ms |
| load + `transactions()` | ≤ 500 ms |
| `check()` | ≤ 500 ms |
| `value()` | ≤ 200 ms |
| `holdings()` | ≤ 200 ms |
| memory (`memory.buffer.byteLength` after the run) | ≤ 50 MB (50,000,000 bytes) |

## Run it

1. Build the wasm: `crates/ledgermill-wasm/build.sh` (the bench serves `pkg/`).
2. Start the server on the Mac: `bun harness/device-bench/server.ts [--port 8765]`. It prints the addresses to open.
3. Phone and Mac on the same Wi-Fi. Open `http://<mac-lan-ip>:8765` (`ipconfig getifaddr en0`) in the phone's browser.
   Plain http is fine: nothing here needs a secure context. (Safari may run WebAssembly slower outside one; if
   that is your phone, say so next to the result.) Allow incoming connections if macOS asks.
4. Keep the tab in front and the screen on until the page shows its verdict: a hidden or sleeping page is throttled and
   the numbers are void.
5. Read the verdict at the top of the page (and in the server's terminal): `device targets: met`, or
   `not met (hledger check, beancount memory)`.

**The numbers stay on your machine.** The page's lines and JSON entries (with the user agent) are appended to
`harness/device-bench/results.jsonl`, which is gitignored. The release notes only say `device targets: met`.

## How it measures

- The server generates both ledgers once, at start. The worker downloads its ledger before any clock starts, so neither
  generation nor the download is counted.
- Each run uses a **new worker** and a new wasm instance, so every run is cold. The server sends `no-store`, so every
  run fetches the glue and the wasm again, over your Wi-Fi: `start` includes that.
- `start` = import the glue + fetch, compile and instantiate the wasm. `load+transactions` = `JSON.stringify` of the
  input, `load()` and `transactions()` (the JSON text is not parsed in the timing). `check`, `value` and `holdings`
  are called on the loaded ledger, as `harness/engine.ts` does.
- Memory is the wasm memory after the run. The page also logs, for information only and where the browser has them,
  `performance.memory` (the page's JS heap, Chromium only) and `measureUserAgentSpecificMemory` (needs a cross-origin
  isolated, secure page: so not over LAN http).
- The run fails (verdict `not met (error: …)`) if a call throws or `transactions()` returns fewer than 10,000 rows.

## Check it on the desktop

`bun harness/device-bench/headless.ts` starts the server on port 8791 (results to a temp file), opens the page in
Playwright's headless Chromium, prints the verdict and exits 0 when it is met. It needs Playwright's Chromium
(`bunx playwright install chromium`); Bun fetches the `playwright` module itself. Desktop numbers say nothing about a
phone; the script only shows that the bench works.
