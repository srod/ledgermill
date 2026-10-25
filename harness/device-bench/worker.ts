// The bench's Web Worker (a module worker). The page starts a new one per format and run and sends it `{ file }`.
// It fetches the generated ledger first (the server generated it; neither generation nor the download is timed), then
// times the engine and posts the numbers back as a `Run` (report.ts).
import type { Run } from "./report.ts";

type Ledger = { transactions(): string; check(): string; value(date: string, commodity: string): string; holdings(query: string): string; free(): void };
type Glue = { default(): Promise<{ memory: WebAssembly.Memory }>; load(files: string): Ledger };

const glue = "/pkg/ledgermill_wasm.js"; // served by server.ts; kept out of the bundle

const timed = <T>(fn: () => T): [ms: number, result: T] => {
  const t0 = performance.now();
  const result = fn();
  return [performance.now() - t0, result];
};

self.onmessage = async (event: MessageEvent<{ file: string }>) => {
  try {
    const { file } = event.data;
    const content = await (await fetch(`/data/${file}`)).text();
    const end = content.match(/^\d{4}-\d\d-\d\d/gm)?.at(-1) ?? "2015-01-01";

    // start: fetch the glue and the wasm, compile, instantiate.
    const t0 = performance.now();
    const api = (await import(glue)) as Glue;
    const { memory } = await api.default();
    const start = performance.now() - t0;

    const [loadMs, ledger] = timed(() => api.load(JSON.stringify([{ name: file, content }])));
    const [txMs, transactions] = timed(() => ledger.transactions());
    const [check] = timed(() => ledger.check());
    const [value] = timed(() => ledger.value(end, "EUR"));
    const [holdings] = timed(() => ledger.holdings(JSON.stringify({ commodity: "EUR", end: null, today: end })));
    ledger.free();

    const run: Run = { start, "load+transactions": loadMs + txMs, check, value, holdings, memory: memory.buffer.byteLength };
    // Outside the clock: a bench that measured an empty ledger would pass every target.
    const count = (JSON.parse(transactions) as unknown[]).length;
    postMessage({ run, count });
  } catch (error) {
    postMessage({ error: error instanceof Error ? error.message : String(error) });
  }
};
