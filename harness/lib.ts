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
type Answer = { transactions: Transaction[] } | { refused: Finding } | { thrown: string };

/**
 * Load `paths` and call one of the ledger's methods that returns transactions as JSON. The wasm throws an
 * `Error` whose message is the finding as JSON when the core refuses (see crates/ledgermill-wasm/src/lib.rs).
 */
function runOursQuery(paths: string[], call: (ledger: ReturnType<typeof wasm.load>) => string): Answer {
  const files = paths.map((name) => ({ name, content: readFileSync(name, "utf8") }));
  let ledger: ReturnType<typeof wasm.load> | undefined;
  try {
    ledger = wasm.load(JSON.stringify(files));
    return { transactions: JSON.parse(call(ledger)) };
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

async function compareBeancountCase(path: string): Promise<Verdict> {
  const rejection = await runBeanCheck(path);
  const ours = runOurs([path]);
  if ("thrown" in ours) return { status: "FAIL", note: `wasm threw: ${ours.thrown}` };
  const errors = ours.findings.filter((f) => f.severity === "error");
  const unsupported = ours.findings.find((f) => f.code === "unsupported-syntax");
  if (unsupported) return { status: "SKIP", note: `unsupported: ${unsupported.details.construct ?? "?"}` };
  if (rejection) {
    const verdict = compareFirstError(rejection, errors, ours.findings);
    if (verdict.status !== "PASS" || !listsComparable(errors)) return verdict;
  } else if (errors[0]) {
    return { status: "FAIL", note: `ours reports ${errors[0].code} at line ${errors[0].line}, bean-check accepts` };
  }
  const ref = await queryBeancount(path);
  const diff = firstDiff(withBasenames(ref.transactions), withBasenames(normalizeOurs(ours.transactions)));
  if (diff) return { status: "FAIL", note: diff };
  const postingsDiff = firstDiff(postingBasenames(ref.postings), postingBasenames(ours.postings));
  if (postingsDiff) return { status: "FAIL", note: `postings: ${postingsDiff}` };
  return { status: "PASS", note: rejection ? "both fail, same first error, same lists" : "" };
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
  if (first !== undefined && paths.length === 1 && isBeancountPath(first)) {
    return { verdict: await compareBeancountCase(first), agreed: false };
  }
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
  const [theirs, mine] = [withBasenames(ref.transactions), withBasenames(ours.transactions)];
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

/** Every comparison of one hledger case: the raw one, `--cost`, and `--value` for each query. The last two only when the first passes. */
export type Comparison = {
  raw: Verdict;
  cost: Verdict | null;
  values: { query: Query; verdict: Verdict }[];
  postings: Verdict | null;
  prices: Verdict | null;
};

/**
 * Compare a case raw, then under `--cost`, then under `--value=DATE,COMM` for `queries` (by default the ones
 * `valueQueries` makes from the journals' `P` lines). The second and third are not run for a Beancount file, for a journal
 * hledger rejects, or when the raw comparison does not pass.
 */
export async function compareAll(paths: string[], ignorePlaces = false, queries?: Query[]): Promise<Comparison> {
  const { verdict: raw, agreed, ours } = await compareRaw(paths, ignorePlaces);
  if (!agreed || !ours) return { raw, cost: null, values: [], postings: null, prices: null };
  const postings = await comparePostings(paths, ours.postings);
  const prices = await comparePrices(paths, ours.prices);
  const cost = await compareCost(paths, ignorePlaces);
  const questions = queries ?? valueQueries(paths.map((p) => readFileSync(p, "utf8")));
  const values: Comparison["values"] = [];
  for (const query of questions) values.push({ query, verdict: await compareValue(paths, query, ignorePlaces) });
  return { raw, cost, values, postings, prices };
}

/** The worst verdict, with its note told apart by what was compared. */
export function summarize(c: Comparison): Verdict {
  const parts: { status: Verdict["status"]; note: string }[] = [
    c.raw,
    ...(c.postings ? [{ ...c.postings, note: c.postings.note && `postings: ${c.postings.note}` }] : []),
    ...(c.prices ? [{ ...c.prices, note: c.prices.note && `prices: ${c.prices.note}` }] : []),
    ...(c.cost ? [{ ...c.cost, note: c.cost.note && `cost: ${c.cost.note}` }] : []),
    ...c.values.map(({ query, verdict }) => ({
      ...verdict,
      note: verdict.note && `value=${query.date},${query.commodity}: ${verdict.note}`,
    })),
  ];
  for (const status of ["FAIL", "SKIP"] as const) {
    const hit = parts.find((p) => p.status === status);
    if (hit) return { status, note: hit.note };
  }
  return { status: "PASS", note: c.raw.note };
}

const JOBS = 8;

type Counts = { PASS: number; FAIL: number; SKIP: number };
const counts = (): Counts => ({ PASS: 0, FAIL: 0, SKIP: 0 });
const countsLine = (t: Counts) => `${t.PASS} pass, ${t.FAIL} fail, ${t.SKIP} skip`;
const total = (...ts: Counts[]): Counts => ({
  PASS: ts.reduce((n, t) => n + t.PASS, 0),
  FAIL: ts.reduce((n, t) => n + t.FAIL, 0),
  SKIP: ts.reduce((n, t) => n + t.SKIP, 0),
});

/** The tallies of what a hledger case compared besides the raw comparison. */
export type Tallies = { cost: Counts; value: Counts; postings: Counts; prices: Counts };
export const tallies = (): Tallies => ({ cost: counts(), value: counts(), postings: counts(), prices: counts() });

/** Add what a hledger case compared under `--cost`, `--value`, `postings()` and `prices()` to the tallies. */
export function tally(c: Comparison, t: Tallies): void {
  if (c.cost) t.cost[c.cost.status]++;
  for (const { verdict } of c.values) t.value[verdict.status]++;
  if (c.postings) t.postings[c.postings.status]++;
  if (c.prices) t.prices[c.prices.status]++;
}

/**
 * Run `cases` (hledger in parallel) and print one line each in order (only non-PASS with `quiet`), then the totals;
 * exit code 1 on any FAIL. A hledger case is compared raw, under `--cost`, and under `--value` at several
 * dates and commodities when it has `P` lines (see `compareAll`); its line shows the worst of them.
 */
export async function runCases(
  cases: { name: string; paths: string[]; ignorePlaces?: boolean }[],
  quiet: boolean,
): Promise<void> {
  const comparisons: Comparison[] = [];
  let next = 0;
  const worker = async () => {
    while (next < cases.length) {
      const i = next++;
      const c = cases[i] as { paths: string[]; ignorePlaces?: boolean };
      comparisons[i] = await compareAll(c.paths, c.ignorePlaces);
    }
  };
  await Promise.all(Array.from({ length: JOBS }, worker));

  const totals = { hledger: counts(), beancount: counts() };
  const t = tallies();
  for (const [i, comparison] of comparisons.entries()) {
    const c = cases[i] as { name: string; paths: string[] };
    totals[isBeancountPath(c.paths[0] ?? "") ? "beancount" : "hledger"][comparison.raw.status]++;
    tally(comparison, t);
    const v = summarize(comparison);
    if (!quiet || v.status !== "PASS") console.log(`${v.status} ${c.name}${v.note ? `  ${v.note}` : ""}`);
  }
  const { hledger: h, beancount: b } = totals;
  if (h.PASS + h.FAIL + h.SKIP > 0 && b.PASS + b.FAIL + b.SKIP > 0) console.log(`\nhledger:   ${countsLine(h)}\nbeancount: ${countsLine(b)}`);
  console.log(`\nhledger postings: ${countsLine(t.postings)}\nhledger prices:   ${countsLine(t.prices)}`);
  console.log(`hledger --cost:  ${countsLine(t.cost)}\nhledger --value: ${countsLine(t.value)} (one per date and commodity asked)`);
  const all = total(h, b, t.cost, t.value, t.postings, t.prices);
  console.log(`\n${all.PASS} pass, ${all.FAIL} fail, ${all.SKIP} skip`);
  if (all.FAIL > 0) process.exitCode = 1;
}
