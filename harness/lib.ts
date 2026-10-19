// Shared by compare.ts and real.ts: run hledger and our wasm on the same files and diff typed values.
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";

const ROOT = join(import.meta.dir, "..");

// The wasm comes from `pkg/`, built as described in harness/README.md.
const wasm = await import(join(ROOT, "pkg/ledgermill_wasm.js")).catch((error: unknown) => {
  console.error(`cannot load pkg/ledgermill_wasm.js (${error}). Build the wasm first: see harness/README.md`);
  process.exit(2);
});
wasm.initSync({ module: readFileSync(join(ROOT, "pkg/ledgermill_wasm_bg.wasm")) });

// ───────────── our typed shape (crates/ledgermill/src/model.rs) ─────────────
export type Amount = { commodity: string; quantity: string; cost?: unknown };
export type Posting = { account: string; kind: string; comment: string; amounts: Amount[] };
export type Transaction = {
  date: string;
  status: "" | "!" | "*";
  description: string;
  comment: string;
  source: { file: string; line: number };
  postings: Posting[];
};
type Finding = { code: string; severity: "error" | "warning"; file: string; line: number; details: { construct?: string } };

export type Verdict = { status: "PASS" | "FAIL" | "SKIP"; note: string };

// ───────────── hledger side ─────────────
/** `-12.50` from mantissa -1250 and 2 places, exact (BigInt, never Number). */
export function decimalString(mantissa: bigint, places: number): string {
  const negative = mantissa < 0n;
  let digits = (negative ? -mantissa : mantissa).toString();
  if (places > 0) {
    digits = digits.padStart(places + 1, "0");
    digits = `${digits.slice(0, -places)}.${digits.slice(-places)}`;
  }
  return negative ? `-${digits}` : digits;
}

/** An integer as hledger prints it: plain (`123`) or scientific (`1.0e20`), exact either way. */
function exactInteger(text: string): bigint {
  const m = /^(-?)(\d+)(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/.exec(text);
  if (!m) throw new Error(`not a number: ${text}`);
  const fraction = m[3] ?? "";
  const shift = Number(m[4] ?? 0) - fraction.length;
  if (shift < 0) throw new Error(`not an integer: ${text}`);
  return BigInt(`${m[1]}${m[2]}${fraction}${"0".repeat(shift)}`);
}

const STATUS = { Unmarked: "", Pending: "!", Cleared: "*" } as const;

// biome-ignore lint/suspicious/noExplicitAny: foreign JSON
type J = any;

function projectAmount(a: J): Amount {
  const out: Amount = {
    commodity: a.acommodity,
    quantity: decimalString(exactInteger(a.aquantity.decimalMantissa), a.aquantity.decimalPlaces),
  };
  // Our shape has no costs yet: surface them so a claimed equality can't hide one.
  if (a.acost || a.acostbasis) out.cost = { cost: a.acost, basis: a.acostbasis };
  return out;
}

/** Project `hledger print -O json` onto our Transaction shape. */
export function projectHledger(text: string): Transaction[] {
  // Quote mantissas first: JSON.parse would round anything above 2^53.
  const exact = text.replace(/"decimalMantissa":\s*([-+\d.eE]+)/g, '"decimalMantissa":"$1"');
  return (JSON.parse(exact) as J[]).map((t) => ({
    date: t.tdate,
    status: STATUS[t.tstatus as keyof typeof STATUS] ?? `?${t.tstatus}`,
    description: t.tdescription,
    comment: (t.tcomment ?? "").trim(),
    source: { file: t.tsourcepos[0].sourceName, line: t.tsourcepos[0].sourceLine },
    postings: t.tpostings.map(
      (p: J): Posting => ({
        account: p.paccount,
        kind: String(p.ptype).replace(/Posting$/, "").toLowerCase(),
        comment: (p.pcomment ?? "").trim(),
        amounts: p.pamount.map(projectAmount),
      }),
    ),
  }));
}

type Reference = { ok: true; transactions: Transaction[] } | { ok: false; message: string };

async function runHledger(paths: string[]): Promise<Reference> {
  const proc = Bun.spawn(["hledger", "--no-conf", ...paths.flatMap((p) => ["-f", p]), "print", "-O", "json"], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  if (exitCode !== 0) return { ok: false, message: stderr.split("\n")[0] ?? "" };
  return { ok: true, transactions: projectHledger(stdout) };
}

// ───────────── our side ─────────────
type Ours = { transactions: Transaction[]; findings: Finding[] } | { thrown: string };

function runOurs(paths: string[]): Ours {
  const files = paths.map((name) => ({ name, content: readFileSync(name, "utf8") }));
  let ledger: ReturnType<typeof wasm.load> | undefined;
  try {
    ledger = wasm.load(JSON.stringify(files));
    return { transactions: JSON.parse(ledger.transactions()), findings: JSON.parse(ledger.check()) };
  } catch (error) {
    return { thrown: String(error) };
  } finally {
    ledger?.free();
  }
}

// ───────────── diff ─────────────
/** Path and values of the first difference, or null when equal. Compares source.file by basename. */
export function firstDiff(a: unknown, b: unknown, path = ""): string | null {
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return `${path || "(root)"}: length hledger=${a.length} ours=${b.length}`;
    for (let i = 0; i < a.length; i++) {
      const d = firstDiff(a[i], b[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && b && typeof a === "object" && typeof b === "object" && !Array.isArray(a) && !Array.isArray(b)) {
    const x = a as Record<string, unknown>;
    const y = b as Record<string, unknown>;
    for (const k of new Set([...Object.keys(x), ...Object.keys(y)])) {
      const d = firstDiff(x[k], y[k], `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return Object.is(a, b) ? null : `${path}: hledger=${JSON.stringify(a)} ours=${JSON.stringify(b)}`;
}

const withBasenames = (txs: Transaction[]): Transaction[] =>
  txs.map((t) => ({ ...t, source: { ...t.source, file: basename(t.source.file) } }));

// ───────────── the verdict ─────────────
export async function compareCase(paths: string[]): Promise<Verdict> {
  const ref = await runHledger(paths);
  const ours = runOurs(paths);
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  const errors = ours.findings.filter((f) => f.severity === "error");

  if (!ref.ok) {
    return errors.length > 0
      ? { status: "PASS", note: "both fail" }
      : { status: "FAIL", note: `hledger fails (${ref.message}), ours reports no error` };
  }
  const unsupported = ours.findings.find((f) => f.code === "unsupported-syntax");
  if (unsupported) return { status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"}` };
  const unbalanced = ours.findings.find((f) => f.code === "unbalanced-transaction");
  if (unbalanced) return { status: "FAIL", note: `ours reports unbalanced-transaction at line ${unbalanced.line}, hledger accepts` };
  const diff = firstDiff(withBasenames(ref.transactions), withBasenames(ours.transactions));
  if (diff) return { status: "FAIL", note: diff };
  if (errors[0]) return { status: "FAIL", note: `ours reports error ${errors[0].code} at line ${errors[0].line}, hledger accepts` };
  return { status: "PASS", note: "" };
}

const JOBS = 8;

/** Run `cases` (hledger in parallel), print one line each in order (only non-PASS with `quiet`) and the totals; exit code 1 on any FAIL. */
export async function runCases(cases: { name: string; paths: string[] }[], quiet: boolean): Promise<void> {
  const verdicts: Verdict[] = [];
  let next = 0;
  const worker = async () => {
    while (next < cases.length) {
      const i = next++;
      verdicts[i] = await compareCase((cases[i] as { paths: string[] }).paths);
    }
  };
  await Promise.all(Array.from({ length: JOBS }, worker));

  const total = { PASS: 0, FAIL: 0, SKIP: 0 };
  for (const [i, v] of verdicts.entries()) {
    total[v.status]++;
    if (!quiet || v.status !== "PASS") console.log(`${v.status} ${(cases[i] as { name: string }).name}${v.note ? `  ${v.note}` : ""}`);
  }
  console.log(`\n${total.PASS} pass, ${total.FAIL} fail, ${total.SKIP} skip`);
  if (total.FAIL > 0) process.exitCode = 1;
}
