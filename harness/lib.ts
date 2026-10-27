// Shared by compare.ts, fuzz.ts and real.ts: run hledger and our wasm on the same files and diff
// typed values.
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
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
/** One row of `postings()`. */
export type FlatPosting = {
  date: string;
  flag: string; // the transaction's
  payee?: string;
  narration: string;
  account: string;
  kind: string;
  number: string;
  currency: string;
  file: string;
  line: number;
};
/** One `P` directive, as `prices()` lists it. */
export type PriceDirective = { date: string; commodity: string; rate: Quantity; file: string; line: number };
type Finding = { code: string; severity: "error" | "warning"; file: string; line: number; details: { construct?: string; reason?: string } };

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

/** Where an error is: the file (basename) and the line it starts on. */
export type Position = { file: string; line: number };

/**
 * Where hledger's first error is, from the first line of its stderr: `hledger: Error: FILE:LINE:COL:`
 * for a parse error or a failed assertion, `FILE:FIRST-LAST:` for a transaction that does not balance
 * (then the line is the first one); a conversion posting with a cost has no last colon. Errors that
 * name no position (a posting date on an assignment, for one) give `null`.
 */
export function firstHledgerError(stderr: string): Position | null {
  const m = /^hledger: Error: (.+?):(\d+)(?:-\d+)?(?::\d+)?:?$/.exec(stderr.split("\n")[0] ?? "");
  return m ? { file: basename(m[1] as string), line: Number(m[2]) } : null;
}

type Reference = { ok: true; transactions: Transaction[] } | { ok: false; message: string; first: Position | null };

/** `extra` options go after `print`, e.g. `--cost` or `--value=2026-01-01,EUR`. */
async function runHledger(paths: string[], extra: string[] = []): Promise<Reference> {
  const proc = Bun.spawn(["hledger", "--no-conf", ...paths.flatMap((p) => ["-f", p]), "print", "-O", "json", ...extra], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  if (exitCode !== 0) return { ok: false, message: stderr.split("\n")[0] ?? "", first: firstHledgerError(stderr) };
  return { ok: true, transactions: projectHledger(stdout) };
}

// ───────────── our side ─────────────
type Loaded = { transactions: Transaction[]; postings: FlatPosting[]; prices: PriceDirective[]; findings: Finding[] };
type Ours = Loaded | { thrown: string };

function runOurs(paths: string[]): Ours {
  const files = paths.map((name) => ({ name, content: readFileSync(name, "utf8") }));
  let ledger: ReturnType<typeof wasm.load> | undefined;
  try {
    ledger = wasm.load(JSON.stringify(files));
    return {
      transactions: JSON.parse(ledger.transactions()),
      postings: JSON.parse(ledger.postings()),
      prices: JSON.parse(ledger.prices()),
      findings: JSON.parse(ledger.check()),
    };
  } catch (error) {
    return { thrown: String(error) };
  } finally {
    ledger?.free();
  }
}

/** What `atCost()` or `value()` of our wasm gave: transactions, a refusal (the finding the core returned) or a crash. */
type Answer<T = Transaction[]> = { answer: T } | { refused: Finding } | { thrown: string };

/**
 * Load `paths` and call one of the ledger's methods that returns transactions as JSON. The wasm throws an
 * `Error` whose message is the finding as JSON when the core refuses (see crates/ledgermill-wasm/src/lib.rs).
 */
function runOursQuery<T = Transaction[]>(paths: string[], call: (ledger: ReturnType<typeof wasm.load>) => string): Answer<T> {
  const files = paths.map((name) => ({ name, content: readFileSync(name, "utf8") }));
  let ledger: ReturnType<typeof wasm.load> | undefined;
  try {
    ledger = wasm.load(JSON.stringify(files));
    return { answer: JSON.parse(call(ledger)) };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    try {
      const finding = JSON.parse(message) as Finding;
      if (typeof finding?.code === "string") return { refused: finding };
    } catch {
      // not a finding: a real crash
    }
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

/** `1.00` is `1` and `-0.50` is `-0.5`: the value without its trailing zeros. */
export function withoutTrailingZeros(quantity: string): string {
  return quantity.includes(".") ? quantity.replace(/\.?0+$/, "") : quantity;
}

/** The transactions with every quantity (amounts, prices, lot costs) reduced to its value. */
function valuesOnly(txs: Transaction[]): Transaction[] {
  const value = (q: Quantity): Quantity => ({ ...q, quantity: withoutTrailingZeros(q.quantity) });
  return txs.map((t) => ({
    ...t,
    postings: t.postings.map((p) => ({
      ...p,
      amounts: p.amounts.map((a) => ({
        ...a,
        quantity: withoutTrailingZeros(a.quantity),
        ...(a.cost ? { cost: { ...a.cost, amount: value(a.cost.amount) } } : {}),
        ...(a.lot ? { lot: { ...a.lot, cost: a.lot.cost ? value(a.lot.cost) : null } } : {}),
      })),
    })),
  }));
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
      kind: "regular",
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

type Rejection = { message: string; first: { file: string; line: number } | null };

/** What bean-check says: `null` when it accepts the journal, else its first line and its first error. */
async function runBeanCheck(path: string): Promise<Rejection | null> {
  // bean-query exits 0 even for an invalid journal, so bean-check decides.
  const check = await Bun.spawn(["bean-check", path], { stdout: "pipe", stderr: "pipe" });
  const [out, err, code] = await Promise.all([
    new Response(check.stdout).text(),
    new Response(check.stderr).text(),
    check.exited,
  ]);
  if (code === 0) return null;
  const printed = out + err;
  return { message: printed.split("\n").find((l) => l.trim()) ?? "", first: firstBeancountError(printed) };
}

/** What bean-query lists for a journal (bean-query prints errors in front of the CSV, and still lists the rest). */
async function queryBeancount(path: string): Promise<{ transactions: Transaction[]; postings: FlatPosting[] }> {
  const [entries, postings, flat] = await Promise.all([
    run(["bean-query", "-f", "csv", path, ENTRIES_QUERY]),
    run(["bean-query", "-f", "csv", path, POSTINGS_QUERY]),
    run(["bean-query", "-f", "csv", path, FLAT_POSTINGS_QUERY]),
  ]);
  return {
    transactions: projectBeancount(entries.stdout, postings.stdout),
    postings: projectFlatPostings(flat.stdout),
  };
}

/** The findings of a file the parser gave up on: ledgermill returns no transaction for it, though bean-query lists some. */
const PARSE_LEVEL = new Set(["parse-error", "invalid-account-name", "unbalanced-push", "duplicate-metadata"]);

/**
 * Whether to diff what bean-query lists for a journal bean-check rejects. Only when the file was booked
 * and bean-query's list is the one we build:
 * - no parse-level error (the parser gives up on the whole file, bean-query lists part of it);
 * - no booking error but two elided postings (that transaction is left out of bean-query's list; for the
 *   other booking errors bean-query keeps the transaction, often with its postings unbooked or missing,
 *   and we leave it out).
 * The errors of the checks (accounts, currencies, assertions, unused pads, unbalanced transactions) leave
 * the list as it is for an accepted journal, padding included.
 */
export function listsComparable(errors: Finding[]): boolean {
  return !errors.some(
    (f) => PARSE_LEVEL.has(f.code) || (f.code === "booking-error" && f.details.reason !== "ambiguous-auto-posting"),
  );
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

/**
 * hledger rejects the journal: our first error-severity finding (`check()` for hledger is in hledger's
 * own order) must be on the same file and line as its first error, when it names one. Errors of
 * ours that say "not supported yet" agree with nothing (SKIP); no error at all is a FAIL; a reference
 * error with no position (a posting date on an assignment) is matched by "both fail" only.
 */
export function compareRejection(ref: { message: string; first: Position | null }, errors: Finding[]): Verdict {
  const ours = errors[0];
  if (!ours) return { status: "FAIL", note: `hledger fails (${ref.message}), ours reports no error` };
  if (ours.code === "unsupported-syntax")
    return { status: "SKIP", note: `unsupported: ${ours.details.construct ?? "?"} (hledger fails)` };
  if (!ref.first) return { status: "PASS", note: "both fail" };
  const mine = `${basename(ours.file)}:${ours.line}`;
  const theirs = `${ref.first.file}:${ref.first.line}`;
  if (mine === theirs) return { status: "PASS", note: "both fail, same first error" };
  return { status: "FAIL", note: `first error: hledger ${theirs} (${ref.message}), ours ${ours.code} at ${mine}` };
}

/** The verdict of a Beancount case, and whether bean-check accepts the journal and we list what bean-query lists (the other reads start from there). */
async function compareBeancountCase(path: string): Promise<{ verdict: Verdict; agreed: boolean; ours?: Loaded }> {
  const rejection = await runBeanCheck(path);
  const ours = runOurs([path]);
  const no = (verdict: Verdict) => ({ verdict, agreed: false });
  if ("thrown" in ours) return no({ status: "FAIL", note: `wasm threw: ${ours.thrown}` });
  const errors = ours.findings.filter((f) => f.severity === "error");
  const unsupported = ours.findings.find((f) => f.code === "unsupported-syntax");
  if (unsupported) return no({ status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"}` });
  if (rejection) {
    const verdict = compareFirstError(rejection, errors, ours.findings);
    if (verdict.status !== "PASS" || !listsComparable(errors)) return no(verdict);
  } else if (errors[0]) {
    return no({ status: "FAIL", note: `ours reports ${errors[0].code} at line ${errors[0].line}, bean-check accepts` });
  }
  const ref = await queryBeancount(path);
  const diff = firstDiff(withBasenames(ref.transactions), withBasenames(normalizeOurs(ours.transactions)));
  if (diff) return no({ status: "FAIL", note: diff });
  const postingsDiff = firstDiff(postingBasenames(ref.postings), postingBasenames(ours.postings));
  if (postingsDiff) return no({ status: "FAIL", note: `postings: ${postingsDiff}` });
  if (rejection) return no({ status: "PASS", note: "both fail, same first error, same lists" });
  return { verdict: { status: "PASS", note: "" }, agreed: true, ours };
}

// ───────────── format ─────────────
/** `a` and `b` are the same text once every whitespace character is removed. */
export const sameWithoutWhitespace = (a: string, b: string): boolean => a.replace(/\s+/g, "") === b.replace(/\s+/g, "");

/**
 * Beancount only: how our text compares with `bean-format`'s. `equal`: the same once line endings and the final newline
 * are alike; `design`: different, for a reason the spec gives (`designDifference`); `none`: no `bean-format` output to
 * compare with (it stops, the file is not UTF-8 or has a BOM, or it changes more than whitespace).
 */
export type BeanFormatReference = "equal" | "design" | "none";

/**
 * What `format()` did to the files of a case: the verdict, and whether any file changed (else it came back as it went in).
 * `reference` is set for a Beancount file only.
 */
export type FormatCheck = { verdict: Verdict; changed: boolean; reference?: BeanFormatReference };

/** `bean-format` rewrites CR and CRLF to LF and adds the final newline; ours keeps both, so the comparison turns ours into its shape. */
export function likeBeanFormat(text: string): string {
  const lf = text.replace(/\r\n?/g, "\n");
  return lf === "" || lf.endsWith("\n") ? lf : `${lf}\n`;
}

/** A string of the text runs over a line break (`"` closes it, `\` escapes, `;` outside a string starts a comment). */
export function hasMultilineString(text: string): boolean {
  let inString = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (inString) {
      if (c === "\\") {
        if (text[i + 1] === "\n") return true;
        i++;
      } else if (c === '"') inString = false;
      else if (c === "\n") return true;
    } else if (c === '"') inString = true;
    else if (c === ";") {
      i = text.indexOf("\n", i);
      if (i < 0) return false;
    }
  }
  return false;
}

/** The account lines (an indent, then an account) all have the same indent, in characters: the case where our column is `bean-format`'s. */
export function hasOneIndent(text: string): boolean {
  const indents = new Set<number>();
  for (const m of text.matchAll(/^([ \t]+)\p{Lu}[\p{L}\p{N}-]*:/gmu)) indents.add((m[1] as string).length);
  return indents.size <= 1;
}

/**
 * Why our output may differ from `bean-format`'s although only alignment is at stake (`spec/beancount/format.md`), or null
 * when it must be equal: we return a file we refuse or cannot parse as it was (`read` is false); `bean-format` rewrites a
 * line inside a multi-line string, we do not; its column is worked out from the indents as written, ours from the usual one
 * (the same when the account lines have one indent).
 */
export function designDifference(original: string, read: boolean): string | null {
  if (!read) return "refused or unparsable, returned unchanged";
  if (hasMultilineString(original)) return "multi-line string";
  if (!hasOneIndent(original)) return "several indents";
  return null;
}

/** The text `bean-format` prints for the file, or null: a BOM or non-UTF-8 bytes (it crashes on some of them), or a non-zero exit. Its stderr is never shown. */
async function runBeanFormat(path: string, bytes: Buffer): Promise<string | null> {
  if (bytes.subarray(0, 3).equals(Buffer.from([0xef, 0xbb, 0xbf]))) return null;
  try {
    new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
  const proc = Bun.spawn(["bean-format", path], { stdout: "pipe", stderr: "ignore" });
  const [out, code] = await Promise.all([new Response(proc.stdout).text(), proc.exited]);
  return code === 0 ? out : null;
}

function firstLineDiff(theirs: string, ours: string): string {
  const [x, y] = [theirs.split("\n"), ours.split("\n")];
  const i = Array.from({ length: Math.max(x.length, y.length) }, (_, k) => k).find((k) => x[k] !== y[k]) ?? 0;
  return `line ${i + 1}: bean-format=${JSON.stringify(x[i])} ours=${JSON.stringify(y[i])}`;
}

/** What bean-check and bean-query say about one file: its rejection, and the lists (null when bean-query printed no CSV). */
async function beanSnapshot(path: string) {
  const rejection = await runBeanCheck(path);
  const lists = await queryBeancount(path).catch(() => null);
  return { rejection, lists };
}

/**
 * Format a Beancount file and check that: formatting again changes nothing, only whitespace changed, bean-check and
 * bean-query read the same from the formatted file as from the original (a rejected file is rejected at the same first
 * error), and the text equals `bean-format`'s unless `designDifference` says it may differ.
 */
async function compareBeancountFormat(path: string): Promise<FormatCheck> {
  const bytes = readFileSync(path);
  const original = bytes.toString("utf8");
  const name = basename(path);
  const fail = (note: string, changed = true): FormatCheck => ({ verdict: { status: "FAIL", note }, changed, reference: "none" });
  let formatted: string;
  try {
    formatted = wasm.format(path, original);
  } catch (error) {
    return fail(`format threw: ${error}`, false);
  }
  const changed = formatted !== original;
  if (wasm.format(path, formatted) !== formatted) return fail(`${name}: format is not idempotent`, changed);
  if (!sameWithoutWhitespace(original, formatted)) return fail(`${name}: more than whitespace changed`, changed);
  if (changed) {
    const dir = mkdtempSync(join(tmpdir(), "ledgermill-format-"));
    try {
      const [before, after] = ["original", "formatted"].map((kind) => {
        mkdirSync(join(dir, kind));
        const file = join(dir, kind, name);
        writeFileSync(file, kind === "original" ? original : formatted);
        return file;
      }) as [string, string];
      const [x, y] = [await beanSnapshot(before), await beanSnapshot(after)];
      if (!x.rejection !== !y.rejection)
        return fail(`bean-check ${x.rejection ? "rejects the original, accepts the formatted" : "accepts the original, rejects the formatted"}`);
      if (x.rejection && y.rejection) {
        const [a, b] = [x.rejection.first ?? x.rejection.message.replaceAll(dir, ""), y.rejection.first ?? y.rejection.message.replaceAll(dir, "")];
        if (JSON.stringify(a) !== JSON.stringify(b)) return fail(`first error moved: ${JSON.stringify(a)} became ${JSON.stringify(b)}`);
      }
      if (!x.lists !== !y.lists) return fail("bean-query lists rows for only one of the two");
      if (x.lists && y.lists) {
        const diff =
          firstDiff(withBasenames(x.lists.transactions), withBasenames(y.lists.transactions)) ??
          firstDiff(postingBasenames(x.lists.postings), postingBasenames(y.lists.postings));
        if (diff) return fail(`bean-query reads other rows (original, formatted) ${diff}`);
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }
  const pass = (reference: BeanFormatReference): FormatCheck => ({ verdict: { status: "PASS", note: "" }, changed, reference });
  const theirs = await runBeanFormat(path, bytes);
  if (theirs === null || !sameWithoutWhitespace(original, theirs)) return pass("none");
  const [mine, expected] = [likeBeanFormat(formatted), likeBeanFormat(theirs)];
  if (mine === expected) return pass("equal");
  const ours = runOurs([path]);
  const read = !("thrown" in ours) && !ours.findings.some((f) => f.code === "unsupported-syntax" || (f.severity === "error" && PARSE_LEVEL.has(f.code)));
  const reason = designDifference(original, read);
  if (reason) return pass("design");
  return fail(`differs from bean-format (alignment only): ${firstLineDiff(expected, mine)}`);
}

/**
 * Format each file (`format(name, content)` of the wasm; a Beancount file: `compareBeancountFormat`) and check that: formatting again changes nothing, the
 * text differs only in whitespace, and hledger reads the same transactions (source lines included) from the formatted
 * files as from the originals, or rejects both with the same first error. Both sets are written to a temp dir under
 * the same basenames (one subdirectory per file, so that a file given twice stays twice).
 */
export async function compareFormat(paths: string[]): Promise<FormatCheck> {
  const [only] = paths;
  if (only !== undefined && paths.length === 1 && isBeancountPath(only)) return compareBeancountFormat(only);
  const originals = paths.map((path) => readFileSync(path, "utf8"));
  const fail = (note: string, changed = true): FormatCheck => ({ verdict: { status: "FAIL", note }, changed });
  let formatted: string[];
  try {
    formatted = paths.map((path, i) => wasm.format(path, originals[i]));
  } catch (error) {
    return fail(`format threw: ${error}`, false);
  }
  const changed = formatted.some((text, i) => text !== originals[i]);
  for (const [i, path] of paths.entries()) {
    const name = basename(path);
    const again = wasm.format(path, formatted[i]);
    if (again !== formatted[i]) return fail(`${name}: format is not idempotent`, changed);
    if (!sameWithoutWhitespace(originals[i] as string, formatted[i] as string)) return fail(`${name}: more than whitespace changed`, changed);
  }
  const dir = mkdtempSync(join(tmpdir(), "ledgermill-format-"));
  try {
    const write = (kind: string, texts: string[]) =>
      texts.map((text, i) => {
        const folder = join(dir, kind, String(i));
        mkdirSync(folder, { recursive: true });
        const file = join(folder, basename(paths[i] as string));
        writeFileSync(file, text);
        return file;
      });
    const [before, after] = [await runHledger(write("original", originals)), await runHledger(write("formatted", formatted))];
    const clean = (message: string) => message.replaceAll(dir, "");
    let verdict: Verdict = { status: "PASS", note: "" };
    if (before.ok && after.ok) {
      const diff = firstDiff(withBasenames(before.transactions), withBasenames(after.transactions));
      if (diff) verdict = { status: "FAIL", note: `hledger reads other transactions (hledger=original, ours=formatted) ${diff}` };
    } else if (!before.ok && !after.ok) {
      const [x, y] = [before.first ?? clean(before.message), after.first ?? clean(after.message)];
      if (JSON.stringify(x) !== JSON.stringify(y)) verdict = { status: "FAIL", note: `first error moved: ${JSON.stringify(x)} became ${JSON.stringify(y)}` };
    } else {
      verdict = { status: "FAIL", note: `hledger ${before.ok ? "accepts the original, rejects the formatted" : "rejects the original, accepts the formatted"}: ${before.ok ? clean((after as { message: string }).message) : clean(before.message)}` };
    }
    return { verdict, changed };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// ───────────── the verdict ─────────────
export const isBeancountPath = (path: string): boolean => /\.(beancount|bean)$/i.test(path);

/**
 * Compare one case. `ignorePlaces` compares quantities by value only: for the journals where hledger
 * adds sub-accounts in an order we do not know, so that a sum can differ in its places (see
 * spec/hledger/assignments.md).
 */
export async function compareCase(paths: string[], ignorePlaces = false): Promise<Verdict> {
  return (await compareRaw(paths, ignorePlaces)).verdict;
}

/** The verdict of a hledger case, and whether hledger accepts the journal and we agree with it (the other comparisons start from there). */
async function compareRaw(paths: string[], ignorePlaces: boolean): Promise<{ verdict: Verdict; agreed: boolean; ours?: Loaded }> {
  const [first] = paths;
  if (first !== undefined && paths.length === 1 && isBeancountPath(first)) return compareBeancountCase(first);
  const ref = await runHledger(paths);
  const ours = runOurs(paths);
  const no = (verdict: Verdict) => ({ verdict, agreed: false });
  if ("thrown" in ours) return no({ status: "FAIL", note: `wasm threw: ${ours.thrown}` });
  const errors = ours.findings.filter((f) => f.severity === "error");

  const unsupported = ours.findings.find((f) => f.code === "unsupported-syntax");
  if (!ref.ok) return no(compareRejection(ref, errors));
  if (unsupported) return no({ status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"}` });
  if (errors[0]) return no({ status: "FAIL", note: `ours reports ${errors[0].code} at line ${errors[0].line}, hledger accepts` });
  const [theirs, mine] = [withBasenames(ref.transactions), withBasenames(ours.transactions)];
  const diff = ignorePlaces ? firstDiff(valuesOnly(theirs), valuesOnly(mine)) : firstDiff(theirs, mine);
  if (diff) return no({ status: "FAIL", note: diff });
  return { verdict: { status: "PASS", note: ignorePlaces ? "same values (places may differ)" : "" }, agreed: true, ours };
}

// ───────────── postings and prices ─────────────
/** One row of `hledger print --location -O csv`, as `spec/hledger/postings-flat.md` reads it. */
export type HledgerRow = {
  txn: number; // `txnidx`
  date: string;
  status: string;
  description: string;
  account: string; // without brackets
  kind: string;
  number: string; // exact, with a point
  commodity: string;
  file: string; // the file as given to us
  first: number; // the transaction's header line
  last: number; // its last line
};

/** The kind of a CSV account and its name: `(a)` is virtual, `[a]` balanced virtual, anything else regular. */
export function csvAccount(text: string): { account: string; kind: string } {
  if (text.length >= 2 && text.startsWith("(") && text.endsWith(")")) return { account: text.slice(1, -1), kind: "virtual" };
  if (text.length >= 2 && text.startsWith("[") && text.endsWith("]")) return { account: text.slice(1, -1), kind: "balanced-virtual" };
  return { account: text, kind: "regular" };
}

/**
 * The file name we were given for the path hledger resolved: the same string when it is absolute, else the
 * one the path ends with after a `/` (a relative name is joined to the working directory, unnormalized).
 */
export function givenName(resolved: string, given: string[]): string {
  const hit = given.find((g) => g === resolved) ?? given.find((g) => resolved.endsWith(`/${g}`));
  if (hit === undefined) throw new Error(`hledger file ${resolved} is none of ${given.join(", ")}`);
  return hit;
}

/** Decode `print --location -O csv`: the last line of `comment` is `location: FILE:FIRST-LAST`. */
export function decodeHledgerPostings(csv: string, given: string[]): HledgerRow[] {
  const [names = [], ...data] = decodeCsv(csv);
  return data
    .filter((cells) => cells.length > 1)
    .map((cells) => {
      const r = Object.fromEntries(names.map((n, i) => [n, cells[i] ?? ""])) as Record<string, string>;
      const location = (r.comment ?? "").split("\n").at(-1) ?? "";
      const m = /^location: (.*):(\d+)-(\d+)$/s.exec(location);
      if (!m) throw new Error(`no location in comment: ${r.comment}`);
      return {
        txn: Number(r.txnidx),
        date: r.date as string,
        status: r.status as string,
        description: r.description as string,
        ...csvAccount(r.account as string),
        number: (r.amount as string).replace(",", "."),
        commodity: r.commodity as string,
        file: givenName(m[1] as string, given),
        first: Number(m[2]),
        last: Number(m[3]),
      };
    });
}

/**
 * Our rows against hledger's, row by row: everything the CSV has (date, status, description, account and kind, exact
 * quantity, commodity, file), and for the line hledger does not give: it lies inside the transaction (`FIRST+1..=LAST`)
 * and never goes back within one transaction (rows are grouped by `txnidx`). Null when equal.
 */
export function diffPostings(theirs: HledgerRow[], ours: FlatPosting[]): string | null {
  if (theirs.length !== ours.length) return `length hledger=${theirs.length} ours=${ours.length}`;
  for (const [i, t] of theirs.entries()) {
    const o = ours[i] as FlatPosting;
    const expected = {
      date: t.date,
      flag: t.status,
      narration: t.description,
      account: t.account,
      kind: t.kind,
      number: t.number,
      currency: t.commodity,
      file: t.file,
    };
    const { line, payee, ...rest } = o;
    const d = firstDiff(expected, payee === undefined ? rest : { ...rest, payee }, `[${i}]`);
    if (d) return d;
    if (line <= t.first || line > t.last) return `[${i}].line: ${line} is outside ${t.first + 1}..=${t.last} (${t.description})`;
    const before = theirs[i - 1];
    if (before && before.txn === t.txn && line < (ours[i - 1] as FlatPosting).line)
      return `[${i}].line: ${line} goes back inside a transaction (after ${(ours[i - 1] as FlatPosting).line})`;
  }
  return null;
}

async function runHledgerText(paths: string[], args: string[]): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  const proc = Bun.spawn(["hledger", "--no-conf", ...paths.flatMap((p) => ["-f", p]), ...args], { stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  return { stdout, stderr, exitCode };
}

/** `Ledger::postings()` against `hledger print --location -O csv`, for a journal hledger accepts. */
export async function comparePostings(paths: string[], ours: FlatPosting[]): Promise<Verdict> {
  const ref = await runHledgerText(paths, ["print", "--location", "-O", "csv"]);
  if (ref.exitCode !== 0) return { status: "FAIL", note: `hledger fails (${ref.stderr.split("\n")[0]}) where plain print passes` };
  const diff = diffPostings(decodeHledgerPostings(ref.stdout, paths), ours);
  return diff ? { status: "FAIL", note: diff } : { status: "PASS", note: "" };
}

/** One row of `hledger prices` (the second run): `P DATE FROM RATE TO`, names without their quotes. */
export type PriceRow = { date: string; commodity: string; rate: Quantity };

const unquote = (name: string): string => (name.startsWith('"') ? name.slice(1, -1) : name);

/**
 * Decode the rows of `hledger prices` as the second run of `spec/hledger/prices-list.md` prints them (the rate as
 * `NUMBER COMMODITY`, a quoted name keeping its quotes). The commodity before the rate is accepted too.
 */
export function decodePrices(text: string): PriceRow[] {
  return text
    .split("\n")
    .filter((l) => l !== "")
    .map((l) => {
      const m = /^P (\S+) ("[^"]*"|[^ ]*) (.*)$/.exec(l);
      if (!m) throw new Error(`not a P row: ${l}`);
      const rate = m[3] as string;
      const after = /^(-?[0-9][0-9.]*)(?: (.*))?$/.exec(rate);
      const before = /^("[^"]*"|[^\s\d-]\S*) (-?[0-9][0-9.]*)$/.exec(rate);
      if (!after && !before) throw new Error(`cannot read the rate of: ${l}`);
      const [quantity, commodity] = after ? [after[1], after[2] ?? ""] : [before?.[2], before?.[1]];
      return { date: m[1] as string, commodity: unquote(m[2] as string), rate: { commodity: unquote(commodity as string), quantity: quantity as string } };
    });
}

/** The commodity of the rate column of a first-run row (`RATE COMM` or `COMM RATE`; a quoted name keeps its quotes). */
export function rateCommodity(row: string): string {
  const rate = row.replace(/^P \S+ ("[^"]*"|[^ ]*) ?/, "");
  return /^[-+0-9.,]/.test(rate) ? rate.replace(/^[-+0-9., ]*[0-9.,] ?/, "") : rate.replace(/ ?[-+]? ?[0-9][-+0-9.,eE ]*$/, "");
}

/** `hledger prices`, twice: the second with `-c "1.000000000000000000 COMM"` per rate commodity (spec/hledger/show-prices.sh). */
async function runHledgerPrices(paths: string[]): Promise<{ ok: true; rows: PriceRow[] } | { ok: false; message: string }> {
  const first = await runHledgerText(paths, ["prices", "--color=n"]);
  if (first.exitCode !== 0) return { ok: false, message: first.stderr.split("\n")[0] ?? "" };
  const lines = first.stdout.split("\n").filter((l) => l !== "");
  const commodities = [...new Set(lines.map(rateCommodity))];
  const styles = commodities.flatMap((c) => ["-c", c === "" ? "1.000000000000000000" : `1.000000000000000000 ${c}`]);
  const second = await runHledgerText(paths, ["prices", "--color=n", ...styles]);
  if (second.exitCode !== 0) return { ok: false, message: second.stderr.split("\n")[0] ?? "" };
  const rows = decodePrices(second.stdout);
  if (rows.length !== lines.length) return { ok: false, message: `the second run lists ${rows.length} rows, the first ${lines.length}` };
  return { ok: true, rows };
}

/**
 * Our `P` directives against hledger's, in order: date, commodity, rate commodity and the rate as the second run prints
 * it (digits and places as written; see "what -c run 2 changes"). The file and the line are ours: the file is one we were
 * given and the line holds a `P`. `texts` are the files, in the order of `paths`.
 */
export function diffPrices(theirs: PriceRow[], ours: PriceDirective[], paths: string[], texts: string[]): string | null {
  if (theirs.length !== ours.length) return `length hledger=${theirs.length} ours=${ours.length}`;
  for (const [i, t] of theirs.entries()) {
    const o = ours[i] as PriceDirective;
    const d = firstDiff(t, { date: o.date, commodity: o.commodity, rate: o.rate }, `[${i}]`);
    if (d) return d;
    const text = texts[paths.indexOf(o.file)];
    if (text === undefined) return `[${i}].file: ${o.file} is none of the files given`;
    if (!(text.split("\n")[o.line - 1] ?? "").startsWith("P")) return `[${i}].line: line ${o.line} of ${basename(o.file)} is not a P line`;
  }
  return null;
}

/** `Ledger::prices()` against `hledger prices`, for a journal hledger accepts. */
export async function comparePrices(paths: string[], ours: PriceDirective[]): Promise<Verdict> {
  const ref = await runHledgerPrices(paths);
  if (!ref.ok) return { status: "FAIL", note: `hledger fails (${ref.message}) where plain print passes` };
  const diff = diffPrices(ref.rows, ours, paths, paths.map((p) => readFileSync(p, "utf8")));
  return diff ? { status: "FAIL", note: diff } : { status: "PASS", note: "" };
}

// ───────────── --cost and --value ─────────────
/** A `--value=DATE,COMMODITY` question. */
export type Query = { date: string; commodity: string };

/**
 * Compare `hledger print -O json ...extra` with what our wasm answers through `call`, for a journal hledger
 * accepts. An error of ours that says "not supported yet" is a SKIP (a refusal; see the Gaps of spec/hledger/cost.md and
 * value-arithmetic.md); any other error, or a hledger failure, is a FAIL.
 */
async function compareDerived(
  paths: string[],
  extra: string[],
  call: (ledger: ReturnType<typeof wasm.load>) => string,
  ignorePlaces: boolean,
): Promise<Verdict> {
  const ref = await runHledger(paths, extra);
  if (!ref.ok) return { status: "FAIL", note: `hledger fails (${ref.message}) where plain print passes` };
  const ours = runOursQuery(paths, call);
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  if ("refused" in ours) {
    const { code, details, message } = ours.refused;
    if (code === "unsupported-syntax") return { status: "SKIP", note: `unsupported: ${details.construct ?? "?"}` };
    return { status: "FAIL", note: `ours returns ${code} (${message}), hledger accepts` };
  }
  const [theirs, mine] = [withBasenames(ref.transactions), withBasenames(ours.answer)];
  const diff = ignorePlaces ? firstDiff(valuesOnly(theirs), valuesOnly(mine)) : firstDiff(theirs, mine);
  return diff ? { status: "FAIL", note: diff } : { status: "PASS", note: "" };
}

/** `--cost`: `Ledger::at_cost()` against `hledger print --cost`. Only for a journal hledger accepts. */
export const compareCost = (paths: string[], ignorePlaces = false): Promise<Verdict> =>
  compareDerived(paths, ["--cost"], (ledger) => ledger.atCost(), ignorePlaces);

/** `--value=DATE,COMMODITY`: `Ledger::value(date, commodity)` against `hledger print --value=DATE,COMMODITY`. */
export const compareValue = (paths: string[], { date, commodity }: Query, ignorePlaces = false): Promise<Verdict> =>
  compareDerived(paths, [`--value=${date},${commodity}`], (ledger) => ledger.value(date, commodity), ignorePlaces);

const isoDate = (t: number): string => new Date(t).toISOString().slice(0, 10);
const DAY = 86_400_000;

/** The `P` lines of some journal texts: date (`YYYY-MM-DD`), the commodity priced and the commodity of the price. */
export function priceLines(texts: string[]): { date: string; from: string; to: string }[] {
  const found: { date: string; from: string; to: string }[] = [];
  const line = /^P\s*(\d{4})([-/.])(\d{1,2})\2(\d{1,2})\s+(?:\d{1,2}:\d{2}(?::\d{2})?\s+)?("[^"]*"|[^\s\d"-]\S*)\s+(.*)$/;
  for (const text of texts)
    for (const l of text.split("\n")) {
      const m = line.exec(l);
      if (!m) continue;
      const [, y, , mo, d, from, rest] = m as unknown as string[];
      const date = `${y}-${(mo as string).padStart(2, "0")}-${(d as string).padStart(2, "0")}`;
      // A real date only: `new Date` would roll 02-30 over to March.
      if (isoDate(Date.parse(`${date}T00:00:00Z`)) !== date) continue;
      const to = /"([^"]+)"/.exec(rest as string)?.[1] ?? /[^\d\s.,;+\-"@=]+/.exec(rest as string)?.[0] ?? "";
      found.push({ date, from: (from as string).replace(/^"|"$/g, ""), to });
    }
  return found;
}

/**
 * `--value` questions for journals with `P` lines: one day before the first price date, each distinct price date
 * (at most 6, spread evenly when there are more) and 30 days after the last; in EUR, USD, the commodity of
 * the first price and the first commodity priced. No `P` line, no question.
 */
export function valueQueries(texts: string[]): Query[] {
  const prices = priceLines(texts).filter((p) => p.date >= "1000-01-01" && p.date <= "9998-12-31");
  const dates = [...new Set(prices.map((p) => p.date))].sort();
  const [first, last] = [dates[0], dates[dates.length - 1]];
  if (first === undefined || last === undefined) return [];
  const spread = dates.length <= 6 ? dates : Array.from({ length: 6 }, (_, i) => dates[Math.floor((i * (dates.length - 1)) / 5)] as string);
  const at = (date: string, days: number) => isoDate(Date.parse(`${date}T00:00:00Z`) + days * DAY);
  const when = [...new Set([at(first, -1), ...spread, at(last, 30)])];
  const [firstPrice] = prices;
  const targets = [...new Set(["EUR", "USD", firstPrice?.to ?? "", firstPrice?.from ?? ""])].filter((c) => c !== "");
  return when.flatMap((date) => targets.map((commodity) => ({ date, commodity })));
}

// ───────────── holdings ─────────────
/** A `holdings()` question: values at `commodity`; `end` is `-e` (exclusive), null for none. */
export type HoldingQuestion = { end: string | null; commodity: string };

/** What our `holdings()` gives (crates/ledgermill/src/model.rs; JSON field names are the Rust ones). */
export type Holding = {
  account: string;
  commodity: string;
  units: string;
  cost: Quantity[];
  value: Quantity[];
  cost_value: Quantity[];
};

/** An exact decimal: `-12.50` is mantissa -1250, 2 places. */
type Dec = { m: bigint; p: number };

function parseDecimal(text: string): Dec {
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(text);
  if (!m) throw new Error(`not a decimal: ${text}`);
  const fraction = m[3] ?? "";
  return { m: BigInt(`${m[1]}${m[2]}${fraction}`), p: fraction.length };
}

/** The sum of exact decimals as a string without trailing zeros (`0` for zero): numbers compared by value. */
export function decimalSum(parts: Dec[]): string {
  const places = Math.max(0, ...parts.map((d) => d.p));
  let sum = 0n;
  for (const d of parts) sum += d.m * 10n ** BigInt(places - d.p);
  let text = decimalString(sum, places);
  if (places > 0) text = text.replace(/\.?0+$/, "");
  return text === "-0" ? "0" : text;
}

/** `Amounts of hledger's balance -O json` per account and commodity, added up whatever their costs; zero sums left out. */
export function decodeBalance(json: string): Map<string, Map<string, string>> {
  const exact = json.replace(/"decimalMantissa":\s*([-+\d.eE]+)/g, '"decimalMantissa":"$1"');
  const [rows] = JSON.parse(exact) as [J[], J];
  const out = new Map<string, Map<string, string>>();
  for (const row of rows) {
    const parts = new Map<string, Dec[]>();
    for (const a of row[3] as J[]) {
      const list = parts.get(a.acommodity) ?? [];
      list.push({ m: exactInteger(a.aquantity.decimalMantissa), p: a.aquantity.decimalPlaces });
      parts.set(a.acommodity, list);
    }
    const sums = new Map<string, string>();
    for (const [commodity, list] of parts) {
      const sum = decimalSum(list);
      if (sum !== "0") sums.set(commodity, sum);
    }
    if (sums.size > 0) out.set(row[0] as string, sums);
  }
  return out;
}

/** The commodity of each name of a `cur:` pattern that matches it and only it: regex characters escaped, anchored. */
export const currencyQuery = (commodity: string): string => `cur:^${commodity.replace(/[\\.^$|?*+()[\]{}]/g, "\\$&")}$`;

/** One side of a holdings comparison: the four positions of an (account, commodity), as sorted `COMMODITY QUANTITY` lists. */
export type HoldingRow = { units: string; cost: string[]; value: string[]; costValue: string[] };

const sortedLines = (m: Map<string, string> | undefined): string[] =>
  [...(m ?? [])].map(([commodity, quantity]) => `${commodity}\0${quantity}`).sort();

const emptyRow = (): HoldingRow => ({ units: "0", cost: [], value: [], costValue: [] });
const rowKey = (account: string, commodity: string): string => `${account}\0${commodity}`;
const isEmptyRow = (r: HoldingRow): boolean => r.units === "0" && !r.cost.length && !r.value.length && !r.costValue.length;

/** Our holdings as rows: quantities by value, exact zeros left out (the reference has no row for them). */
export function oursAsRows(holdings: Holding[]): Map<string, HoldingRow> {
  const lines = (list: Quantity[]) =>
    list
      .map((q) => ({ commodity: q.commodity, quantity: decimalSum([parseDecimal(q.quantity)]) }))
      .filter((q) => q.quantity !== "0")
      .map((q) => `${q.commodity}\0${q.quantity}`)
      .sort();
  const rows = new Map<string, HoldingRow>();
  for (const h of holdings) {
    const row = {
      units: decimalSum([parseDecimal(h.units)]),
      cost: lines(h.cost),
      value: lines(h.value),
      costValue: lines(h.cost_value),
    };
    if (!isEmptyRow(row)) rows.set(rowKey(h.account, h.commodity), row);
  }
  return rows;
}

const shown = (lines: string[]): string => (lines.length ? lines.map((l) => l.replace("\0", " ")).join(", ") : "-");

/** The first difference between the reference's rows and ours (`ACCOUNT [COMMODITY] field: hledger=… ours=…`), or null. */
export function diffHoldingRows(theirs: Map<string, HoldingRow>, ours: Map<string, HoldingRow>, reference = "hledger"): string | null {
  for (const key of [...new Set([...theirs.keys(), ...ours.keys()])].sort()) {
    const [a, b] = [theirs.get(key) ?? emptyRow(), ours.get(key) ?? emptyRow()];
    const [account, commodity] = key.split("\0");
    const at = `${account} [${commodity}]`;
    if (a.units !== b.units) return `${at} units: ${reference}=${a.units} ours=${b.units}`;
    for (const [field, x, y] of [
      ["cost", a.cost, b.cost],
      ["value", a.value, b.value],
      ["costValue", a.costValue, b.costValue],
    ] as const)
      if (x.join("\n") !== y.join("\n")) return `${at} ${field}: ${reference}=${shown(x)} ours=${shown(y)}`;
  }
  return null;
}

const codePoints = (s: string): number[] => Array.from(s, (c) => c.codePointAt(0) as number);
function compareCodePoints(a: string, b: string): number {
  const [x, y] = [codePoints(a), codePoints(b)];
  for (let i = 0; i < Math.min(x.length, y.length); i++) if (x[i] !== y[i]) return (x[i] as number) - (y[i] as number);
  return x.length - y.length;
}

/** The order `holdings()` promises: the account's `:`-separated names by code point, then the commodity by code point. */
export function holdingsInOrder(holdings: Holding[]): string | null {
  const before = (a: Holding, b: Holding): number => {
    const [x, y] = [a.account.split(":"), b.account.split(":")];
    for (let i = 0; i < Math.min(x.length, y.length); i++) {
      const c = compareCodePoints(x[i] as string, y[i] as string);
      if (c !== 0) return c;
    }
    return x.length - y.length || compareCodePoints(a.commodity, b.commodity);
  };
  for (let i = 1; i < holdings.length; i++) {
    const [a, b] = [holdings[i - 1] as Holding, holdings[i] as Holding];
    if (before(a, b) >= 0) return `order: ${a.account} [${a.commodity}] comes before ${b.account} [${b.commodity}]`;
  }
  return null;
}

/** Today's date on this machine, which is hledger's clock. */
export function localToday(now = new Date()): string {
  const two = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
}

/** hledger runs of one journal, kept: the units and the `--cost` runs do not depend on the valuation commodity. */
export type RunCache = Map<string, Promise<string>>;

async function cached(cache: RunCache, paths: string[], args: string[]): Promise<string> {
  const key = args.join("\0");
  let run = cache.get(key);
  if (!run) {
    run = runHledgerText(paths, args).then((r) => {
      if (r.exitCode !== 0) throw new Error(`hledger ${args.join(" ")} fails: ${r.stderr.split("\n")[0]}`);
      return r.stdout;
    });
    cache.set(key, run);
  }
  return run;
}

/** Every commodity of a journal, as `hledger commodities` lists them (names without quotes). */
async function journalCommodities(paths: string[], cache: RunCache): Promise<string[]> {
  return (await cached(cache, paths, ["commodities"])).split("\n").filter((l) => l !== "").map(unquote);
}

/** Two different commodities that one `cur:` pattern matches: it is case-insensitive, so `X` also finds `x`. */
const foldsTogether = (a: string, b: string): boolean => a !== b && (a.toLowerCase() === b.toLowerCase() || a.toUpperCase() === b.toUpperCase());

type Holdings = { holdings: Holding[] } | { refused: Finding } | { thrown: string };

function runOursHoldings(paths: string[], query: { commodity: string; end: string | null; today: string }): Holdings {
  const answer = runOursQuery<Holding[]>(paths, (ledger) => ledger.holdings(JSON.stringify(query)));
  return "answer" in answer ? { holdings: answer.answer } : answer;
}

/**
 * `Ledger::holdings()` against `hledger balance --layout=tidy -O json --real` (spec/hledger/holdings.md): the units
 * of every account, then for each commodity C that is held, `--cost`, `--value=end,V` and both, with `cur:^C$`.
 * Units and positions by value. A commodity whose `cur:` pattern also matches another (`X`, `x`) cannot be asked
 * alone: the rows of the other commodities are compared, and the verdict is a SKIP if they agree.
 */
export async function compareHoldings(paths: string[], q: HoldingQuestion, cache: RunCache, today = localToday()): Promise<Verdict> {
  const ours = runOursHoldings(paths, { commodity: q.commodity, end: q.end, today });
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  if ("refused" in ours) {
    const { code, details, message } = ours.refused;
    if (code === "unsupported-syntax") return { status: "SKIP", note: `unsupported: ${details.construct ?? "?"}` };
    return { status: "FAIL", note: `ours returns ${code} (${message}), hledger accepts` };
  }
  const order = holdingsInOrder(ours.holdings);
  if (order) return { status: "FAIL", note: order };
  try {
    const base = ["balance", "--layout=tidy", "-O", "json", "--real", "-E", ...(q.end ? ["-e", q.end] : [])];
    const units = decodeBalance(await cached(cache, paths, base));
    const all = await journalCommodities(paths, cache);
    const commodities = new Set([...ours.holdings.map((h) => h.commodity), ...[...units.values()].flatMap((m) => [...m.keys()])]);
    const skipped = new Set([...commodities].filter((c) => all.some((other) => foldsTogether(c, other))));
    const theirs = new Map<string, HoldingRow>();
    const at = (account: string, commodity: string): HoldingRow => {
      const key = rowKey(account, commodity);
      return theirs.get(key) ?? (theirs.set(key, emptyRow()), theirs.get(key) as HoldingRow);
    };
    for (const [account, sums] of units)
      for (const [commodity, sum] of sums) if (!skipped.has(commodity)) at(account, commodity).units = sum;
    const value = `--value=end,${q.commodity}`;
    for (const commodity of commodities) {
      if (skipped.has(commodity)) continue;
      const cur = currencyQuery(commodity);
      const runs = [
        ["cost", ["--cost"]],
        ["value", [value]],
        ["costValue", ["--cost", value]],
      ] as const;
      for (const [field, options] of runs)
        for (const [account, sums] of decodeBalance(await cached(cache, paths, [...base, ...options, cur])))
          at(account, commodity)[field] = sortedLines(sums);
    }
    for (const [key, row] of theirs) if (isEmptyRow(row)) theirs.delete(key);
    const mine = oursAsRows(ours.holdings.filter((h) => !skipped.has(h.commodity)));
    const diff = diffHoldingRows(theirs, mine);
    if (diff) return { status: "FAIL", note: diff };
    return skipped.size ? { status: "SKIP", note: `cur: cannot tell ${[...skipped].join(", ")} from another commodity` } : { status: "PASS", note: "" };
  } catch (error) {
    return { status: "FAIL", note: String(error instanceof Error ? error.message : error) };
  }
}

/** The dates of the transactions of some journal texts (`YYYY-MM-DD`, real dates only), sorted. */
function transactionDates(texts: string[]): string[] {
  const dates: string[] = [];
  for (const text of texts)
    for (const l of text.split("\n")) {
      const m = /^(\d{4})([-/.])(\d{1,2})\2(\d{1,2})(?=[\s=*!]|$)/.exec(l);
      if (!m) continue;
      const date = `${m[1]}-${(m[3] as string).padStart(2, "0")}-${(m[4] as string).padStart(2, "0")}`;
      if (date >= "1000-01-01" && date <= "9998-12-31" && isoDate(Date.parse(`${date}T00:00:00Z`)) === date) dates.push(date);
    }
  return dates.sort();
}

/**
 * The holdings questions for a journal: no `end` and up to three end dates around its `P` lines (the first
 * `--value` date, the middle one and the last), valued in EUR, USD and the first commodity priced. With no `P` line:
 * no `end` and the day after its middle transaction, in EUR.
 */
export function holdingQuestions(texts: string[]): HoldingQuestion[] {
  const values = valueQueries(texts);
  if (values.length === 0) {
    const dates = transactionDates(texts);
    const middle = dates[Math.floor(dates.length / 2)];
    const ends = middle === undefined ? [null] : [null, isoDate(Date.parse(`${middle}T00:00:00Z`) + DAY)];
    return ends.map((end) => ({ end, commodity: "EUR" }));
  }
  const dates = [...new Set(values.map((v) => v.date))];
  const ends = [...new Set([dates[0], dates[Math.floor(dates.length / 2)], dates[dates.length - 1]])] as string[];
  const [first] = priceLines(texts);
  const commodities = [...new Set(["EUR", "USD", first?.from ?? ""])].filter((c) => c !== "");
  return [null, ...ends].flatMap((end) => commodities.map((commodity) => ({ end, commodity })));
}

// ───────────── Plainledger's own holdings reads ─────────────
const VALUATION_ALIASES: Record<string, string[]> = { EUR: ["EUR", "€"], USD: ["USD", "$", "US$"] };
const isValuationAlias = (commodity: string, valuation: string): boolean =>
  (VALUATION_ALIASES[valuation.toUpperCase()] ?? [valuation]).some((a) => a.toUpperCase() === commodity.trim().toUpperCase());
const startsWithPrefix = (account: string, prefixes: string[]): boolean =>
  prefixes.some((p) => account.toLowerCase().startsWith(p.toLowerCase()));

/** One row of the tidy CSV as Plainledger reads it: commodity, the number, and the places the CSV wrote. */
type CsvRow = { account: string; commodity: string; value: number; places: number };

function csvBalanceRows(text: string): CsvRow[] {
  const [header, ...rows] = decodeCsv(text);
  if (!header) return [];
  const [iAccount, iCommodity, iValue] = ["account", "commodity", "value"].map((h) => header.indexOf(h)) as [number, number, number];
  return rows
    .filter((r) => r.length > iValue)
    .map((r) => {
      const value = (r[iValue] as string).trim().replace(",", ".");
      return { account: (r[iAccount] as string).trim(), commodity: (r[iCommodity] as string).trim(), value: Number(value), places: value.split(".")[1]?.length ?? 0 };
    });
}

/** What Plainledger makes of each (account, C): cost basis and market value in V, null as it does. */
type Figures = Map<string, { costBasis: number | null; marketValue: number | null; slack: number }>;

/**
 * Plainledger's reads (packages/ledger-domain/src/holdings/hledgerHoldings.ts): the units run, then for each held
 * commodity `balance --layout=tidy -O csv -- PREFIXES cur:C` with `--cost`, `--cost --value=end,V` and `--value=end,V`;
 * `real` adds `--real` to those three (Plainledger's units run has it already). Accounts with a space are left out:
 * Plainledger looks them up by their first word, which is its own quirk.
 */
async function plainledgerFigures(paths: string[], prefixes: string[], valuation: string, real: boolean, cache: RunCache): Promise<Figures> {
  const run = (args: string[]) => cached(cache, paths, ["balance", "--layout=tidy", "-O", "csv", ...args]);
  const held = new Set<string>();
  for (const r of csvBalanceRows(await run(["--real"])))
    if (r.commodity && startsWithPrefix(r.account, prefixes) && !isValuationAlias(r.commodity, valuation) && !/\s/.test(r.account) && Math.abs(r.value) > 1e-8)
      held.add(rowKey(r.account, r.commodity));
  const figures: Figures = new Map();
  for (const commodity of new Set([...held].map((k) => k.split("\0")[1] as string))) {
    const flags = real ? ["--real"] : [];
    const query = ["--", ...prefixes, `cur:${commodity}`];
    const sums = (rows: CsvRow[]) => {
      const byAccount = new Map<string, { value: number; slack: number }>();
      for (const r of rows)
        if (r.commodity && isValuationAlias(r.commodity, valuation) && Number.isFinite(r.value)) {
          const so_far = byAccount.get(r.account) ?? { value: 0, slack: 0 };
          byAccount.set(r.account, { value: so_far.value + r.value, slack: so_far.slack + 0.5 * 10 ** -r.places });
        }
      return byAccount;
    };
    const rawCost = csvBalanceRows(await run([...flags, "--cost", ...query]));
    const costBasis = sums(csvBalanceRows(await run([...flags, "--cost", `--value=end,${valuation}`, ...query])));
    const market = sums(csvBalanceRows(await run([...flags, `--value=end,${valuation}`, ...query])));
    for (const key of held) {
      const [account, c] = key.split("\0") as [string, string];
      if (c !== commodity) continue;
      const missingCost = rawCost.some((r) => r.account === account && r.commodity.toUpperCase() === c.toUpperCase() && Math.abs(r.value) > 1e-8);
      const cb = costBasis.get(account);
      const mv = market.get(account);
      figures.set(key, { costBasis: missingCost || !cb ? null : cb.value, marketValue: mv ? mv.value : null, slack: (cb?.slack ?? 0) + (mv?.slack ?? 0) });
    }
  }
  return figures;
}

/** The same figures from our holdings, filtered as Plainledger filters: tracked prefix, not V, not an exact zero. */
function figuresFromOurs(holdings: Holding[], prefixes: string[], valuation: string): Figures {
  const inValuation = (list: Quantity[]): number | null => {
    const mine = list.filter((q) => isValuationAlias(q.commodity, valuation));
    return mine.length ? mine.reduce((n, q) => n + Number(q.quantity), 0) : null;
  };
  const figures: Figures = new Map();
  for (const h of holdings) {
    if (!h.commodity || !startsWithPrefix(h.account, prefixes) || isValuationAlias(h.commodity, valuation) || /\s/.test(h.account)) continue;
    if (Math.abs(Number(h.units)) <= 1e-8) continue;
    const missingCost = h.cost.some((q) => q.commodity.toUpperCase() === h.commodity.toUpperCase() && Math.abs(Number(q.quantity)) > 1e-8);
    figures.set(rowKey(h.account, h.commodity), {
      costBasis: missingCost ? null : inValuation(h.cost_value),
      marketValue: inValuation(h.value),
      slack: 0,
    });
  }
  return figures;
}

/** The first difference between Plainledger's figures and ours, or null; the CSV's rounding is allowed for. */
function diffFigures(theirs: Figures, ours: Figures): string | null {
  for (const key of [...new Set([...theirs.keys(), ...ours.keys()])].sort()) {
    const [a, b] = [theirs.get(key), ours.get(key)];
    const at = key.replace("\0", " [") + "]";
    if (!a || !b) return `${at}: held in ${a ? "Plainledger's reads" : "ours"} only`;
    for (const field of ["costBasis", "marketValue"] as const) {
      const [x, y] = [a[field], b[field]];
      const same = x === null || y === null ? x === y : Math.abs(x - y) <= a.slack + 1e-9;
      if (!same) return `${at} ${field}: Plainledger=${x} ours=${y}`;
    }
  }
  return null;
}

/** Plainledger's holdings reads on one journal: V = EUR, prefixes `assets` and `liabilities`, no end. */
export type PlainledgerVerdict = { verdict: Verdict; changes: string | null };

/**
 * Compare Plainledger's figures (the commands it runs, with `--real` added to the per-commodity runs) with ours: it
 * must agree. Then run the commands as Plainledger runs them today (no `--real` there): where the figures differ,
 * `changes` says how. That happens for a virtual posting on a tracked account, which `holdings()` ignores.
 */
export async function comparePlainledger(paths: string[], cache: RunCache): Promise<PlainledgerVerdict> {
  const [prefixes, valuation] = [["assets", "liabilities"], "EUR"];
  const ours = runOursHoldings(paths, { commodity: valuation, end: null, today: localToday() });
  if ("thrown" in ours) return { verdict: { status: "FAIL", note: `wasm threw: ${ours.thrown}` }, changes: null };
  if ("refused" in ours) {
    const { code, details } = ours.refused;
    const status = code === "unsupported-syntax" ? "SKIP" : "FAIL";
    return { verdict: { status, note: `${code} ${details.construct ?? ""}`.trim() }, changes: null };
  }
  try {
    const mine = figuresFromOurs(ours.holdings, prefixes, valuation);
    const real = await plainledgerFigures(paths, prefixes, valuation, true, cache);
    const diff = diffFigures(real, mine);
    if (diff) return { verdict: { status: "FAIL", note: `plainledger: ${diff}` }, changes: null };
    const today = await plainledgerFigures(paths, prefixes, valuation, false, cache);
    return { verdict: { status: "PASS", note: "" }, changes: diffFigures(today, mine) };
  } catch (error) {
    return { verdict: { status: "FAIL", note: `plainledger: ${error instanceof Error ? error.message : error}` }, changes: null };
  }
}

// ───────────── Beancount: prices, cost, value and holdings ─────────────
// The queries Plainledger sends to bean-query (apps/server/lib/beancountQueries.ts, packages/ledger-domain/src/holdings/
// beancountHoldingsQueries.ts), word for word; spec/beancount/price-table.md, convert-rates.md, convert-positions.md,
// cost-of-position.md and holdings.md say what each gives.
const PRICE_QUERY = "SELECT date, currency, number(amount), currency(amount) FROM prices";
/** Not Plainledger's: the file and line of each price entry, in the order of the table, for the line we list. */
const PRICE_LINES_QUERY = "SELECT date, lineno, filename FROM entries WHERE type = 'price'";
const POSTING_COLUMNS = "id, account, posting_flag, number, currency, cost_number, cost_currency, cost_date, cost_label, number(price), currency(price), filename, lineno";
const COST_QUERY = `SELECT ${POSTING_COLUMNS}, number(cost(position)) AS amount, currency(cost(position)) AS commodity FROM postings`;

function convertedQuery(date: string, commodity: string): string {
  const [year, month, day] = date.split("-");
  const converted = `convert(position, "${commodity}", date(${year}, ${month}, ${day}))`;
  return `SELECT ${POSTING_COLUMNS}, number(${converted}) AS amount, currency(${converted}) AS commodity FROM postings`;
}

function unitsQuery(through: string | null): string {
  return `SELECT account, currency, str(sum(units(position))) AS units${through ? ` WHERE date <= ${through}` : ""} GROUP BY account, currency`;
}

function holdingsQuery(commodity: string, through: string | null): string {
  const where = through ? `WHERE date <= ${through}` : "";
  return [
    "SELECT account, currency, str(sum(units(position))) AS units, str(sum(cost(position))) AS cost_basis,",
    `str(sum(convert(cost(position), "${commodity}"))) AS cost_basis_value, str(sum(convert(position, "${commodity}"))) AS market_value`,
    where,
    "GROUP BY account, currency",
  ].join(" ");
}

/**
 * The rows bean-query prints for a query, header left out, padding trimmed. A failure of bean-query, or a Python
 * traceback in its output, is an error without its text: tracebacks are not read (EXPOSURE.md).
 */
async function beanRows(path: string, query: string): Promise<string[][]> {
  const proc = Bun.spawn(["bean-query", "-f", "csv", path, query], { stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  if (exitCode !== 0 || /Traceback \(most recent call last\)/.test(stdout + stderr))
    throw new Error("bean-query failed or printed a traceback (not shown; EXPOSURE.md says what to do)");
  const [, ...rows] = decodeCsv(stdout);
  return rows.filter((r) => r.length > 1).map((r) => r.map((c) => c.trim()));
}

type BeanRuns = Map<string, Promise<string[][]>>;

const beanRowsCached = (runs: BeanRuns, path: string, query: string): Promise<string[][]> => {
  let run = runs.get(query);
  if (!run) runs.set(query, (run = beanRows(path, query)));
  return run;
};

const failed = (error: unknown): Verdict => ({ status: "FAIL", note: String(error instanceof Error ? error.message : error) });

/** `Ledger::prices()` against the prices table, in order, then the file and line of each entry. */
export async function compareBeanPrices(path: string, ours: PriceDirective[], runs: BeanRuns = new Map()): Promise<Verdict> {
  try {
    const rows = await beanRowsCached(runs, path, PRICE_QUERY);
    const lines = await beanRowsCached(runs, path, PRICE_LINES_QUERY);
    if (rows.length !== ours.length) return { status: "FAIL", note: `length bean-query=${rows.length} ours=${ours.length}` };
    for (const [i, row] of rows.entries()) {
      const [mine, line] = [ours[i] as PriceDirective, lines[i] as string[]];
      const [date, commodity, number, quote] = row as [string, string, string, string];
      const theirs = `${date} ${commodity} ${number} ${quote}`;
      const got = `${mine.date} ${mine.commodity} ${mine.rate.quantity} ${mine.rate.commodity}`;
      if (theirs !== got) return { status: "FAIL", note: `[${i}] bean-query=${theirs} ours=${got}` };
      const [lineDate, lineno, filename] = line as [string, string, string];
      if (lineDate !== mine.date || Number(lineno) !== mine.line || basename(filename) !== basename(mine.file))
        return { status: "FAIL", note: `[${i}] entry bean-query=${lineDate} ${basename(filename)}:${lineno} ours=${mine.date} ${basename(mine.file)}:${mine.line}` };
    }
    return { status: "PASS", note: "" };
  } catch (error) {
    return failed(error);
  }
}

/**
 * One amount per posting of `after` against the `amount` and `commodity` columns (the last two) of a postings query
 * that starts with POSTING_COLUMNS, row by row. `before` are the postings as booked: the account, units and commodity
 * columns must agree with them, so that a row cannot be matched to the wrong posting.
 */
function diffPostingAmounts(rows: string[][], before: Transaction[], after: Transaction[]): string | null {
  const [was, now] = [before.flatMap((t) => t.postings), after.flatMap((t) => t.postings)];
  if (rows.length !== was.length || rows.length !== now.length) return `length bean-query=${rows.length} ours=${now.length} (booked ${was.length})`;
  for (const [i, row] of rows.entries()) {
    const [p, q] = [was[i] as Posting, now[i] as Posting];
    const [account, number, currency, amount, commodity, lineno] = [row[1], row[3], row[4], row[row.length - 2], row[row.length - 1], row[row.length - 3]] as string[];
    const a = q.amounts[0] as Amount;
    const unit = p.amounts[0] as Amount;
    if (account !== p.account || number !== unit.quantity || currency !== unit.commodity)
      return `[${i}] posting bean-query=${account} ${number} ${currency} ours=${p.account} ${unit.quantity} ${unit.commodity}`;
    if (amount !== a.quantity || commodity !== a.commodity)
      return `[${i}] ${account} ${number} ${currency} (line ${lineno}): bean-query=${amount} ${commodity} ours=${a.quantity} ${a.commodity}`;
  }
  return null;
}

/** An answer of ours that is a refusal is a SKIP when it says "not supported", else a FAIL; a crash is a FAIL. */
function refusal(ours: { refused: Finding } | { thrown: string }): Verdict {
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  const { code, details, message } = ours.refused;
  if (code === "unsupported-syntax") return { status: "SKIP", note: `unsupported: ${details.construct ?? "?"}` };
  return { status: "FAIL", note: `ours returns ${code} (${message}), bean-query accepts` };
}

/** `Ledger::at_cost()` against the cost query. */
export async function compareBeanCost(path: string, booked: Transaction[], runs: BeanRuns = new Map()): Promise<Verdict> {
  const ours = runOursQuery([path], (ledger) => ledger.atCost());
  if (!("answer" in ours)) return refusal(ours);
  try {
    const diff = diffPostingAmounts(await beanRowsCached(runs, path, COST_QUERY), booked, ours.answer);
    return diff ? { status: "FAIL", note: diff } : { status: "PASS", note: "" };
  } catch (error) {
    return failed(error);
  }
}

/** `Ledger::value(date, commodity)` against the value query on that date. */
export async function compareBeanValue(path: string, booked: Transaction[], { date, commodity }: Query, runs: BeanRuns = new Map()): Promise<Verdict> {
  const ours = runOursQuery([path], (ledger) => ledger.value(date, commodity));
  if (!("answer" in ours)) return refusal(ours);
  try {
    const diff = diffPostingAmounts(await beanRowsCached(runs, path, convertedQuery(date, commodity)), booked, ours.answer);
    return diff ? { status: "FAIL", note: diff } : { status: "PASS", note: "" };
  } catch (error) {
    return failed(error);
  }
}

/** The amounts of a cell of the holdings queries, `(35 USD,20 EUR,3 CHF)` or `()`, as `COMMODITY\0NUMBER` lines in code point order. */
export function decodeCell(cell: string): string[] {
  const inner = /^\((.*)\)$/s.exec(cell.trim())?.[1];
  if (inner === undefined) throw new Error(`not a cell: ${cell}`);
  return inner
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s !== "")
    .map((s) => {
      const [number, commodity, ...rest] = s.split(" ");
      if (number === undefined || commodity === undefined || rest.length > 0) throw new Error(`not an amount: ${s}`);
      return `${commodity}\0${number}`;
    })
    .sort();
}

/** The units of a cell: its number (`0` for an empty cell). A cell of units holds at most the commodity of its row. */
const cellUnits = (cell: string): string => decodeCell(cell)[0]?.split("\0")[1] ?? "0";

/** Our holdings as rows, exact: the strings as given, a row with nothing in it left out (the reference lists a `()` row). */
export function oursAsExactRows(holdings: Holding[]): Map<string, HoldingRow> {
  const lines = (list: Quantity[]) => list.map((q) => `${q.commodity}\0${q.quantity}`).sort();
  const rows = new Map<string, HoldingRow>();
  for (const h of holdings) {
    const row = { units: h.units, cost: lines(h.cost), value: lines(h.value), costValue: lines(h.cost_value) };
    if (!isEmptyRow(row)) rows.set(rowKey(h.account, h.commodity), row);
  }
  return rows;
}

/** A question for the Beancount holdings: `through` is Plainledger's `asOfDate` (inclusive), null for none. */
export type BeanHoldingQuestion = { through: string | null; commodity: string };

const nextDay = (date: string): string => isoDate(Date.parse(`${date}T00:00:00Z`) + DAY);
const previousDay = (date: string): string => isoDate(Date.parse(`${date}T00:00:00Z`) - DAY);

/**
 * `Ledger::holdings()` against the units query and the holdings query: `end` is the day after `through` (the
 * date filter is inclusive there, exclusive here). Every cell exactly, places included; the order of the amounts
 * inside a cell is not compared (spec/beancount/holdings.md). A row bean-query lists with nothing in it counts as
 * absent, as ours may leave it out.
 */
export async function compareBeanHoldings(path: string, q: BeanHoldingQuestion, runs: BeanRuns = new Map()): Promise<Verdict> {
  const query = { commodity: q.commodity, end: q.through ? nextDay(q.through) : null, today: localToday() };
  const ours = runOursHoldings([path], query);
  if (!("holdings" in ours)) return refusal(ours);
  const order = holdingsInOrder(ours.holdings);
  if (order) return { status: "FAIL", note: order };
  try {
    const theirs = new Map<string, HoldingRow>();
    const units = new Map<string, string>();
    for (const [account, currency, cell] of await beanRowsCached(runs, path, unitsQuery(q.through)))
      units.set(rowKey(account as string, currency as string), cellUnits(cell as string));
    for (const row of await beanRowsCached(runs, path, holdingsQuery(q.commodity, q.through))) {
      const [account, currency, unitsCell, cost, costValue, value] = row as [string, string, string, string, string, string];
      const key = rowKey(account, currency);
      if (units.get(key) !== cellUnits(unitsCell)) return { status: "FAIL", note: `${account} [${currency}] units: the two queries disagree` };
      const mine = { units: cellUnits(unitsCell), cost: decodeCell(cost), value: decodeCell(value), costValue: decodeCell(costValue) };
      if (!isEmptyRow(mine)) theirs.set(key, mine);
    }
    const diff = diffHoldingRows(theirs, oursAsExactRows(ours.holdings), "bean-query");
    return diff ? { status: "FAIL", note: diff } : { status: "PASS", note: "" };
  } catch (error) {
    return failed(error);
  }
}

/**
 * The `--value` questions for a Beancount journal, from the price table bean-query lists: one day before the first
 * price date, each distinct date (six at most, spread evenly), 30 days after the last; in EUR, USD and the
 * commodities of the first price. With no price, the middle transaction's date in EUR and USD.
 */
export function beanValueQueries(priceDates: string[], firstPrice: { from: string; to: string } | null, transactions: Transaction[]): Query[] {
  const dates = [...new Set(priceDates)].sort();
  const [first, last] = [dates[0], dates[dates.length - 1]];
  const targets = [...new Set(["EUR", "USD", firstPrice?.to ?? "", firstPrice?.from ?? ""])].filter((c) => c !== "");
  let when: string[];
  if (first === undefined || last === undefined) {
    const middle = transactions[Math.floor(transactions.length / 2)]?.date;
    when = middle === undefined ? [] : [middle];
  } else {
    const spread = dates.length <= 6 ? dates : Array.from({ length: 6 }, (_, i) => dates[Math.floor((i * (dates.length - 1)) / 5)] as string);
    const at = (date: string, days: number) => isoDate(Date.parse(`${date}T00:00:00Z`) + days * DAY);
    when = [...new Set([at(first, -1), ...spread, at(last, 30)])];
  }
  return when.filter((d) => d >= "1000-01-01" && d <= "9998-12-31").flatMap((date) => targets.map((commodity) => ({ date, commodity })));
}

/** The holdings questions: no date, and the first, middle and last of the value dates, in the same commodities as the value questions. */
export function beanHoldingQuestions(values: Query[]): BeanHoldingQuestion[] {
  const dates = [...new Set(values.map((v) => v.date))].sort();
  const through = [null, ...new Set([dates[0], dates[Math.floor(dates.length / 2)], dates[dates.length - 1]].filter((d): d is string => d !== undefined))];
  const commodities = [...new Set(values.map((v) => v.commodity))];
  return through.flatMap((t) => (commodities.length ? commodities : ["EUR"]).map((commodity) => ({ through: t, commodity })));
}

/** Everything compared for one Beancount journal that bean-check accepts and whose transactions we list as it does. */
export async function compareBeancountReads(
  path: string,
  ours: Loaded,
  raw: Verdict,
  asked: { queries?: Query[]; holdings?: boolean | HoldingQuestion[] } = {},
): Promise<Comparison> {
  const runs: BeanRuns = new Map();
  const prices = await compareBeanPrices(path, ours.prices, runs);
  const cost = await compareBeanCost(path, ours.transactions, runs);
  let priceRows: string[][] = [];
  try {
    priceRows = await beanRowsCached(runs, path, PRICE_QUERY);
  } catch {
    // the prices verdict already says so
  }
  const [firstRow] = priceRows;
  const questions =
    asked.queries ??
    beanValueQueries(
      priceRows.map((r) => r[0] as string),
      firstRow ? { from: firstRow[1] as string, to: firstRow[3] as string } : null,
      ours.transactions,
    );
  const values: Comparison["values"] = [];
  for (const query of questions) values.push({ query, verdict: await compareBeanValue(path, ours.transactions, query, runs) });
  const held: Comparison["beanHoldings"] = [];
  // Given questions have an exclusive `end` (hledger's `-e`): the date filter of the query is the day before.
  const given = Array.isArray(asked.holdings)
    ? asked.holdings.map((q): BeanHoldingQuestion => ({ through: q.end ? previousDay(q.end) : null, commodity: q.commodity }))
    : null;
  for (const question of given ?? beanHoldingQuestions(questions)) held.push({ question, verdict: await compareBeanHoldings(path, question, runs) });
  return { raw, cost, values, postings: null, prices, holdings: [], beanHoldings: held, plainledger: null, format: "beancount" };
}

/** Every comparison of one case: the raw one, `--cost`, and `--value` for each query. The last two only when the first passes. */
export type Comparison = {
  raw: Verdict;
  cost: Verdict | null;
  values: { query: Query; verdict: Verdict }[];
  postings: Verdict | null;
  prices: Verdict | null;
  holdings: { question: HoldingQuestion; verdict: Verdict }[];
  /** Plainledger's own reads (see `comparePlainledger`), for the cases that ask for them. */
  plainledger: PlainledgerVerdict | null;
  /** Beancount only: the holdings questions, against bean-query's units and holdings queries. */
  beanHoldings?: { question: BeanHoldingQuestion; verdict: Verdict }[];
  /** What `format()` did to the files (`compareFormat`). */
  formatCheck?: FormatCheck | null;
  /** Beancount cases fill `prices`, `cost`, `values` and `beanHoldings`; absent for hledger. */
  format?: "beancount";
};

/**
 * Compare a case raw, then under `--cost`, then under `--value=DATE,COMM` for `queries` (by default the ones
 * `valueQueries` makes from the journals' `P` lines). `holdings` asks the questions `holdingQuestions` makes, or the ones given. The `--cost`, `--value` and holdings comparisons are not run for a Beancount file, for a journal
 * hledger rejects, or when the raw comparison does not pass.
 */
export async function compareAll(
  paths: string[],
  ignorePlaces = false,
  queries?: Query[],
  { plainledger = false, holdings = false }: { plainledger?: boolean; holdings?: boolean | HoldingQuestion[] } = {},
): Promise<Comparison> {
  // Asked of every case, whatever the other comparisons say: formatting must hold for a journal the reference rejects too.
  // (Several files with a Beancount one among them are refused: ledgermill loads one Beancount file at a time.)
  const formatCheck = paths.length > 1 && paths.some(isBeancountPath) ? null : await compareFormat(paths);
  const { verdict: raw, agreed, ours } = await compareRaw(paths, ignorePlaces);
  if (!agreed || !ours) return { raw, cost: null, values: [], postings: null, prices: null, holdings: [], plainledger: null, formatCheck };
  const [only] = paths;
  if (only !== undefined && paths.length === 1 && isBeancountPath(only))
    return { ...(await compareBeancountReads(only, ours, raw, { queries, holdings })), formatCheck };
  const postings = await comparePostings(paths, ours.postings);
  const prices = await comparePrices(paths, ours.prices);
  const cost = await compareCost(paths, ignorePlaces);
  const questions = queries ?? valueQueries(paths.map((p) => readFileSync(p, "utf8")));
  const values: Comparison["values"] = [];
  for (const query of questions) values.push({ query, verdict: await compareValue(paths, query, ignorePlaces) });
  const cache: RunCache = new Map();
  const held: Comparison["holdings"] = [];
  if (holdings)
    for (const question of holdings === true ? holdingQuestions(paths.map((p) => readFileSync(p, "utf8"))) : holdings)
      held.push({ question, verdict: await compareHoldings(paths, question, cache) });
  return { raw, cost, values, postings, prices, holdings: held, plainledger: plainledger ? await comparePlainledger(paths, cache) : null, formatCheck };
}

/** The worst verdict, with its note told apart by what was compared. */
export function summarize(c: Comparison): Verdict {
  const parts: { status: Verdict["status"]; note: string }[] = [
    c.raw,
    ...(c.postings ? [{ ...c.postings, note: c.postings.note && `postings: ${c.postings.note}` }] : []),
    ...(c.prices ? [{ ...c.prices, note: c.prices.note && `prices: ${c.prices.note}` }] : []),
    ...(c.formatCheck ? [{ ...c.formatCheck.verdict, note: c.formatCheck.verdict.note && `format: ${c.formatCheck.verdict.note}` }] : []),
    ...(c.cost ? [{ ...c.cost, note: c.cost.note && `cost: ${c.cost.note}` }] : []),
    ...c.values.map(({ query, verdict }) => ({
      ...verdict,
      note: verdict.note && `value=${query.date},${query.commodity}: ${verdict.note}`,
    })),
    ...c.holdings.map(({ question, verdict }) => ({
      ...verdict,
      note: verdict.note && `holdings end=${question.end ?? "-"} in ${question.commodity}: ${verdict.note}`,
    })),
    ...(c.beanHoldings ?? []).map(({ question, verdict }) => ({
      ...verdict,
      note: verdict.note && `holdings through=${question.through ?? "-"} in ${question.commodity}: ${verdict.note}`,
    })),
    ...(c.plainledger ? [c.plainledger.verdict] : []),
  ];
  for (const status of ["FAIL", "SKIP"] as const) {
    const hit = parts.find((p) => p.status === status);
    if (hit) return { status, note: hit.note };
  }
  return { status: "PASS", note: c.raw.note };
}

const JOBS = 8;

/** Verdicts by status, and the SKIPs by why (`skipReason`). */
export type Counts = { PASS: number; FAIL: number; SKIP: number; why: Record<string, number> };
export const counts = (): Counts => ({ PASS: 0, FAIL: 0, SKIP: 0, why: {} });

/**
 * What a SKIP's note says was refused: the construct of an `unsupported-syntax` refusal, `document`, or `cur`. The note
 * may carry `summarize`'s prefix (`value=DATE,COMM: `).
 */
export function skipReason(note: string): string {
  const m = /(?:^|: )(?:unsupported: |unsupported-syntax )(\S+)/.exec(note);
  if (m) return m[1] as string;
  if (/(?:^|: )ignored: document/.test(note)) return "document";
  if (/(?:^|: )cur: /.test(note)) return "cur";
  return note === "" ? "?" : note;
}

/**
 * The refusals the corpus and the fuzzers are known to meet (spec/README.md lists what ledgermill refuses). A SKIP for
 * anything else is printed as `UNEXPECTED SKIP` and makes the exit code 1: a new refusal must not hide in a count.
 */
export const EXPECTED_SKIPS: ReadonlySet<string> = new Set([
  // Beancount value()/holdings() with a zero price in a pair priced both ways (spec/beancount/convert-rates.md).
  "price-both-ways",
  // hledger --cost with conversion pairs next to several priced postings or an assignment (Ledger::at_cost).
  "cost-conversion-postings",
  // The harness, not ledgermill: hledger's `cur:` query cannot tell this commodity from another (compareHoldings).
  "cur",
  // Beancount `document`: an ignored-syntax warning, bean-check looks at the file (spec/README.md).
  "document",
]);

export function count(t: Counts, v: Verdict): void {
  t[v.status]++;
  if (v.status !== "SKIP") return;
  const why = skipReason(v.note);
  t.why[why] = (t.why[why] ?? 0) + 1;
}

/** `N pass, N fail, N skip`, then the SKIPs by why: `(price-both-ways 38)`. */
export const countsLine = (t: Counts) => {
  const why = Object.entries(t.why).sort(([a], [b]) => a.localeCompare(b));
  return `${t.PASS} pass, ${t.FAIL} fail, ${t.SKIP} skip${why.length ? ` (${why.map(([k, n]) => `${k} ${n}`).join(", ")})` : ""}`;
};

/** The SKIP reasons of `ts` not in `EXPECTED_SKIPS`, with their counts. */
export function unexpectedSkips(...ts: Counts[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const t of ts) for (const [k, n] of Object.entries(t.why)) if (!EXPECTED_SKIPS.has(k)) out[k] = (out[k] ?? 0) + n;
  return out;
}

const total = (...ts: Counts[]): Counts => {
  const all = counts();
  for (const t of ts) {
    all.PASS += t.PASS;
    all.FAIL += t.FAIL;
    all.SKIP += t.SKIP;
    for (const [k, n] of Object.entries(t.why)) all.why[k] = (all.why[k] ?? 0) + n;
  }
  return all;
};

/** `N pass, N fail (N changed, N returned unchanged)`: unchanged files include the ones the formatter's safety check gave back. */
export const formatLine = (t: Tallies): string =>
  `${t.format.PASS} pass, ${t.format.FAIL} fail (${t.formatted.changed} changed, ${t.formatted.unchanged} returned unchanged)`;

/** The same for Beancount files, with how our text compares with `bean-format`'s. */
export const beanFormatLine = (t: Tallies): string => {
  const { format, formatted, reference: r } = t.beanFormat;
  return `${format.PASS} pass, ${format.FAIL} fail (${formatted.changed} changed, ${formatted.unchanged} returned unchanged; bean-format: ${r.equal} equal, ${r.design} differ by design, ${r.none} no reference)`;
};

/** The tallies of what a hledger case compared besides the raw comparison. */
export type Tallies = {
  cost: Counts;
  value: Counts;
  postings: Counts;
  prices: Counts;
  holdings: Counts;
  plainledger: Counts;
  /** `format()` on hledger files: the verdicts, and how many cases it changed or returned as they were. */
  format: Counts;
  formatted: { changed: number; unchanged: number };
  /** `format()` on Beancount files: the same, and the files by how they compare with `bean-format` (`BeanFormatReference`). */
  beanFormat: { format: Counts; formatted: { changed: number; unchanged: number }; reference: Record<BeanFormatReference, number> };
  /** What a Beancount case compared: `Ledger::at_cost()`, `value()`, `prices()` and `holdings()` against bean-query. */
  beancount: { cost: Counts; value: Counts; prices: Counts; holdings: Counts };
};
export const tallies = (): Tallies => ({
  cost: counts(),
  value: counts(),
  postings: counts(),
  prices: counts(),
  holdings: counts(),
  plainledger: counts(),
  format: counts(),
  formatted: { changed: 0, unchanged: 0 },
  beanFormat: { format: counts(), formatted: { changed: 0, unchanged: 0 }, reference: { equal: 0, design: 0, none: 0 } },
  beancount: { cost: counts(), value: counts(), prices: counts(), holdings: counts() },
});

function tallyFormat({ verdict, changed, reference }: FormatCheck, t: Tallies): void {
  const own = reference === undefined ? t : t.beanFormat;
  count(own.format, verdict);
  own.formatted[changed ? "changed" : "unchanged"]++;
  if (reference !== undefined) t.beanFormat.reference[reference]++;
}

/** Add what a hledger case compared under `--cost`, `--value`, `postings()`, `prices()` and `holdings()` to the tallies. */
export function tally(c: Comparison, t: Tallies): void {
  if (c.formatCheck) tallyFormat(c.formatCheck, t);
  if (c.format === "beancount") {
    const b = t.beancount;
    if (c.cost) count(b.cost, c.cost);
    for (const { verdict } of c.values) count(b.value, verdict);
    if (c.prices) count(b.prices, c.prices);
    for (const { verdict } of c.beanHoldings ?? []) count(b.holdings, verdict);
    return;
  }
  if (c.cost) count(t.cost, c.cost);
  for (const { verdict } of c.values) count(t.value, verdict);
  if (c.postings) count(t.postings, c.postings);
  if (c.prices) count(t.prices, c.prices);
  for (const { verdict } of c.holdings) count(t.holdings, verdict);
  if (c.plainledger) count(t.plainledger, c.plainledger.verdict);
}

/**
 * Run `cases` (hledger in parallel) and print one line each in order (only non-PASS with `quiet`), then the totals;
 * exit code 1 on any FAIL. A hledger case is compared raw, under `--cost`, and under `--value` at several
 * dates and commodities when it has `P` lines (see `compareAll`); its line shows the worst of them.
 */
export async function runCases(
  cases: { name: string; paths: string[]; ignorePlaces?: boolean; plainledger?: boolean; formatOnly?: boolean }[],
  quiet: boolean,
): Promise<void> {
  const comparisons: Comparison[] = [];
  const formatOnly: (FormatCheck | undefined)[] = [];
  let next = 0;
  const worker = async () => {
    while (next < cases.length) {
      const i = next++;
      const c = cases[i] as { paths: string[]; ignorePlaces?: boolean; plainledger?: boolean; formatOnly?: boolean };
      if (c.formatOnly) formatOnly[i] = await compareFormat(c.paths);
      else comparisons[i] = await compareAll(c.paths, c.ignorePlaces, undefined, { plainledger: c.plainledger, holdings: true });
    }
  };
  await Promise.all(Array.from({ length: JOBS }, worker));

  const totals = { hledger: counts(), beancount: counts() };
  const t = tallies();
  const changes: string[] = [];
  for (const [i, c] of cases.entries()) {
    const only = formatOnly[i];
    if (only) {
      // Not counted in the raw totals: the file is only asked to format (the files ledgermill refuses).
      tallyFormat(only, t);
      if (!quiet || only.verdict.status !== "PASS") console.log(`${only.verdict.status} ${c.name}  format${only.verdict.note ? `: ${only.verdict.note}` : ""}`);
      continue;
    }
    const comparison = comparisons[i] as Comparison;
    count(totals[isBeancountPath(c.paths[0] ?? "") ? "beancount" : "hledger"], comparison.raw);
    tally(comparison, t);
    if (comparison.plainledger?.changes) changes.push(`${c.name}: ${comparison.plainledger.changes}`);
    const v = summarize(comparison);
    if (!quiet || v.status !== "PASS") console.log(`${v.status} ${c.name}${v.note ? `  ${v.note}` : ""}`);
  }
  const { hledger: h, beancount: b } = totals;
  if (h.PASS + h.FAIL + h.SKIP > 0 && b.PASS + b.FAIL + b.SKIP > 0) console.log(`\nhledger:   ${countsLine(h)}\nbeancount: ${countsLine(b)}`);
  console.log(`\nhledger postings: ${countsLine(t.postings)}\nhledger prices:   ${countsLine(t.prices)}`);
  console.log(`hledger format:   ${formatLine(t)}`);
  if (t.beanFormat.format.PASS + t.beanFormat.format.FAIL > 0) console.log(`beancount format: ${beanFormatLine(t)}`);
  console.log(`hledger --cost:  ${countsLine(t.cost)}\nhledger --value: ${countsLine(t.value)} (one per date and commodity asked)`);
  console.log(`hledger holdings: ${countsLine(t.holdings)} (one per end date and commodity asked)`);
  console.log(`Plainledger's reads: ${countsLine(t.plainledger)} (journals of the app corpus)`);
  const b2 = t.beancount;
  if (total(b2.cost, b2.value, b2.prices, b2.holdings).PASS > 0 || total(b2.cost, b2.value, b2.prices, b2.holdings).FAIL > 0) {
    console.log(`\nbeancount prices:   ${countsLine(b2.prices)}\nbeancount cost:     ${countsLine(b2.cost)}`);
    console.log(`beancount value:    ${countsLine(b2.value)} (one per date and commodity asked)`);
    console.log(`beancount holdings: ${countsLine(b2.holdings)} (units and holdings queries, one per date and commodity asked)`);
  }
  if (changes.length > 0)
    console.log(`\nPlainledger's figures that change with real postings only (expected; a virtual posting on a tracked account):\n${changes.map((c) => `  ${c}`).join("\n")}`);
  const all = total(h, b, t.cost, t.value, t.postings, t.prices, t.holdings, t.plainledger, t.format, t.beanFormat.format, b2.cost, b2.value, b2.prices, b2.holdings);
  console.log(`\n${countsLine(all)}`);
  const unexpected = unexpectedSkips(all);
  for (const [k, n] of Object.entries(unexpected)) console.log(`UNEXPECTED SKIP ${k} (${n}): a refusal not in EXPECTED_SKIPS (lib.ts)`);
  if (all.FAIL > 0 || Object.keys(unexpected).length > 0) process.exitCode = 1;
}
