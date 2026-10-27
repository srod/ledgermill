// bun harness/fuzz.ts [COUNT] [SEED] [--keep] [--save DIR] [--save-skipped]
// bun harness/fuzz.ts --graph [COUNT] [SEED] [--keep] [--save DIR] [--save-skipped]
// bun harness/fuzz.ts --beancount [COUNT] [SEED] [--keep] [--save DIR] [--save-skipped]
// bun harness/fuzz.ts --both-ways [COUNT] [SEED] [--keep] [--save DIR] [--save-skipped]
// Random journals, mostly valid, compared like the corpus: hledger against our wasm. Covers
// amount forms, decimal marks and digit groups, directives, prices, lots, elided amounts,
// inferred prices, conversion postings, virtual postings, balance assertions and assignments (small
// numbers, so that assertions often hold) and big numbers. Journals sometimes have `P` lines. Each
// journal is compared raw, formatted (`format()` must keep what hledger reads), under `--cost` and under one or two random `--value=DATE,COMM`.
// With `--graph`: dense random networks of prices (9 to 11 commodities, 1 to 16 `P` lines) valued
// by two random `--value` runs each, raw and `--cost` too. Each journal is also asked two `holdings()` questions
// (no end date, and one end date; a random valuation commodity). `--save DIR` writes the journals that fail
// there, with their questions. With `--beancount`: random Beancount journals (price directives among seven
// commodities, lots, price annotations, plain units of either sign) compared against bean-check and bean-query
// for their transactions and postings, their prices, their cost and, at several dates and commodities,
// their value and holdings. With `--both-ways`: Beancount journals of 3 or 4 commodities with up to 9 prices, several pairs
// priced in both directions at rates whose inverse is not exact (3, 7, 11, 0.3), a few one way, and plain holdings, compared
// the same way: the evidence for "a pair priced both ways" in spec/beancount/convert-rates.md. Every case must
// PASS or SKIP; any FAIL is printed with its journal and makes the exit code 1. The summary counts the SKIPs by what
// was refused; a SKIP for a refusal not in `EXPECTED_SKIPS` (lib.ts) is printed like a FAIL and fails the run too.
// `--save-skipped` makes `--save DIR` keep the journals with a SKIP as well, to rerun one with `real.ts`.
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  beanFormatLine,
  type Comparison,
  compareAll,
  count as countVerdict,
  counts as newCounts,
  countsLine,
  EXPECTED_SKIPS,
  formatLine,
  type HoldingQuestion,
  type Query,
  skipReason,
  summarize,
  tallies,
  tally,
  unexpectedSkips,
} from "./lib.ts";
import { rng } from "./rng.ts";

const saveAt = process.argv.indexOf("--save");
const saveDir = saveAt < 0 ? null : (process.argv[saveAt + 1] ?? null);
const args = process.argv.slice(2).filter((a, i, all) => !a.startsWith("--") && all[i - 1] !== "--save");
const count = Number(args[0] ?? 500);
const seed = Number(args[1] ?? 1);
const keep = process.argv.includes("--keep");
const saveSkipped = process.argv.includes("--save-skipped");
const graph = process.argv.includes("--graph");
const bothWays = process.argv.includes("--both-ways");
const beancount = bothWays || process.argv.includes("--beancount");

const r = rng(seed);
// Its own stream: asking holdings questions must not change the journals a seed gives.
const rq = rng(seed ^ 0x5bd1e995);
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
  let text = chance(0.3) ? `${priceLines(1 + int(3), ["EUR", "USD", "X"], PLAIN)}\n` : "";
  for (let k = 0, n = 1 + int(5); k < n; k++) text += `${balanceTransaction()}\n`;
  const second = chance(0.2) ? `${balanceTransaction()}\n` : undefined;
  return second === undefined ? { text } : { text, second };
}

// ───────────── rules ─────────────
// Automated (`=`) and periodic (`~`) rules are read and ignored, so a journal with them gives the
// transactions it gives without them. Their postings are read like any posting, and in a rule `*`
// may stand before an amount.
const QUERIES = ["expenses", "acct:a", "desc:foo", "re:[", "date:garbage", "amt:>10", ""] as const;
const PERIODS = ["monthly", "every 2 weeks", "weekly from 2026-01-01", "every 5th day of month", "every jan 5th", "daily in 2026"] as const;
function rule(style: Style): string {
  const automated = chance(0.5);
  const query = pick(QUERIES);
  let t = automated ? `=${query ? " " : ""}${query}\n` : `~ ${pick(PERIODS)}${chance(0.2) ? "  a description" : ""}\n`;
  for (let k = 0, n = int(4); k < n; k++) {
    const c = pick(COMMODITIES);
    const x = randomN();
    const star = automated && chance(0.4) ? "*" : "";
    const tail = pick(["", "", ` @ ${amount(pick(COMMODITIES), randomN(), style)}`, ` = ${amount(c, x, style)}`, " {3 EUR}", " ((x))"]);
    t += `    ${pick(["(budget)", "a", "[x]", "* b"])}  ${star}${amount(c, x, style)}${tail}${comment()}\n`;
  }
  return t;
}

// ───────────── prices ─────────────
/** `P` lines: dates 2026-01-01 to 06 (the same day twice sometimes), a time of day now and then, prices that are zero or negative now and then. */
function priceLines(count: number, commodities: readonly string[], style: Style): string {
  let t = "";
  for (let k = 0; k < count; k++) {
    const from = pick(commodities);
    const to = chance(0.95) ? pick(commodities.filter((c) => c !== from)) : from;
    const rate = chance(0.1) ? { v: 0n, dp: 0 } : randomN(pick([0, 1, 2, 5, 10]));
    const price = chance(0.9) && rate.v < 0n ? neg(rate) : rate;
    const time = chance(0.15) ? ` ${pick(["09:00", "10:30:15", "23:59"])}` : "";
    t += `P 2026-01-0${1 + int(6)}${time} ${from} ${amount(to, price, style)}\n`;
  }
  return t;
}

/** A random `--value` question: a date around the `P` lines, a commodity of the fuzzer's. */
function randomQuery(commodities: readonly string[], rand: () => number = r): Query {
  const day = Math.floor(rand() * 10);
  return { date: day === 0 ? "2025-12-31" : `2026-01-${String(day).padStart(2, "0")}`, commodity: (commodities[Math.floor(rand() * commodities.length)] as string).replace(/^"|"$/g, "") };
}

function journal(): { text: string; second?: string } {
  if (chance(0.3)) return balanceJournal();
  const { style, header } = fileStyle();
  let text = header;
  if (chance(0.2)) text += "D 1.00 EUR\n";
  if (chance(0.1)) text += "alias e = expenses\n";
  if (chance(0.1)) text += "apply account top\n";
  if (chance(0.45)) text += priceLines(1 + int(4), COMMODITIES, style);
  text += "\n";
  for (let k = 0, n = 1 + int(3); k < n; k++) {
    if (chance(0.25)) text += `${rule(style)}\n`;
    text += `${transaction(style)}\n`;
  }
  // Now and then a second file: directives must not reach it.
  const second = chance(0.15) ? `${transaction({ mark: ".", group: "" })}\n` : undefined;
  return second === undefined ? { text } : { text, second };
}

// ───────────── dense networks of prices ─────────────
const GRAPH_COMMODITIES = ["A", "B", "C", "D", "E", "F", "G", "H", "Z", "a", "EUR", "USD"] as const;
const GRAPH_RATES = ["2", "3", "7", "0.5", "1.25", "0.3333333333", "1000.5", "0", "-4", "1.000", "0.00000000005", "12345.6789", "0.1", "1.1", "0.07", "0.3"] as const;
const GRAPH_QUANTITIES = ["1", "3", "9", "0.9", "2.7", "0.207", "100", "-5", "0.01", "12345.678", "0.0000000009", "1.50"] as const;

/** 9 to 11 commodities, 1 to 16 prices among them, an amount of each commodity to value, and now and then a posting with several amounts. */
function graphJournal(): { text: string; used: string[] } {
  const pool = [...GRAPH_COMMODITIES];
  for (let k = pool.length - 1; k > 0; k--) {
    const j = int(k + 1);
    [pool[k], pool[j]] = [pool[j] as string, pool[k] as string];
  }
  const used = pool.slice(0, 9 + int(3));
  let text = "";
  for (let k = 0, n = 1 + int(16); k < n; k++) {
    const from = pick(used);
    const to = chance(0.95) ? pick(used.filter((c) => c !== from)) : from;
    text += `P 2026-01-0${1 + int(6)} ${from} ${pick(GRAPH_RATES)} ${to}\n`;
  }
  text += "\n2026-01-03 t\n";
  used.forEach((c, k) => {
    const priced = chance(0.15) ? ` @ ${pick(["2", "3.5"])} ${pick(used)}` : "";
    const lot = chance(0.1) ? " {1 EUR}" : "";
    text += `  (v${k})  ${pick(GRAPH_QUANTITIES)} ${c}${priced}${lot}\n`;
  });
  if (chance(0.5)) {
    // Several amounts in one posting: they must merge after conversion.
    text += `  a  ${pick(["1", "2.5"])} ${pick(used)}\n  a  ${pick(["3", "0.7"])} ${pick(used)}\n  b  ${pick(["-1", "-4.25"])} ${pick(used)}\n  c\n`;
  }
  return { text, used };
}

// ───────────── Beancount ─────────────
const BEAN_COMMODITIES = ["EUR", "USD", "CHF", "AAA", "BBB", "CCC", "DDD"] as const;
const BEAN_RATES = ["2", "3", "7", "0.5", "1.25", "0.3333333333", "1000.5", "0", "-4", "1.000", "0.00000000005", "12345.6789", "0.1", "1.1", "0.07", "0.3", "62.2", "35884.57"] as const;
// No tiny units and no cost of zero: interpolating those next to a cost is not what this asks about.
const BEAN_UNITS = ["1", "3", "9", "0.9", "2.50", "0.207", "100", "1.5", "2.25", "12345.678", "0.01", "10"] as const;
const BEAN_ACCOUNTS = ["Assets:A", "Assets:B", "Assets:C", "Assets:A:Sub"] as const;
const BEAN_LOT_ACCOUNTS = ["Assets:L1", "Assets:L2", "Assets:L2:Sub"] as const;

/**
 * A Beancount journal that bean-check accepts: prices of random pairs on random days (out of order, twice a day,
 * zero and negative now and then, one in 2099 now and then), and transactions whose postings are plain units of
 * either sign, units bought at a cost, or units with a price; `Equity:O` takes the rest. A cost is never negative and
 * units at a cost never zero or negative (booking errors are not what this asks about).
 */
function beanJournal(): { text: string; used: string[] } {
  const used = BEAN_COMMODITIES.filter(() => chance(0.8));
  if (used.length < 3) used.push("EUR", "USD", "AAA");
  const others = (c: string) => used.filter((u) => u !== c);
  let text = 'option "operating_currency" "EUR"\n';
  for (const a of [...BEAN_ACCOUNTS, ...BEAN_LOT_ACCOUNTS, "Equity:O"]) text += `2026-01-01 open ${a}\n`;
  text += "\n";
  for (let k = 0, n = int(14); k < n; k++) {
    const from = pick(used);
    const to = pick(others(from));
    const day = chance(0.05) ? "2099-01-01" : `2026-01-0${1 + int(6)}`;
    const rate = pick(BEAN_RATES);
    text += `${day} price ${from} ${chance(0.1) ? rate.replace(/^-/, "") : rate} ${to}\n`;
  }
  text += "\n";
  for (let k = 0, n = 1 + int(5); k < n; k++) {
    text += `2026-01-0${1 + int(6)} * "t${k}"\n`;
    // One currency is left to the elided posting: costs and prices are in `base`, and plain units of another commodity
    // come with their offset. Several residual currencies, and lots next to plain units of their commodity in one
    // account, are booking shapes the transactions comparison owns, not what these reads ask about.
    const base = pick(used);
    const nonBase = used.filter((u) => u !== base);
    for (let p = 0, m = 1 + int(4); p < m; p++) {
      const c = pick(nonBase);
      const account = pick(BEAN_ACCOUNTS);
      const units = pick(BEAN_UNITS);
      const roll = r();
      if (roll < 0.3) text += `  ${pick(BEAN_LOT_ACCOUNTS)}  ${units} ${c} {${pick(["3", "4.5", "1.25", "0.1", "2"])} ${base}}\n`;
      else if (roll < 0.4) text += `  ${account}  ${units} ${c} @ ${pick(["2", "3.5", "0.8"])} ${base}\n`;
      else if (roll < 0.7) {
        const sign = chance(0.4) ? "-" : "";
        text += `  ${account}  ${sign}${units} ${c}\n  Equity:O  ${sign ? "" : "-"}${units} ${c}\n`;
      } else text += `  ${account}  ${chance(0.4) ? "-" : ""}${units} ${base}\n`;
    }
    text += "  Equity:O\n\n";
  }
  return { text, used };
}

// ───────────── Beancount, pairs priced both ways ─────────────
// Rates whose inverse is not a short decimal (1/3, 1/7, 1/11, 1/0.3 ...), and a few exact ones (the rule hides on those). No zero:
// a zero in a pair priced both ways is refused (`price-both-ways`), which is a SKIP and says nothing about the rule.
const BOTH_WAYS_RATES = ["3", "7", "11", "0.3", "13", "6", "0.7", "1.1", "2.5", "1.7", "9", "0.15", "2", "4", "0.5"] as const;

/**
 * A Beancount journal of 3 or 4 commodities and up to 9 `price` directives: two or three pairs priced in both directions
 * (one to three prices each way), the others one way. The prices come in any order and on any of six days, the same
 * day twice now and then. Plain holdings in each commodity are there for `value()` and `holdings()` to value.
 */
function bothWaysJournal(): { text: string; used: string[] } {
  const pool = [...BEAN_COMMODITIES];
  for (let k = pool.length - 1; k > 0; k--) {
    const j = int(k + 1);
    [pool[k], pool[j]] = [pool[j] as string, pool[k] as string];
  }
  const used = pool.slice(0, 3 + int(2));
  const pairs: [string, string][] = [];
  for (const a of used) for (const b of used) if (a < b) pairs.push([a, b]);
  for (let k = pairs.length - 1; k > 0; k--) {
    const j = int(k + 1);
    [pairs[k], pairs[j]] = [pairs[j] as [string, string], pairs[k] as [string, string]];
  }
  const both = 2 + int(2);
  const prices: string[] = [];
  pairs.forEach(([a, b], k) => {
    const way = (from: string, to: string, n: number) => {
      for (let i = 0; i < n; i++) prices.push(`2026-01-0${1 + int(6)} price ${from} ${pick(BOTH_WAYS_RATES)} ${to}`);
    };
    if (k < both) {
      way(a, b, 1 + int(3));
      way(b, a, 1 + int(3));
    } else if (k < both + 2 && chance(0.7)) way(...(chance(0.5) ? [a, b] : [b, a]), 1 + int(2));
  });
  for (let k = prices.length - 1; k > 0; k--) {
    const j = int(k + 1);
    [prices[k], prices[j]] = [prices[j] as string, prices[k] as string];
  }
  let text = 'option "operating_currency" "EUR"\n';
  for (const a of [...BEAN_ACCOUNTS, "Equity:O"]) text += `2026-01-01 open ${a}\n`;
  text += `\n${prices.slice(0, 9).join("\n")}\n\n`;
  for (let k = 0, n = 1 + int(3); k < n; k++) {
    text += `2026-01-0${1 + int(6)} * "t${k}"\n`;
    for (const c of used) {
      if (!chance(0.7)) continue;
      const units = `${chance(0.2) ? "-" : ""}${pick(BEAN_UNITS)}`;
      text += `  ${pick(BEAN_ACCOUNTS)}  ${units} ${c}\n  Equity:O  ${units.startsWith("-") ? units.slice(1) : `-${units}`} ${c}\n`;
    }
    text += "\n";
  }
  return { text, used };
}

// ───────────── run ─────────────
const dir = mkdtempSync(join(tmpdir(), "ledgermill-fuzz-"));
const raw = newCounts();
const t = tallies();
let valid = 0;
for (let k = 0; k < count; k++) {
  const j = bothWays ? bothWaysJournal() : beancount ? beanJournal() : graph ? graphJournal() : journal();
  const second = "second" in j ? j.second : undefined;
  const paths = [join(dir, `f${k}.${beancount ? "beancount" : "journal"}`)];
  writeFileSync(paths[0] as string, j.text);
  if (second !== undefined) {
    paths.push(join(dir, `f${k}b.journal`));
    writeFileSync(paths[1] as string, second);
  }
  const commodities = "used" in j ? j.used : [...COMMODITIES];
  const asked = Array.from({ length: 2 }, (_, i): HoldingQuestion => {
    const q = randomQuery(commodities, rq);
    return { end: i === 0 ? null : q.date, commodity: q.commodity };
  });
  const queries = Array.from({ length: graph ? 2 : 1 + int(2) }, () => randomQuery(commodities));
  const c: Comparison = await compareAll(paths, false, queries, { holdings: asked });
  countVerdict(raw, c.raw);
  tally(c, t);
  if (c.raw.status === "PASS" && c.raw.note === "") valid++;
  const v = summarize(c);
  const unexpected = v.status === "SKIP" && !EXPECTED_SKIPS.has(skipReason(v.note));
  if (v.status === "FAIL" || unexpected) {
    const queried = c.values.find((x) => x.verdict.status === v.status);
    console.log(`${unexpected ? "UNEXPECTED SKIP" : "FAIL"} fuzz#${k} ${v.note}${queried ? `  (--value=${queried.query.date},${queried.query.commodity})` : ""}\n${j.text}${second ? `--- second file\n${second}` : ""}`);
  }
  if (v.status === "FAIL" || (v.status === "SKIP" && (unexpected || saveSkipped))) {
    if (saveDir) {
      mkdirSync(saveDir, { recursive: true });
      const questions = asked.map((q) => `${q.end ?? "-"},${q.commodity}`).join(" ");
      const name = join(saveDir, `${bothWays ? "both" : beancount ? "bean" : graph ? "graph" : "fuzz"}-${seed}-${k}`);
      writeFileSync(`${name}.${beancount ? "beancount" : "journal"}`, `; ${v.note}\n; holdings questions (end,commodity): ${questions}\n${j.text}`);
      if (second !== undefined) writeFileSync(`${name}.second.journal`, second);
    }
  }
}
const counts = countsLine;
console.log(`fuzz${graph ? " --graph" : ""}${bothWays ? " --both-ways" : beancount ? " --beancount" : ""} (seed ${seed}): raw ${raw.PASS} pass (${valid} valid, the rest fail in both), ${countsLine(raw).replace(/^\d+ pass, /, "")}`);
if (beancount) {
  const b = t.beancount;
  console.log(`  format: ${beanFormatLine(t)}\n  prices: ${counts(b.prices)}\n  cost: ${counts(b.cost)}\n  value: ${counts(b.value)}\n  holdings: ${counts(b.holdings)}`);
} else {
  console.log(`  postings: ${counts(t.postings)}\n  prices: ${counts(t.prices)}\n  format: ${formatLine(t)}\n  --cost: ${counts(t.cost)}\n  --value: ${counts(t.value)}\n  holdings: ${counts(t.holdings)}`);
}
if (!keep) rmSync(dir, { recursive: true });
else console.log(`journals kept in ${dir}`);
const b = t.beancount;
const unexpected = unexpectedSkips(raw, t.format, t.beanFormat.format, t.cost, t.value, t.postings, t.prices, t.holdings, b.cost, b.value, b.prices, b.holdings);
for (const [k, n] of Object.entries(unexpected)) console.log(`UNEXPECTED SKIP ${k} (${n}): a refusal not in EXPECTED_SKIPS (lib.ts)`);
if (raw.FAIL + t.format.FAIL + t.beanFormat.format.FAIL + t.cost.FAIL + t.value.FAIL + t.postings.FAIL + t.prices.FAIL + t.holdings.FAIL + b.cost.FAIL + b.value.FAIL + b.prices.FAIL + b.holdings.FAIL > 0 || Object.keys(unexpected).length > 0) process.exitCode = 1;
