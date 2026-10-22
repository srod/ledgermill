// bun harness/fuzz.ts [COUNT] [SEED] [--keep]
// Random journals, mostly valid, compared like the corpus: hledger against our wasm. Covers
// amount forms, decimal marks and digit groups, directives, prices, lots, elided amounts,
// inferred prices, conversion postings, virtual postings, balance assertions and assignments (small
// numbers, so that assertions often hold) and big numbers. Every case must
// PASS or SKIP; any FAIL is printed with its journal and makes the exit code 1.
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { compareCase, type Verdict } from "./lib.ts";

const args = process.argv.slice(2).filter((a) => !a.startsWith("--"));
const count = Number(args[0] ?? 500);
const seed = Number(args[1] ?? 1);
const keep = process.argv.includes("--keep");

// xorshift32: the same seed gives the same journals on every machine.
function rng(start: number): () => number {
  let s = start >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s >>>= 0;
    s ^= s >>> 17;
    s ^= s << 5;
    s >>>= 0;
    return s / 4294967296;
  };
}
const r = rng(seed);
const int = (n: number) => Math.floor(r() * n);
const pick = <T>(xs: readonly T[]): T => xs[int(xs.length)] as T;
const chance = (p: number) => r() < p;

// ───────────── numbers ─────────────
type N = { v: bigint; dp: number };
const places = [0, 0, 1, 2, 2, 3, 5] as const;
function randomN(dp: number = pick(places)): N {
  // Zeros with places matter: summing onto a zero keeps the other term's places.
  if (chance(0.08)) return { v: 0n, dp };
  const big = chance(0.03);
  const magnitude = big ? 10n ** BigInt(16 + int(15)) : 1n;
  const v = (BigInt(1 + int(chance(0.2) ? 100000 : 3000)) * magnitude) * (chance(0.5) ? 1n : -1n);
  return { v, dp };
}
const neg = (x: N): N => ({ v: -x.v, dp: x.dp });
const mul = (a: N, b: N): N => ({ v: a.v * b.v, dp: a.dp + b.dp });
const nudge = (x: N): N => (chance(0.2) ? { v: x.v + (chance(0.5) ? 1n : -1n), dp: x.dp } : x);

type Style = { mark: "." | ","; group: "" | "," | "." | " " };
function digits(x: N, style: Style): string {
  const negative = x.v < 0n;
  const abs = (negative ? -x.v : x.v).toString().padStart(x.dp + 1, "0");
  let int_ = abs.slice(0, abs.length - x.dp);
  const frac = abs.slice(abs.length - x.dp);
  if (style.group && int_.length > 3) {
    const parts: string[] = [];
    while (int_.length > 3) {
      parts.unshift(int_.slice(-3));
      int_ = int_.slice(0, -3);
    }
    parts.unshift(int_);
    int_ = parts.join(style.group);
  }
  return `${negative ? "-" : ""}${int_}${x.dp ? `${style.mark}${frac}` : ""}`;
}

// ───────────── amounts ─────────────
const COMMODITIES = ["EUR", "USD", "X", "Y", "$", "AB", '"A B"'] as const;
function amount(c: string, x: N, style: Style): string {
  const n = digits(x, style);
  if (c === "$") return x.v < 0n ? `-$${digits(neg(x), style)}` : `$${n}`;
  if (c === "AB") return chance(0.5) ? `${n} AB` : `AB ${n}`;
  if (c === '"A B"') return `${n} "A B"`;
  return chance(0.15) ? `${n}${c}` : `${n} ${c}`;
}

// A file-wide style: a period mark with comma groups, or the reverse, or spaces. The file
// declares it with `decimal-mark` when ambiguous amounts like `1,000` could appear.
function fileStyle(): { style: Style; header: string } {
  switch (int(4)) {
    case 0:
      return { style: { mark: ".", group: "" }, header: "" };
    case 1:
      return { style: { mark: ".", group: "," }, header: "decimal-mark .\n" };
    case 2:
      return { style: { mark: ",", group: "." }, header: "decimal-mark ,\n" };
    default:
      return { style: { mark: ",", group: " " }, header: chance(0.5) ? "commodity 1 000,00 EUR\n" : "decimal-mark ,\n" };
  }
}

// ───────────── transactions ─────────────
const ACCOUNTS = ["a", "b", "c:d", "e", "f:g:h", "Assets:Bank Account"] as const;
const CONVERSION = ["equity:conversion", "equity:trading:x", "equity:trade"] as const;
const comment = () => (chance(0.15) ? `  ; c${int(9)}` : chance(0.04) ? "  ; date:2026-01-02" : "");
const line = (account: string, body: string) => `    ${account}${body ? `  ${body}` : ""}${comment()}\n`;

function transaction(style: Style): string {
  const status = pick(["", "* ", "! "]);
  const code = chance(0.1) ? `(${int(99)}) ` : "";
  let t = `2026-01-0${1 + int(3)} ${status}${code}t${int(9)}\n`;
  const c1 = pick(COMMODITIES);
  let c2 = pick(COMMODITIES);
  while (c2 === c1) c2 = pick(COMMODITIES);
  const acct = () => pick(ACCOUNTS);
  switch (int(11)) {
    case 0: {
      // one commodity, last amount elided or written (sometimes off by one: must fail in both)
      const dp = pick(places);
      let sum = 0n;
      for (let k = 0, n = 1 + int(3); k < n; k++) {
        const x = randomN(dp);
        sum += x.v;
        t += line(acct(), amount(c1, x, style));
      }
      t += line(acct(), chance(0.5) ? "" : amount(c1, nudge({ v: -sum, dp }), style));
      break;
    }
    case 1: {
      const q = randomN();
      const p = { ...randomN(), v: BigInt(1 + int(500)) };
      t += line("a", `${amount(c1, q, style)} @ ${amount(c2, p, style)}`);
      t += line("b", chance(0.3) ? "" : amount(c2, nudge(neg(mul(q, p))), style));
      break;
    }
    case 2: {
      const q = randomN();
      const total = { ...randomN(), v: BigInt(1 + int(5000)) * (chance(0.3) ? -1n : 1n) };
      t += line("a", `${amount(c1, q, style)} @@ ${amount(c2, total, style)}`);
      t += line("b", amount(c2, neg(q.v < 0n ? neg(total) : total), style));
      break;
    }
    case 3: {
      // price inference
      t += line("a", amount(c1, randomN(), style));
      if (chance(0.4)) t += line("e", amount(c1, randomN(), style));
      t += line("b", amount(c2, randomN(), style));
      if (chance(0.3)) t += line("c", amount(c2, randomN(), style));
      break;
    }
    case 4: {
      // a conversion pair after a priced posting
      const q = randomN();
      const p = { ...randomN(), v: BigInt(1 + int(50)) };
      const cost = mul(q, p);
      const total = chance(0.4);
      t += line("a", total ? `${amount(c1, q, style)} @@ ${amount(c2, cost.v < 0n ? neg(cost) : cost, style)}` : `${amount(c1, q, style)} @ ${amount(c2, p, style)}`);
      const pair = [line(pick(CONVERSION), amount(c1, neg(q), style)), line(pick(CONVERSION), amount(c2, cost, style))];
      if (chance(0.3)) pair.reverse();
      t += pair.join("");
      t += line("b", amount(c2, nudge(neg(cost)), style));
      break;
    }
    case 5: {
      // lots, with or without a price
      const q = randomN();
      const p = { ...randomN(), v: BigInt(1 + int(500)) };
      const lot = chance(0.5)
        ? `{${amount(c2, p, style)}}`
        : chance(0.5)
          ? `{{${amount(c2, mul(q.v < 0n ? neg(q) : q, p), style)}}}`
          : `{=${amount(c2, p, style)}} [2026-01-02] (l${int(3)})`;
      t += line("a", `${amount(c1, q, style)} ${lot}${chance(0.3) ? ` @ ${amount(c2, p, style)}` : ""}`);
      t += line("b", chance(0.5) ? "" : amount(c1, neg(q), style));
      break;
    }
    case 6: {
      // several amounts, some priced alike, into one elided posting: what the elided amount's
      // places come out as depends on how equal-looking terms are grouped and summed
      const unit = { v: BigInt(1 + int(3)) * 5n, dp: 1 };
      const total = { v: BigInt(1 + int(3)) * 50n, dp: 2 };
      for (let k = 0, n = 2 + int(4); k < n; k++) {
        const x = chance(0.3) ? { v: 0n, dp: pick(places) } : randomN();
        const form = int(4);
        if (form === 0) t += line(acct(), amount(c1, x, style));
        else if (form === 1) t += line(acct(), amount(c2, x, style));
        else if (form === 2) t += line(acct(), `${amount(c1, x, style)} @ ${amount(c2, unit, style)}`);
        else t += line(acct(), `${amount(c1, x, style)} @@ ${amount(c2, total, style)}`);
      }
      t += line("c", "");
      break;
    }
    case 7: {
      // virtual postings
      t += line("a", amount(c1, randomN(), style));
      t += line("(v)", chance(0.5) ? amount(c2, randomN(), style) : "");
      t += line("[bv]", amount(c1, randomN(0), style));
      t += line("[bw]", chance(0.5) ? "" : amount(c1, randomN(0), style));
      t += line("b", "");
      break;
    }
    case 8: {
      // exponents and odd number forms
      const x = randomN(0);
      const form = pick([`${x.v}e0`, `${x.v}0e-1`, `+${x.v < 0n ? -x.v : x.v}`, `${x.v}.`]);
      t += line("a", `${form} ${c1}`);
      t += line("b", "");
      break;
    }
    case 9: {
      // conversion postings anywhere around one or two priced postings, with numbers equal or
      // close to their quantities and costs, then balanced by cost or by quantity at random
      const sums = [new Map<string, N>(), new Map<string, N>()];
      const add = (c: string, x: N, only?: number) => {
        sums.forEach((m, i) => {
          if (only !== undefined && only !== i) return;
          const s = m.get(c) ?? { v: 0n, dp: 0 };
          const dp = Math.max(s.dp, x.dp);
          m.set(c, { v: s.v * 10n ** BigInt(dp - s.dp) + x.v * 10n ** BigInt(dp - x.dp), dp });
        });
      };
      const near = (x: N): N => (chance(0.6) ? x : { v: x.v * 10n + BigInt(int(11) - 5), dp: x.dp + 1 });
      const lines: string[] = [];
      const priced: { q: N; cost: N }[] = [];
      for (let k = 0, n = 1 + int(2); k < n; k++) {
        const q = priced[0] && chance(0.3) ? priced[0].q : randomN(pick([0, 1, 2]));
        const unit = { v: BigInt(1 + int(9)), dp: pick([0, 0, 1]) };
        const cost = mul(q, unit);
        priced.push({ q, cost });
        lines.push(line(`a${k}`, `${amount(c1, q, style)} @ ${amount(c2, unit, style)}`));
        add(c2, cost, 0);
        add(c1, q, 1);
      }
      for (let k = 0, n = 1 + int(4); k < n; k++) {
        const p = pick(priced);
        const form = int(3);
        const c = form === 0 ? c1 : form === 1 ? c2 : pick([c1, c2]);
        const x = form === 0 ? near(neg(p.q)) : form === 1 ? near(p.cost) : randomN(0);
        lines.push(line(pick(CONVERSION), amount(c, x, style)));
        add(c, x);
      }
      for (let k = 0, n = int(3); k < n; k++) {
        const c = pick(COMMODITIES);
        const x = chance(0.5) ? near(pick(priced).q) : randomN(0);
        lines.push(line(acct(), amount(c, x, style)));
        add(c, x);
      }
      for (let k = lines.length - 1; k > 0; k--) {
        const j = int(k + 1);
        [lines[k], lines[j]] = [lines[j] as string, lines[k] as string];
      }
      for (const [c, s] of sums[int(2)] as Map<string, N>) if (s.v !== 0n) lines.push(line("b", amount(c, neg(s), style)));
      t += lines.join("");
      break;
    }
    default: {
      // transactions made only of conversion accounts
      const dp = pick(places);
      let sum = 0n;
      for (let k = 0, n = 2 + int(3); k < n; k++) {
        const x = randomN(dp);
        sum += x.v;
        t += line(pick(CONVERSION), amount(c1, x, style));
      }
      t += line(pick(CONVERSION), chance(0.5) ? "" : amount(c1, { v: -sum, dp }, style));
    }
  }
  return t;
}

// ───────────── balance assertions and assignments ─────────────
const BALANCE_ACCOUNTS = ["a", "a:b", "a:b:c", "a:d", "ab", "c"] as const;
const BALANCE_COMMODITIES = ["EUR", "USD", "X", ""] as const;
const PLAIN: Style = { mark: ".", group: "" };
/** A small number, now and then with places, so that sums and assertions meet. */
function small(): N {
  const dp = pick([0, 0, 0, 1, 2]);
  return { v: BigInt(int(9) - 4) * 10n ** BigInt(dp), dp };
}
const withCommodity = (c: string, x: N) => (c === "" ? digits(x, PLAIN) : `${digits(x, PLAIN)} ${c}`);
const assertion = () => `${pick(["=", "==", "=*", "==*"])}${chance(0.5) ? " " : ""}${withCommodity(pick(BALANCE_COMMODITIES), small())}`;

function balanceTransaction(): string {
  let t = `2026-01-0${1 + int(5)} t${int(9)}\n`;
  const dated = () => (chance(0.1) ? `  ; date:2026-01-0${1 + int(5)}` : "");
  const name = (account: string) => (chance(0.1) ? `(${account})` : account);
  const plain = (account: string, body: string) => `    ${name(account)}  ${body}${dated()}\n`;
  const account = () => pick(BALANCE_ACCOUNTS);
  const c = pick(BALANCE_COMMODITIES);
  switch (int(5)) {
    case 0: {
      // plain postings, some with an assertion; the last one balances (or is elided)
      let sum = 0n;
      const dp = pick([0, 0, 1, 2]);
      for (let k = 0, n = 1 + int(3); k < n; k++) {
        const x = { v: BigInt(int(9) - 4) * 10n ** BigInt(dp), dp };
        sum += x.v;
        t += plain(account(), `${withCommodity(c, x)}${chance(0.4) ? ` ${assertion()}` : ""}`);
      }
      t += plain(account(), chance(0.5) ? "" : `${withCommodity(c, { v: -sum, dp })}${chance(0.3) ? ` ${assertion()}` : ""}`);
      break;
    }
    case 1: {
      // assignments, the partner takes what is left
      for (let k = 0, n = 1 + int(2); k < n; k++) t += plain(account(), assertion());
      t += plain("z", "");
      break;
    }
    case 2: {
      // a plain posting, an assignment of the same or another account, an elided partner
      t += plain(account(), `${withCommodity(c, small())}${chance(0.4) ? ` ${assertion()}` : ""}`);
      t += plain(account(), assertion());
      t += plain("z", "");
      break;
    }
    case 3: {
      // lots and prices on the postings, then an assignment that zeroes them
      const lot = pick(["{10 EUR}", "{{30 EUR}}", "[2026-01-01] (l) {10 EUR}", "@ 3 EUR", "@@ 9 USD"]);
      t += plain(account(), `${withCommodity("X", small())} ${lot}`);
      if (chance(0.5)) t += plain(account(), withCommodity("X", small()));
      t += plain(account(), assertion());
      t += plain("z", "");
      break;
    }
    default: {
      // an assertion on the elided posting's account, before or after it, with an assignment
      t += plain("e", "");
      t += plain(account(), assertion());
      t += plain("e", `${withCommodity(c, { v: 0n, dp: 0 })} ${assertion()}`);
      break;
    }
  }
  return t;
}

function balanceJournal(): { text: string; second?: string } {
  let text = "";
  for (let k = 0, n = 1 + int(5); k < n; k++) text += `${balanceTransaction()}\n`;
  const second = chance(0.2) ? `${balanceTransaction()}\n` : undefined;
  return second === undefined ? { text } : { text, second };
}

function journal(): { text: string; second?: string } {
  if (chance(0.3)) return balanceJournal();
  const { style, header } = fileStyle();
  let text = header;
  if (chance(0.2)) text += "D 1.00 EUR\n";
  if (chance(0.1)) text += "alias e = expenses\n";
  if (chance(0.1)) text += "apply account top\n";
  if (chance(0.3)) text += `P 2026-01-01 X ${digits({ v: 2n, dp: 0 }, style)} EUR\n`;
  text += "\n";
  for (let k = 0, n = 1 + int(3); k < n; k++) text += `${transaction(style)}\n`;
  // Now and then a second file: directives must not reach it.
  const second = chance(0.15) ? `${transaction({ mark: ".", group: "" })}\n` : undefined;
  return second === undefined ? { text } : { text, second };
}

// ───────────── run ─────────────
const dir = mkdtempSync(join(tmpdir(), "ledgermill-fuzz-"));
const total = { PASS: 0, FAIL: 0, SKIP: 0 };
let valid = 0;
for (let k = 0; k < count; k++) {
  const j = journal();
  const paths = [join(dir, `f${k}.journal`)];
  writeFileSync(paths[0] as string, j.text);
  if (j.second !== undefined) {
    paths.push(join(dir, `f${k}b.journal`));
    writeFileSync(paths[1] as string, j.second);
  }
  const v: Verdict = await compareCase(paths);
  total[v.status]++;
  if (v.status === "PASS" && v.note === "") valid++;
  if (v.status === "FAIL") {
    console.log(`FAIL fuzz#${k} ${v.note}\n${j.text}${j.second ? `--- second file\n${j.second}` : ""}`);
  }
}
console.log(
  `fuzz (seed ${seed}): ${total.PASS} pass (${valid} valid, the rest fail in both), ${total.FAIL} fail, ${total.SKIP} skip`,
);
if (!keep) rmSync(dir, { recursive: true });
else console.log(`journals kept in ${dir}`);
if (total.FAIL > 0) process.exitCode = 1;
