// Shared by compare.ts, fuzz.ts and real.ts: run hledger and our wasm on the same files and diff
// typed values.
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
export type Quantity = { commodity: string; quantity: string };
export type Cost = { kind: "unit" | "total"; amount: Quantity };
export type Lot = { cost: Quantity | null; date: string | null; label: string | null };
export type Amount = { commodity: string; quantity: string; cost?: Cost; lot?: Lot };
export type Posting = {
  account: string;
  kind: string;
  comment: string;
  amounts: Amount[];
  flag?: string; // Beancount only
  line?: number; // Beancount only
};
export type Transaction = {
  date: string;
  status: "" | "!" | "*";
  flag?: string; // Beancount only
  payee?: string; // Beancount only
  description: string;
  comment: string;
  source: { file: string; line: number };
  postings: Posting[];
};
/** One row of `postings()` (Beancount only for now). */
export type FlatPosting = {
  date: string;
  flag: string; // the transaction's
  payee?: string;
  narration: string;
  account: string;
  number: string;
  currency: string;
  file: string;
  line: number;
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

function projectQuantity(a: J): Quantity {
  return {
    commodity: a.acommodity,
    quantity: decimalString(exactInteger(a.aquantity.decimalMantissa), a.aquantity.decimalPlaces),
  };
}

function projectAmount(a: J): Amount {
  const out: Amount = projectQuantity(a);
  if (a.acost) {
    const kind = a.acost.tag === "UnitCost" ? "unit" : a.acost.tag === "TotalCost" ? "total" : `?${a.acost.tag}`;
    out.cost = { kind: kind as Cost["kind"], amount: projectQuantity(a.acost.contents) };
  }
  if (a.acostbasis) {
    const b = a.acostbasis;
    out.lot = { cost: b.cbCost ? projectQuantity(b.cbCost) : null, date: b.cbDate ?? null, label: b.cbLabel ?? null };
  }
  return out;
}

const KIND: Record<string, string> = {
  RegularPosting: "regular",
  VirtualPosting: "virtual",
  BalancedVirtualPosting: "balanced-virtual",
};

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
        kind: KIND[p.ptype] ?? `?${p.ptype}`,
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
type Ours = { transactions: Transaction[]; postings: FlatPosting[]; findings: Finding[] } | { thrown: string };

function runOurs(paths: string[]): Ours {
  const files = paths.map((name) => ({ name, content: readFileSync(name, "utf8") }));
  let ledger: ReturnType<typeof wasm.load> | undefined;
  try {
    ledger = wasm.load(JSON.stringify(files));
    return {
      transactions: JSON.parse(ledger.transactions()),
      postings: JSON.parse(ledger.postings()),
      findings: JSON.parse(ledger.check()),
    };
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
  // Our JSON leaves out an absent cost or lot; hledger's projection sets neither then.
  return Object.is(a, b) ? null : `${path}: hledger=${JSON.stringify(a)} ours=${JSON.stringify(b)}`;
}

const withBasenames = (txs: Transaction[]): Transaction[] =>
  txs.map((t) => ({ ...t, source: { ...t.source, file: basename(t.source.file) } }));

const postingBasenames = (rows: FlatPosting[]): FlatPosting[] => rows.map((r) => ({ ...r, file: basename(r.file) }));

// ───────────── Beancount side ─────────────
/**
 * Decode bean-query's CSV: CRLF rows, fields in double quotes when they hold a comma, a quote or
 * a newline (quotes doubled inside). Fields come back as written: the padding bean-query adds to
 * numbers is for the caller to trim.
 */
export function decodeCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let quoted = false;
  let i = 0;
  const endField = () => {
    row.push(field);
    field = "";
  };
  while (i < text.length) {
    const c = text[i] as string;
    if (quoted) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i += 2;
          continue;
        }
        quoted = false;
      } else {
        field += c;
      }
    } else if (c === '"') {
      quoted = true;
    } else if (c === ",") {
      endField();
    } else if (c === "\r" || c === "\n") {
      if (c === "\r" && text[i + 1] === "\n") i++;
      endField();
      rows.push(row);
      row = [];
    } else {
      field += c;
    }
    i++;
  }
  if (field !== "" || row.length > 0) {
    endField();
    rows.push(row);
  }
  return rows;
}

const ENTRIES_QUERY =
  "SELECT id, type, filename, lineno, date, flag, payee, narration FROM entries WHERE type = 'transaction'";
/** The flat list: one row per booked posting, in the order `postings()` promises. */
const FLAT_POSTINGS_QUERY =
  "SELECT date, flag, payee, narration, account, number, currency, filename, lineno ORDER BY date, filename, lineno";
const POSTINGS_QUERY =
  "SELECT id, account, posting_flag, number, currency, cost_number, cost_currency, cost_date, cost_label, number(price), currency(price), filename, lineno FROM postings";

/** Rows of a CSV as objects keyed by column name. bean-query prints its errors in front of the CSV, so decoding starts at the header. */
function csvRows(csv: string, header: string): Record<string, string>[] {
  const at = csv.indexOf(header);
  if (at < 0) throw new Error(`no CSV header "${header}" in: ${csv.slice(0, 200)}`);
  const [names = [], ...data] = decodeCsv(csv.slice(at));
  return data.map((cells) => Object.fromEntries(names.map((n, i) => [n, cells[i] ?? ""])) as Record<string, string>);
}

/** Project the result of `FLAT_POSTINGS_QUERY` onto our FlatPosting shape; the padding around numbers is trimmed. */
export function projectFlatPostings(csv: string): FlatPosting[] {
  return csvRows(csv, "date,flag,payee,narration,account").map((r) => {
    const row: FlatPosting = {
      date: r.date as string,
      flag: r.flag as string,
      narration: r.narration as string,
      account: r.account as string,
      number: (r.number as string).trim(),
      currency: r.currency as string,
      file: r.filename as string,
      line: Number(r.lineno),
    };
    // bean-query prints a missing payee as an empty field, like an empty one.
    if (r.payee) row.payee = r.payee;
    return row;
  });
}

/**
 * Join the two query results on `id` (a transaction without postings stays) and project them onto our
 * Transaction shape. bean-query prints its errors in front of the CSV, so decoding starts at the header.
 */
export function projectBeancount(entriesCsv: string, postingsCsv: string): Transaction[] {
  const postings = csvRows(postingsCsv, "id,account,posting_flag");
  const byId = new Map<string, Record<string, string>[]>();
  for (const p of postings) byId.set(p.id as string, [...(byId.get(p.id as string) ?? []), p]);
  return csvRows(entriesCsv, "id,type,filename").map((e) => {
    const flag = e.flag as string;
    const tx: Transaction = {
      date: e.date as string,
      status: flag === "*" || flag === "!" ? flag : "",
      flag,
      description: e.narration as string,
      comment: "",
      source: { file: e.filename as string, line: Number(e.lineno) },
      postings: (byId.get(e.id as string) ?? []).map((p): Posting => {
        const amount: Amount = { commodity: p.currency as string, quantity: (p.number as string).trim() };
        if (p["currency(price)"]) {
          amount.cost = {
            kind: "unit",
            amount: { commodity: p["currency(price)"], quantity: (p["number(price)"] as string).trim() },
          };
        }
        if (p.cost_number?.trim()) {
          amount.lot = {
            cost: { commodity: p.cost_currency as string, quantity: p.cost_number.trim() },
            date: p.cost_date || null,
            label: p.cost_label || null,
          };
        }
        const posting: Posting = {
          account: p.account as string,
          kind: "regular",
          comment: "",
          amounts: [amount],
          line: Number(p.lineno),
        };
        if (p.posting_flag) posting.flag = p.posting_flag;
        return posting;
      }),
    };
    // bean-query prints a missing payee as an empty field, like an empty one.
    if (e.payee) tx.payee = e.payee;
    return tx;
  });
}

async function run(cmd: string[]): Promise<{ stdout: string; exitCode: number }> {
  const proc = Bun.spawn(cmd, { stdout: "pipe", stderr: "pipe" });
  const [stdout, , exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  return { stdout, exitCode };
}

/**
 * The file (basename) and line of the first error `bean-check` printed. Its errors are lines
 * `FILE:LINE: MESSAGE`, each followed by a copy of the directive it is about (indented) and a blank
 * line, in the order the reference found them; line 0 is a file-level error.
 */
export function firstBeancountError(output: string): { file: string; line: number } | null {
  for (const text of output.split("\n")) {
    const m = /^(\S.*?):(\d+): /.exec(text);
    if (m) return { file: basename(m[1] as string), line: Number(m[2]) };
  }
  return null;
}

type BeancountReference =
  | { ok: true; transactions: Transaction[]; postings: FlatPosting[] }
  | { ok: false; message: string; first: { file: string; line: number } | null };

async function runBeancount(path: string): Promise<BeancountReference> {
  // bean-query exits 0 even for an invalid journal, so bean-check decides.
  const check = await Bun.spawn(["bean-check", path], { stdout: "pipe", stderr: "pipe" });
  const [out, err, code] = await Promise.all([
    new Response(check.stdout).text(),
    new Response(check.stderr).text(),
    check.exited,
  ]);
  if (code !== 0) {
    const printed = out + err;
    return { ok: false, message: printed.split("\n").find((l) => l.trim()) ?? "", first: firstBeancountError(printed) };
  }
  const [entries, postings, flat] = await Promise.all([
    run(["bean-query", "-f", "csv", path, ENTRIES_QUERY]),
    run(["bean-query", "-f", "csv", path, POSTINGS_QUERY]),
    run(["bean-query", "-f", "csv", path, FLAT_POSTINGS_QUERY]),
  ]);
  return {
    ok: true,
    transactions: projectBeancount(entries.stdout, postings.stdout),
    postings: projectFlatPostings(flat.stdout),
  };
}

/** What our side reports for a Beancount journal, in the shape the reference gives. */
function normalizeOurs(txs: Transaction[]): Transaction[] {
  return txs.map((t) => {
    const { payee, ...rest } = t;
    return payee ? { ...rest, payee } : rest;
  });
}

/**
 * The reference rejects the journal: our first error must be on the same file and line as its first.
 * An error-free result is a FAIL, except when the reference's error is a missing `document` file and we
 * read the directive and warn: we do not look at files, so that one is a SKIP.
 */
export function compareFirstError(
  ref: { message: string; first: { file: string; line: number } | null },
  errors: Finding[],
  findings: Finding[],
): Verdict {
  const ours = errors[0];
  if (!ours) {
    const ignored = findings.find((f) => f.code === "ignored-syntax" && f.details.construct === "document");
    if (ignored && ref.message.includes("File does not exist"))
      return { status: "SKIP", note: `ignored: document (we do not look at files): ${ref.message}` };
    return { status: "FAIL", note: `reference rejects (${ref.message}), ours reports no error` };
  }
  if (!ref.first) return { status: "FAIL", note: `cannot read the first error of: ${ref.message}` };
  const mine = `${basename(ours.file)}:${ours.line}`;
  const theirs = `${ref.first.file}:${ref.first.line}`;
  if (mine === theirs) return { status: "PASS", note: "both fail, same first error" };
  return { status: "FAIL", note: `first error: bean-check ${theirs} (${ref.message}), ours ${ours.code} at ${mine}` };
}

async function compareBeancountCase(path: string): Promise<Verdict> {
  const ref = await runBeancount(path);
  const ours = runOurs([path]);
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  const errors = ours.findings.filter((f) => f.severity === "error");
  const unsupported = ours.findings.find((f) => f.code === "unsupported-syntax");
  if (unsupported) return { status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"}` };
  if (!ref.ok) return compareFirstError(ref, errors, ours.findings);
  if (errors[0]) return { status: "FAIL", note: `ours reports ${errors[0].code} at line ${errors[0].line}, bean-check accepts` };
  const diff = firstDiff(withBasenames(ref.transactions), withBasenames(normalizeOurs(ours.transactions)));
  if (diff) return { status: "FAIL", note: diff };
  const postingsDiff = firstDiff(postingBasenames(ref.postings), postingBasenames(ours.postings));
  if (postingsDiff) return { status: "FAIL", note: `postings: ${postingsDiff}` };
  return { status: "PASS", note: "" };
}

// ───────────── the verdict ─────────────
export const isBeancountPath = (path: string): boolean => /\.(beancount|bean)$/i.test(path);

export async function compareCase(paths: string[]): Promise<Verdict> {
  const [first] = paths;
  if (first !== undefined && paths.length === 1 && isBeancountPath(first)) return compareBeancountCase(first);
  const ref = await runHledger(paths);
  const ours = runOurs(paths);
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  const errors = ours.findings.filter((f) => f.severity === "error");

  const unsupported = ours.findings.find((f) => f.code === "unsupported-syntax");
  if (!ref.ok) {
    // Only an error of our own counts as agreeing; "not supported yet" agrees with nothing.
    if (errors.some((f) => f.code !== "unsupported-syntax")) return { status: "PASS", note: "both fail" };
    if (unsupported) return { status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"} (hledger fails)` };
    return { status: "FAIL", note: `hledger fails (${ref.message}), ours reports no error` };
  }
  if (unsupported) return { status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"}` };
  if (errors[0]) return { status: "FAIL", note: `ours reports ${errors[0].code} at line ${errors[0].line}, hledger accepts` };
  const diff = firstDiff(withBasenames(ref.transactions), withBasenames(ours.transactions));
  if (diff) return { status: "FAIL", note: diff };
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

  const totals = {
    hledger: { PASS: 0, FAIL: 0, SKIP: 0 },
    beancount: { PASS: 0, FAIL: 0, SKIP: 0 },
  };
  for (const [i, v] of verdicts.entries()) {
    const c = cases[i] as { name: string; paths: string[] };
    const t = totals[isBeancountPath(c.paths[0] ?? "") ? "beancount" : "hledger"];
    t[v.status]++;
    if (!quiet || v.status !== "PASS") console.log(`${v.status} ${c.name}${v.note ? `  ${v.note}` : ""}`);
  }
  const line = (t: { PASS: number; FAIL: number; SKIP: number }) => `${t.PASS} pass, ${t.FAIL} fail, ${t.SKIP} skip`;
  const { hledger: h, beancount: b } = totals;
  if (h.PASS + h.FAIL + h.SKIP > 0 && b.PASS + b.FAIL + b.SKIP > 0) console.log(`\nhledger:   ${line(h)}\nbeancount: ${line(b)}`);
  console.log(`\n${h.PASS + b.PASS} pass, ${h.FAIL + b.FAIL} fail, ${h.SKIP + b.SKIP} skip`);
  if (h.FAIL + b.FAIL > 0) process.exitCode = 1;
}
