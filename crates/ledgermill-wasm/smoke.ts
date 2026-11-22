// bun crates/ledgermill-wasm/smoke.ts [JOURNAL...]
// Loads the wasm built into pkg/ (see harness/README.md), checks every exported method and prints a summary.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { format, initSync, load } from "../../pkg/ledgermill_wasm.js";
import type { Finding } from "./types.d.ts";

const root = join(import.meta.dir, "../..");
initSync({ module: readFileSync(join(root, "pkg/ledgermill_wasm_bg.wasm")) });

const paths = process.argv.slice(2);
const files = (paths.length > 0 ? paths : [join(root, "spec/hledger/probes/basic.journal")]).map(
  (path) => ({ name: path, content: readFileSync(path, "utf8") }),
);
const ledger = load(JSON.stringify(files));
try {
  const transactions = JSON.parse(ledger.transactions());
  const postings = JSON.parse(ledger.postings());
  const prices = JSON.parse(ledger.prices());
  const findings = JSON.parse(ledger.check());
  const atCost = JSON.parse(ledger.atCost());
  const valued = JSON.parse(ledger.value("2026-10-25", "EUR"));
  const holdings = JSON.parse(ledger.holdings(JSON.stringify({ commodity: "EUR", today: "2026-10-25" })));
  const formatted = format(files[0].name, files[0].content);
  if (typeof formatted !== "string") throw new Error("format() did not return a string");

  // Malformed holdings() query must throw an Error whose message is a JSON-encoded Finding.
  let threw = false;
  try {
    ledger.holdings("{bad");
  } catch (err) {
    threw = true;
    const finding: Finding = JSON.parse(err instanceof Error ? err.message : String(err));
    if (finding.code !== "invalid-argument") {
      throw new Error(`expected invalid-argument finding, got ${finding.code}`);
    }
  }
  if (!threw) throw new Error("expected malformed holdings() query to throw");

  console.log(
    JSON.stringify(
      {
        transactions: transactions.length,
        postings: postings.length,
        prices: prices.length,
        findings,
        atCost: atCost.length,
        valued: valued.length,
        holdings: holdings.length,
      },
      null,
      2,
    ),
  );
} finally {
  ledger.free();
}
