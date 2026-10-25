// The bench page: runs each format RUNS times, each in a new Web Worker, shows every metric against its target,
// and posts each line and the result to the server (server.ts), which prints and records them.
import { type Format, FORMATS, line, overall, RUNS, rows, type Run } from "./report.ts";

const TRANSACTIONS = 10_000;
const files: Record<Format, string> = { hledger: "big.journal", beancount: "big.beancount" };

type Reply = { run: Run; count: number } | { error: string };
type Info = { jsHeapBytes?: number; agentBytes?: number };

const out = document.getElementById("out") as HTMLElement;
const verdict = document.getElementById("verdict") as HTMLElement;

const post = (entry: Record<string, unknown>) =>
  fetch("/log", { method: "POST", body: JSON.stringify(entry) }).catch(() => undefined); // the page still shows it

const say = (text: string, entry: Record<string, unknown> = { kind: "line" }) => {
  out.textContent += `${text}\n`;
  return post({ ...entry, line: text });
};

/** Information only, Chromium's, read in the page while the worker is still alive (neither exists in a worker). The agent measure needs a cross-origin isolated page, so not over LAN http, and waits for a GC. */
async function info(): Promise<Info> {
  const out: Info = {};
  const heap = (performance as { memory?: { usedJSHeapSize: number } }).memory;
  if (heap) out.jsHeapBytes = heap.usedJSHeapSize;
  const measure = (performance as { measureUserAgentSpecificMemory?: () => Promise<{ bytes: number }> }).measureUserAgentSpecificMemory;
  if (self.crossOriginIsolated && measure) {
    const timeout = new Promise<undefined>((resolve) => setTimeout(resolve, 10_000));
    // It may throw even when it exists (not allowed in this browser build): a missing reading, never a failed bench.
    out.agentBytes = (await Promise.race([measure.call(performance), timeout]).catch(() => undefined))?.bytes;
  }
  return out;
}

function runInWorker(format: Format): Promise<Extract<Reply, { run: Run }> & Info> {
  return new Promise((resolve, reject) => {
    const worker = new Worker("/worker.js", { type: "module" });
    worker.onmessage = async (event: MessageEvent<Reply>) => {
      const reply = event.data;
      const extra = "error" in reply ? {} : await info();
      worker.terminate();
      if ("error" in reply) reject(new Error(reply.error));
      else resolve({ ...reply, ...extra });
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message || "the worker failed to load"));
    };
    worker.postMessage({ file: files[format] });
  });
}

const mb = (bytes: number | undefined) => (bytes === undefined ? "n/a" : `${(bytes / 1e6).toFixed(1)} MB`);

async function main() {
  await say(`${navigator.userAgent}`, { kind: "start", ua: navigator.userAgent, crossOriginIsolated: self.crossOriginIsolated });
  await say(`${TRANSACTIONS} transactions per format, ${RUNS} runs each, a new worker per run. Keep this tab in front and the screen on.`);
  const samples: Record<Format, Run[]> = { hledger: [], beancount: [] };
  for (const format of FORMATS) {
    for (let i = 0; i < RUNS; i++) {
      const reply = await runInWorker(format);
      if (reply.count < TRANSACTIONS) throw new Error(`${format} run ${i + 1}: transactions() returned ${reply.count} rows, expected at least ${TRANSACTIONS}`);
      samples[format].push(reply.run);
      const { run } = reply;
      await say(
        `${format} run ${i + 1}: start ${run.start.toFixed(0)}, load+tx ${run["load+transactions"].toFixed(0)}, check ${run.check.toFixed(0)}, value ${run.value.toFixed(0)}, holdings ${run.holdings.toFixed(0)} ms; wasm memory ${mb(run.memory)} (info: page js heap ${mb(reply.jsHeapBytes)}, browser agent memory ${mb(reply.agentBytes)})`,
        { kind: "run", format, run: i + 1, metrics: run, transactions: reply.count, jsHeapBytes: reply.jsHeapBytes, agentBytes: reply.agentBytes },
      );
    }
  }
  const table = rows(samples);
  const summary = overall(table);
  await say(`\n${table.map(line).join("\n")}\n`, { kind: "table" });
  verdict.textContent = summary;
  verdict.dataset.met = String(summary === "device targets: met");
  await say(summary, { kind: "result", met: summary === "device targets: met", overall: summary, rows: table });
}

main().catch(async (error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  const summary = `device targets: not met (error: ${message})`;
  verdict.textContent = summary;
  verdict.dataset.met = "false";
  await say(`ERROR ${message}`, { kind: "error", message });
  await say(summary, { kind: "result", met: false, overall: summary });
});
