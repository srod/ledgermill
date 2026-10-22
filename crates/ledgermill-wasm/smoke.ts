// bun crates/ledgermill-wasm/smoke.ts [JOURNAL...]
// Loads the wasm built into pkg/ (see harness/README.md) and prints transactions, postings and findings.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { initSync, load } from "../../pkg/ledgermill_wasm.js";

const root = join(import.meta.dir, "../..");
initSync({ module: readFileSync(join(root, "pkg/ledgermill_wasm_bg.wasm")) });

const paths = process.argv.slice(2);
const files = (paths.length > 0 ? paths : [join(root, "spec/hledger/probes/basic.journal")]).map(
  (path) => ({ name: path, content: readFileSync(path, "utf8") }),
);
const ledger = load(JSON.stringify(files));
try {
  console.log(
    JSON.stringify(
      {
        transactions: JSON.parse(ledger.transactions()),
        postings: JSON.parse(ledger.postings()),
        findings: JSON.parse(ledger.check()),
      },
      null,
      2,
    ),
  );
} finally {
  ledger.free();
}
