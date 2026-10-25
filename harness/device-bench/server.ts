// bun harness/device-bench/server.ts [--port 8765] [--results FILE]
// Serves the device bench page, its worker and `pkg/` to a phone on the LAN (see README.md). The 10,000-transaction
// ledgers are generated here, once, at start; the worker only fetches them, before its clock starts. Every line the
// page reports is printed here and appended, with the user agent, to results.jsonl (gitignored).
import { appendFileSync, existsSync } from "node:fs";
import { networkInterfaces } from "node:os";
import { basename, join } from "node:path";
import { beancountLedger, hledgerJournal } from "../bigjournal.ts";

const TRANSACTIONS = 10_000;
const SEED = 1; // as harness/bench.ts

const here = import.meta.dir;
const pkg = join(here, "../../pkg");
const args = process.argv.slice(2);
const option = (name: string): string | undefined => {
  const at = args.indexOf(name);
  return at < 0 ? undefined : args[at + 1];
};
const port = Number(option("--port") ?? 8765);
const results = option("--results") ?? join(here, "results.jsonl");
if (!Number.isInteger(port) || port < 0 || port > 65535) {
  console.error("usage: bun harness/device-bench/server.ts [--port 8765] [--results FILE]");
  process.exit(2);
}
if (!existsSync(join(pkg, "ledgermill_wasm_bg.wasm"))) {
  console.error("pkg/ is missing: build the wasm first with crates/ledgermill-wasm/build.sh");
  process.exit(2);
}

async function bundle(entry: string): Promise<string> {
  const build = await Bun.build({ entrypoints: [join(here, entry)], target: "browser", format: "esm", external: ["/pkg/*"] });
  const [output] = build.outputs;
  if (!build.success || output === undefined) throw new Error(`bundling ${entry} failed:\n${build.logs.join("\n")}`);
  return output.text();
}
const pageJs = await bundle("page.ts");
const workerJs = await bundle("worker.ts");

const data = new Map([
  ["big.journal", hledgerJournal(TRANSACTIONS, SEED)],
  ["big.beancount", beancountLedger(TRANSACTIONS, SEED)],
]);

const html = `<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ledgermill device bench</title>
<style>
  body { font: 12px/1.4 ui-monospace, Menlo, monospace; margin: 12px; }
  #out { white-space: pre-wrap; }
  #verdict { font-size: 16px; }
  #verdict[data-met="true"] { color: #070; }
  #verdict[data-met="false"] { color: #b00; }
</style>
<h1 id="verdict">running…</h1>
<pre id="out"></pre>
<script type="module" src="/page.js"></script>`;

// Cross-origin isolation lets Chromium offer measureUserAgentSpecificMemory (where the page is a secure context).
const headers = {
  "cache-control": "no-store", // every run fetches the wasm again, like a first visit
  "cross-origin-opener-policy": "same-origin",
  "cross-origin-embedder-policy": "require-corp",
};
const text = (body: string, type: string) => new Response(body, { headers: { ...headers, "content-type": type } });

const server = Bun.serve({
  hostname: "0.0.0.0",
  port,
  async fetch(req) {
    const path = new URL(req.url).pathname;
    if (req.method === "POST" && path === "/log") {
      const entry = (await req.json()) as { line?: string };
      appendFileSync(results, `${JSON.stringify({ ...entry, ua: req.headers.get("user-agent"), at: new Date().toISOString() })}\n`);
      console.log(entry.line ?? JSON.stringify(entry));
      return new Response("ok");
    }
    if (path === "/") return text(html, "text/html; charset=utf-8");
    if (path === "/page.js") return text(pageJs, "text/javascript");
    if (path === "/worker.js") return text(workerJs, "text/javascript");
    const ledger = data.get(basename(path));
    if (path.startsWith("/data/") && ledger !== undefined) return text(ledger, "text/plain; charset=utf-8");
    if (path.startsWith("/pkg/") && /^ledgermill_wasm(_bg\.wasm|\.js)$/.test(basename(path))) {
      return new Response(Bun.file(join(pkg, basename(path))), { headers });
    }
    return new Response("not found", { status: 404 });
  },
});

console.log(`Device bench: ${TRANSACTIONS} transactions per format, results in ${results}`);
const lan = Object.values(networkInterfaces()).flatMap((list) => list ?? []).filter((i) => i.family === "IPv4" && !i.internal);
for (const { address } of lan) console.log(`Open on the phone: http://${address}:${server.port}`);
console.log(`Open here: http://localhost:${server.port}`);
