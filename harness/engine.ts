// Shared by stress.ts and bench.ts: load a built `@ledgermill/wasm` package directory as its own module
// instance, and the engine calls both of them time. Does not import lib.ts, so no reference tool is needed.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { beancountLedger, hledgerJournal } from "./bigjournal.ts";

export type Ledger = {
  transactions(): string;
  postings(): string;
  prices(): string;
  check(): string;
  atCost(): string;
  value(date: string, commodity: string): string;
  holdings(query: string): string;
  free(): void;
};
export type Api = {
  initSync(init: { module: BufferSource }): { memory: WebAssembly.Memory };
  load(files: string): Ledger;
  format(name: string, content: string): string;
};
/** One build of the package: its glue, its wasm bytes and the memory once started. */
export type Build = { api: Api; bytes: Uint8Array; memory?: WebAssembly.Memory };

export const ROOT = join(import.meta.dir, "..");
export const PKG = join(ROOT, "pkg");

/** Import `DIR/ledgermill_wasm.js` as a fresh module instance (its own wasm state): `tag` must be new each time. */
export async function importBuild(dir: string, tag: string): Promise<Build> {
  const api = (await import(`${join(dir, "ledgermill_wasm.js")}?${tag}`)) as Api;
  return { api, bytes: readFileSync(join(dir, "ledgermill_wasm_bg.wasm")) };
}

/** Compile and instantiate the wasm; `build.memory` is set. */
export function start(build: Build): void {
  build.memory = build.api.initSync({ module: build.bytes }).memory;
}

export const formats = [
  { name: "hledger", file: "big.journal", generate: hledgerJournal },
  { name: "beancount", file: "big.beancount", generate: beancountLedger },
] as const;

/** What the engine is asked about one generated ledger. `end` is its last date. */
export type Case = { file: string; content: string; end: string };
export const makeCase = (file: string, content: string): Case => ({
  file,
  content,
  end: content.match(/^\d{4}-\d\d-\d\d/gm)?.at(-1) ?? "2015-01-01",
});

export const load = (api: Api, c: Case): Ledger => api.load(JSON.stringify([{ name: c.file, content: c.content }]));

/** The calls on a loaded ledger, by the name each is reported under. They return the JSON text. */
export const calls = {
  transactions: (l: Ledger) => l.transactions(),
  postings: (l: Ledger) => l.postings(),
  prices: (l: Ledger) => l.prices(),
  check: (l: Ledger) => l.check(),
  atCost: (l: Ledger) => l.atCost(),
  value: (l: Ledger, c: Case) => l.value(c.end, "EUR"),
  holdings: (l: Ledger, c: Case) => l.holdings(JSON.stringify({ commodity: "EUR", end: null, today: c.end })),
} satisfies Record<string, (l: Ledger, c: Case) => string>;

export const time = <T>(fn: () => T): [ms: number, result: T] => {
  const t0 = performance.now();
  const result = fn();
  return [performance.now() - t0, result];
};

/** The finding a ledgermill call threw as the message of an `Error` (a refusal), or null for any other failure. */
export function findingOf(error: unknown): { code: string; message?: string } | null {
  if (!(error instanceof Error) || error instanceof WebAssembly.RuntimeError) return null;
  try {
    const parsed: unknown = JSON.parse(error.message);
    return typeof parsed === "object" && parsed !== null && "code" in parsed && typeof parsed.code === "string"
      ? (parsed as { code: string; message?: string })
      : null;
  } catch {
    return null;
  }
}
