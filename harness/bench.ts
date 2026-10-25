// bun harness/bench.ts [--runs N=7] [--against DIR | --against-npm VERSION]
// Desktop timing bench: 10,000 generated transactions per format (fixed seed), the median of N runs after one
// warm-up. With --against (a previous @ledgermill/wasm package directory) or --against-npm (fetched with
// `npm pack`), the two builds run interleaved in this process, each as its own module instance, and the bench
// fails (exit 1) when a metric of the current build (pkg/) is more than 20% slower than the previous one.
import { appendFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { calls, type Build, formats, type Ledger, importBuild, load, makeCase, PKG, start, time } from "./engine.ts";

const TRANSACTIONS = 10_000;
const SEED = 1;
const LIMIT = 1.2;
// A timer cannot tell 0.01 ms from 0.02 ms: a metric must also be this much slower, in ms, to fail.
const NOISE_MS = 0.1;

const args = process.argv.slice(2);
const option = (name: string): string | undefined => {
  const at = args.indexOf(name);
  return at < 0 ? undefined : args[at + 1];
};
const runs = Number(option("--runs") ?? 7);
const against = option("--against");
const againstNpm = option("--against-npm");
if (!Number.isInteger(runs) || runs < 1 || (against !== undefined && againstNpm !== undefined) || args.includes("--against") !== (against !== undefined) || args.includes("--against-npm") !== (againstNpm !== undefined)) {
  console.error("usage: bun harness/bench.ts [--runs N=7] [--against DIR | --against-npm VERSION]");
  process.exit(2);
}

/** The directory of the package to compare with, or null when there is none to compare with. */
function previousDir(): string | null {
  if (against !== undefined) return resolve(against);
  if (againstNpm === undefined) return null;
  const tmp = mkdtempSync(join(tmpdir(), "ledgermill-bench-"));
  const spec = `@ledgermill/wasm@${againstNpm}`;
  const pack = Bun.spawnSync(["npm", "pack", spec, "--pack-destination", tmp], { cwd: tmp });
  if (!pack.success) {
    const err = pack.stderr.toString();
    if (/E404|404 Not Found/.test(err)) {
      console.log(`No previous release: ${spec} is not on npm. Reporting the current timings only.`);
      return null;
    }
    console.error(`npm pack ${spec} failed:\n${err}`);
    process.exit(1);
  }
  const tarball = pack.stdout.toString().trim().split("\n").pop() ?? "";
  const untar = Bun.spawnSync(["tar", "-xzf", join(tmp, tarball), "-C", tmp]);
  if (!untar.success) {
    console.error(`cannot unpack ${tarball}:\n${untar.stderr.toString()}`);
    process.exit(1);
  }
  return join(tmp, "package");
}

const median = (xs: number[]): number => {
  const s = [...xs].sort((a, b) => a - b);
  const mid = s.length >> 1;
  return s.length % 2 === 1 ? (s[mid] as number) : ((s[mid - 1] as number) + (s[mid] as number)) / 2;
};

// ───────────── the metrics ─────────────
const cases = formats.map((f) => ({ name: f.name, c: makeCase(f.file, f.generate(TRANSACTIONS, SEED)) }));
const metrics = ["start", "load+transactions", "check", "value", "holdings", "postings", "prices", "format"] as const;
const key = (metric: string, format: string) => `${metric}|${format}`;

type Side = { label: string; dir: string; build: Build; samples: Map<string, number[]>; notes: Set<string> };

/** One run of every metric on one build. A metric that throws (the previous build may lack it) is left out and noted. */
async function measure(side: Side, run: number): Promise<void> {
  const record = (metric: string, format: string, fn: () => unknown) => {
    try {
      const [ms] = time(fn);
      const k = key(metric, format);
      side.samples.set(k, [...(side.samples.get(k) ?? []), ms]);
    } catch (error) {
      side.notes.add(`${metric} ${format}: ${error instanceof Error ? error.message.slice(0, 200) : String(error)}`);
    }
  };
  const fresh = await importBuild(side.dir, `${side.label}-start-${run}`); // initSync does nothing twice in one instance
  record("start", "both", () => start(fresh));
  for (const { name, c } of cases) {
    const held: { ledger?: Ledger } = {};
    record("load+transactions", name, () => {
      held.ledger = load(side.build.api, c);
      return calls.transactions(held.ledger);
    });
    const ledger = held.ledger;
    if (ledger) {
      try {
        for (const m of ["check", "value", "holdings", "postings", "prices"] as const) record(m, name, () => calls[m](ledger, c));
      } finally {
        ledger.free();
      }
    }
    record("format", name, () => side.build.api.format(c.file, c.content));
  }
}

async function side(label: string, dir: string): Promise<Side> {
  const build = await importBuild(dir, label).catch((error: unknown) => {
    console.error(`cannot load ${dir}/ledgermill_wasm.js (${error}). ${dir === PKG ? "Build the wasm first: see harness/README.md" : ""}`);
    process.exit(2);
  });
  start(build);
  return { label, dir, build, samples: new Map(), notes: new Set() };
}

const previousPath = previousDir();
const current = await side("current", PKG);
const previous = previousPath === null ? null : await side("previous", previousPath);
const sides = previous ? [current, previous] : [current];
console.log(`${TRANSACTIONS} transactions per format, seed ${SEED}, ${runs} runs after 1 warm-up${previous ? `, previous build: ${previousPath}` : ""}`);

for (let run = 0; run <= runs; run++) {
  // Alternate who goes first, so that neither build always runs on a warmer or colder machine.
  for (const s of run % 2 === 0 ? sides : [...sides].reverse()) {
    await measure(s, run);
    if (run === 0) s.samples.clear(); // the warm-up
  }
}

// ───────────── the table ─────────────
const ms = (x: number | undefined) => (x === undefined ? "n/a" : x < 10 ? x.toFixed(2) : x.toFixed(1));
const rows = ["| metric | format | previous ms | current ms | ratio | verdict |", "|---|---|---:|---:|---:|---|"];
let failed = false;
for (const metric of metrics) {
  for (const format of metric === "start" ? ["both"] : formats.map((f) => f.name)) {
    const k = key(metric, format);
    const cur = current.samples.get(k);
    const prev = previous?.samples.get(k);
    const curMs = cur && median(cur);
    const prevMs = prev && median(prev);
    let ratio = "—";
    let verdict = previous ? "n/a" : "—";
    if (curMs === undefined) {
      verdict = "ERROR";
      failed = true;
    } else if (prevMs !== undefined && previous) {
      const x = curMs / prevMs;
      ratio = x.toFixed(2);
      const slower = x > LIMIT && curMs - prevMs > NOISE_MS;
      verdict = slower ? `SLOWER (> ${LIMIT}×)` : x > LIMIT ? "ok (under 0.1 ms)" : "ok";
      if (slower) failed = true;
    }
    rows.push(`| ${metric} | ${format} | ${previous ? ms(prevMs) : "—"} | ${ms(curMs)} | ${ratio} | ${verdict} |`);
  }
}
const table = rows.join("\n");
console.log(table);
for (const s of sides) for (const note of s.notes) console.log(`${s.label} build: ${note}`);
if (process.env.GITHUB_STEP_SUMMARY) {
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### Desktop bench (${TRANSACTIONS} transactions, median of ${runs})\n\n${table}\n\n`);
}
if (failed) {
  console.log(`bench: FAIL (an ERROR, or a metric more than ${Math.round((LIMIT - 1) * 100)}% slower than the previous build)`);
  process.exit(1);
}
console.log("bench: ok");
