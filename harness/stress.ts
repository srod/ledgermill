// bun harness/stress.ts [N=100000]
// Generates N transactions per format (bigjournal.ts), runs them through every call of the wasm in pkg/ and
// prints the time and the wasm memory after each step. Fails (exit 1) only on a crash: a wasm trap, an
// error that is not a finding, or the process dying. Refusals and findings are printed and are fine.
import { calls, findingOf, formats, importBuild, load, makeCase, PKG, start, time, type Build, type Ledger } from "./engine.ts";

const SEED = 100;
const n = Number(process.argv[2] ?? 100_000);
if (!Number.isInteger(n) || n < 0) {
  console.error("usage: bun harness/stress.ts [N=100000]");
  process.exit(2);
}

const mb = (bytes: number) => `${(bytes / 1048576).toFixed(1)} MB`;
const memory = (build: Build) => mb(build.memory?.buffer.byteLength ?? 0);

class Crash extends Error {}

/** Run one step and print it. A finding thrown by the wasm is a refusal (printed, returns undefined); anything else is a crash. */
function step<T>(build: Build, label: string, fn: () => T, describe: (result: T) => string = () => ""): T | undefined {
  try {
    const [ms, result] = time(fn);
    console.log(`  ${label.padEnd(13)} ${ms.toFixed(0).padStart(7)} ms   wasm ${memory(build).padStart(9)}   ${describe(result)}`);
    return result;
  } catch (error) {
    const finding = findingOf(error);
    if (finding) {
      console.log(`  ${label.padEnd(13)} refused: ${finding.code}${finding.message ? ` (${finding.message})` : ""}`);
      return undefined;
    }
    console.log(`  ${label.padEnd(13)} CRASH: ${error instanceof Error ? `${error.name}: ${error.message.slice(0, 300)}` : String(error)}`);
    throw new Crash(label);
  }
}

const size = (json: string) => (json.length < 1048576 ? `${(json.length / 1024).toFixed(0)} KB` : mb(json.length));
/** `check()` as counts per finding code. */
function findings(json: string): string {
  const counts = new Map<string, number>();
  for (const f of JSON.parse(json) as { code: string }[]) counts.set(f.code, (counts.get(f.code) ?? 0) + 1);
  return `${size(json)}; ${counts.size === 0 ? "no findings" : [...counts].map(([code, k]) => `${k} ${code}`).join(", ")}`;
}

let crashed = false;
const peaks: string[] = [];
for (const format of formats) {
  console.log(`${format.name}, ${n} transactions`);
  const build = await importBuild(PKG, format.name).catch((error: unknown) => {
    console.error(`cannot load pkg/ledgermill_wasm.js (${error}). Build the wasm first: see harness/README.md`);
    process.exit(2);
  });
  let ledger: Ledger | undefined;
  try {
    const [genMs, content] = time(() => format.generate(n, SEED));
    console.log(`  ${"generate".padEnd(13)} ${genMs.toFixed(0).padStart(7)} ms   ${mb(content.length)} of text`);
    const c = makeCase(format.file, content);
    step(build, "start", () => start(build));
    ledger = step(build, "load", () => load(build.api, c));
    if (ledger) {
      const l = ledger;
      step(build, "transactions", () => calls.transactions(l), size);
      step(build, "postings", () => calls.postings(l), size);
      step(build, "prices", () => calls.prices(l), size);
      step(build, "check", () => calls.check(l), findings);
      step(build, "atCost", () => calls.atCost(l), size);
      step(build, "value", () => calls.value(l, c), size);
      step(build, "holdings", () => calls.holdings(l, c), size);
    }
    step(build, "format", () => build.api.format(c.file, c.content), (out) => (out === c.content ? "unchanged" : "changed"));
    if (ledger) {
      const l = ledger;
      step(build, "free", () => l.free());
    }
  } catch (error) {
    if (!(error instanceof Crash)) throw error;
    crashed = true;
  }
  peaks.push(`${format.name} ${memory(build)}`);
}
console.log(`peak wasm memory: ${peaks.join(", ")}; process rss ${mb(process.memoryUsage().rss)}`);
if (crashed) {
  console.log("stress: CRASH");
  process.exit(1);
}
console.log("stress: ok (no crash)");
