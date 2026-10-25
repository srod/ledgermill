// Big, realistic, valid ledgers for the stress test and the bench (stress.ts, bench.ts).
//   hledgerJournal(transactions, seed)  → text of an hledger journal
//   beancountLedger(transactions, seed) → text of a Beancount ledger
// The same seed and count give the same text on every machine. One personal-finance story is
// generated once and written in either syntax: about thirty accounts, EUR with USD, GBP, VWCE and AAPL,
// 2 to 4 postings per transaction (some amounts left out), prices, purchases at `@`, `@@` and `{}`,
// sales of lots, balance assertions and comments. A 10,000 journal must pass `hledger -f F check` and
// `bean-check F`; check it by hand after changing this file.
import { rng } from "./rng.ts";

/** An amount in whole units of 10^-dp: 12345 with dp 2 is 123.45. */
type Money = { n: number; cur: string; dp: 0 | 2 | 4 };
type Cost =
  | { kind: "at"; price: Money } //            ` @ 0.92 EUR`
  | { kind: "total"; total: Money } //         ` @@ 46.00 EUR`
  | { kind: "lot"; lot: Money } //             ` {105.20 EUR}`: a purchase
  | { kind: "sell"; lot: Money; price: Money }; // a sale: Beancount ` {} @ 110.00 EUR` (FIFO), hledger ` {105.20 EUR} @ 110.00 EUR`
type Post = {
  account: string; // hledger's spelling, `assets:bank:checking`
  amount?: Money; // absent: the engine works it out (always for the gain of a sale)
  cost?: Cost;
  elide?: boolean; // leave the amount out although it is known
  assert?: Money; // hledger balance assertion after this posting
  comment?: string;
  only?: "hledger" | "beancount";
};
type Txn = { date: string; flag: "*" | "!" | ""; code?: string; payee: string; note: string; tag?: string; comment?: string; posts: Post[] };
type Entry =
  | { kind: "txn"; txn: Txn }
  | { kind: "price"; date: string; cur: string; rate: Money }
  | { kind: "balance"; date: string; account: string; amount: Money }; // Beancount only

const START = Date.UTC(2015, 0, 1);
const DAY = 86_400_000;
const isoDate = (day: number) => new Date(START + day * DAY).toISOString().slice(0, 10);

const ACCOUNTS = [
  "assets:bank:checking",
  "assets:bank:savings",
  "assets:cash",
  "assets:brokerage:vwce",
  "assets:brokerage:aapl",
  "assets:brokerage:cash-usd",
  "liabilities:creditcard",
  "equity:opening-balances",
  "income:salary",
  "income:interest",
  "income:dividends",
  "income:gains",
  "income:gifts",
  "expenses:food:groceries",
  "expenses:food:restaurants",
  "expenses:food:coffee",
  "expenses:housing:rent",
  "expenses:housing:electricity",
  "expenses:housing:internet",
  "expenses:housing:insurance",
  "expenses:housing:household",
  "expenses:transport:fuel",
  "expenses:transport:transit",
  "expenses:transport:car",
  "expenses:health:pharmacy",
  "expenses:health:doctor",
  "expenses:leisure:cinema",
  "expenses:leisure:books",
  "expenses:leisure:travel",
  "expenses:shopping:clothes",
  "expenses:shopping:electronics",
  "expenses:subscriptions:streaming",
  "expenses:subscriptions:software",
  "expenses:taxes:income",
  "expenses:fees:bank",
  "expenses:gifts",
];

// [account, min cents, max cents, payees]
const SPENDING: [string, number, number, string[]][] = [
  ["expenses:food:groceries", 800, 14000, ["Carrefour", "Lidl", "Monoprix", "Biocoop", "Picard", "Marché Bio"]],
  ["expenses:food:restaurants", 1200, 9000, ["Le Bistrot", "Pizzeria Roma", "Sushi Bar", "Café Noir", "Chez Léon"]],
  ["expenses:food:coffee", 180, 650, ["Café Noir", "Starbucks", "Boulangerie Paul"]],
  ["expenses:housing:electricity", 3500, 12000, ["EDF"]],
  ["expenses:housing:internet", 2999, 4999, ["Orange"]],
  ["expenses:housing:insurance", 1500, 6000, ["MAIF"]],
  ["expenses:housing:household", 500, 12000, ["Leroy Merlin", "Ikea", "Castorama"]],
  ["expenses:transport:fuel", 3000, 8500, ["Total", "Esso", "Shell"]],
  ["expenses:transport:transit", 200, 8600, ["RATP", "SNCF"]],
  ["expenses:transport:car", 2500, 60000, ["Norauto", "Midas"]],
  ["expenses:health:pharmacy", 300, 4500, ["Pharmacie Centrale", "Pharmacie du Marché"]],
  ["expenses:health:doctor", 2500, 8000, ["Dr Martin", "Dr Dupont"]],
  ["expenses:leisure:cinema", 900, 3200, ["UGC", "Pathé"]],
  ["expenses:leisure:books", 700, 4500, ["Fnac", "Librairie Dialogues"]],
  ["expenses:shopping:clothes", 1500, 15000, ["Decathlon", "Zara", "Uniqlo"]],
  ["expenses:shopping:electronics", 1900, 90000, ["Fnac", "Darty", "Amazon"]],
  ["expenses:subscriptions:streaming", 599, 1799, ["Netflix", "Spotify", "Disney+"]],
  ["expenses:subscriptions:software", 299, 4999, ["GitHub", "JetBrains", "1Password"]],
  ["expenses:fees:bank", 100, 900, ["Crédit Mutuel"]],
  ["expenses:gifts", 1500, 12000, ["Fleuriste", "Cadeaux & Cie"]],
];
const NOTES = ["", "", "", "weekly", "monthly", "online", "cash back", "refund pending", "with Sam", "for the flat"];
const TAGS = ["holiday", "work", "gift", "tax", "flat"];
const COMMODITIES = ["EUR", "USD", "GBP", "VWCE", "AAPL"];

const money = (n: number, cur: string, dp: 0 | 2 | 4 = 2): Money => ({ n, cur, dp });
function show({ n, cur, dp }: Money): string {
  if (dp === 0) return `${n} ${cur}`;
  const scale = 10 ** dp;
  const abs = Math.abs(n);
  return `${n < 0 ? "-" : ""}${Math.floor(abs / scale)}.${String(abs % scale).padStart(dp, "0")} ${cur}`;
}

/** The story of `count` transactions: the same entries are written in either syntax. */
function story(count: number, seed: number): Entry[] {
  const r = rng(seed);
  const int = (n: number) => Math.floor(r() * n);
  const pick = <T>(xs: readonly T[]): T => xs[int(xs.length)] as T;
  const chance = (p: number) => r() < p;
  const between = (lo: number, hi: number) => lo + int(hi - lo + 1);

  const entries: Entry[] = [];
  let txns = 0;
  let day = 0;
  // Rates in 1/10000: USD and GBP in EUR, VWCE in EUR, AAPL in USD.
  const rates: Record<string, number> = { USD: 9200, GBP: 11500, VWCE: 1052000, AAPL: 1900000 };
  const quoted: Record<string, string> = { USD: "EUR", GBP: "EUR", VWCE: "EUR", AAPL: "USD" };
  let bank = 0; // cents on assets:bank:checking, the account that has assertions
  let cashUsd = 0; // cents on assets:brokerage:cash-usd
  const lots: Record<string, { units: number; cost: number }[]> = { VWCE: [], AAPL: [] };

  const quote = (cur: string) => {
    entries.push({ kind: "price", date: isoDate(day), cur, rate: money(rates[cur] as number, quoted[cur] as string, 4) });
  };
  const walk = (cur: string) => {
    rates[cur] = Math.max(1000, Math.round((rates[cur] as number) * (1 + (r() - 0.5) * 0.03)));
    quote(cur);
  };
  /** Price with two places, as a transaction would write it. */
  const price2 = (cur: string) => Math.max(1, Math.round((rates[cur] as number) / 100));

  const emit = (t: Omit<Txn, "date" | "flag" | "payee" | "note"> & Partial<Pick<Txn, "payee" | "note">>) => {
    const x = r();
    const txn: Txn = {
      date: isoDate(day),
      flag: x < 0.9 ? "*" : x < 0.96 ? "!" : "",
      payee: "",
      note: "",
      ...t,
    };
    if (chance(0.05)) txn.code = String(between(1000, 9999));
    if (chance(0.06)) txn.tag = pick(TAGS);
    if (chance(0.08)) txn.comment = pick(["checked with the statement", "ask about the invoice", "split later", "receipt in the drawer"]);
    entries.push({ kind: "txn", txn });
    txns++;
  };
  /** A posting on the checking account; it tracks the balance, and now and then asserts it. */
  const checking = (cents: number, mayElide = true): Post => {
    bank += cents;
    if (mayElide && chance(0.01)) return { account: "assets:bank:checking", amount: money(cents, "EUR"), assert: money(bank, "EUR") };
    return { account: "assets:bank:checking", amount: money(cents, "EUR"), elide: mayElide && chance(0.5) };
  };
  const payFrom = (cents: number, payer: number): Post => {
    if (payer < 0.6) return checking(-cents);
    if (payer < 0.85) return { account: "liabilities:creditcard", amount: money(-cents, "EUR"), elide: chance(0.5) };
    return { account: "assets:cash", amount: money(-cents, "EUR"), elide: chance(0.5) };
  };

  const expense = () => {
    const [account, lo, hi, payees] = pick(SPENDING);
    const cents = between(lo, hi);
    const payee = pick(payees);
    const note = pick(NOTES);
    const posts: Post[] = [];
    if (account === "expenses:food:groceries" && chance(0.25)) {
      // a second category on the same receipt: 3 postings
      const extra = Math.min(between(300, 2500), cents - 100);
      posts.push({ account, amount: money(cents - extra, "EUR") });
      posts.push({ account: "expenses:housing:household", amount: money(extra, "EUR") });
    } else {
      posts.push({ account, amount: money(cents, "EUR"), comment: chance(0.05) ? "reimbursable" : undefined });
    }
    posts.push(payFrom(cents, r()));
    emit({ payee, note, posts });
  };
  const salary = () => {
    const gross = between(300000, 360000);
    const tax = Math.round(gross * 0.22);
    const save = 20000;
    emit({
      payee: "ACME Corp",
      note: "salary",
      posts: [
        { account: "income:salary", amount: money(-gross, "EUR") },
        { account: "expenses:taxes:income", amount: money(tax, "EUR") },
        { account: "assets:bank:savings", amount: money(save, "EUR") },
        checking(gross - tax - save),
      ],
    });
  };
  const rent = () => {
    emit({ payee: "Régie Immobilière", note: "rent", posts: [{ account: "expenses:housing:rent", amount: money(90000, "EUR") }, checking(-90000)] });
  };
  const cardPayment = () => {
    const cents = between(20000, 80000);
    emit({ payee: "Crédit Mutuel", note: "card statement", posts: [{ account: "liabilities:creditcard", amount: money(cents, "EUR") }, checking(-cents)] });
  };
  const toSavings = () => {
    const cents = between(5, 60) * 1000;
    emit({ payee: "Transfer", note: "to savings", posts: [{ account: "assets:bank:savings", amount: money(cents, "EUR") }, checking(-cents)] });
  };
  const interest = () => {
    const dividend = chance(0.5);
    const cents = between(200, 9000);
    emit({
      payee: dividend ? "Vanguard" : "Crédit Mutuel",
      note: dividend ? "dividend" : "interest",
      posts: [{ account: dividend ? "income:dividends" : "income:interest", amount: money(-cents, "EUR") }, checking(cents)],
    });
  };
  const gift = () => {
    const cents = between(2000, 10000);
    emit({ payee: "Aunt Claire", note: "birthday", posts: [{ account: "income:gifts", amount: money(-cents, "EUR") }, checking(cents)] });
  };
  /** Spending abroad: the foreign amount carries its price in EUR. */
  const abroad = () => {
    const cur = pick(["USD", "GBP"]);
    const units = between(8, 400);
    const rate = price2(cur);
    const eur = units * rate; // cents: whole units times a two-place rate
    const account = pick(["expenses:leisure:travel", "expenses:shopping:electronics", "expenses:subscriptions:software", "expenses:leisure:travel"]);
    const foreign = money(units * 100, cur);
    const cost: Cost = chance(0.3) ? { kind: "total", total: money(eur, "EUR") } : { kind: "at", price: money(rate, "EUR") };
    emit({ payee: pick(["Airbnb", "Booking.com", "Heathrow Express", "Amazon.com", "Uber"]), note: pick(NOTES), posts: [{ account, amount: foreign, cost }, checking(-eur)] });
  };
  const buyVwce = () => {
    const units = between(1, 20);
    const p = price2("VWCE");
    const total = units * p;
    lots.VWCE?.push({ units, cost: p });
    emit({
      payee: "Boursorama",
      note: "buy VWCE",
      posts: [{ account: "assets:brokerage:vwce", amount: money(units, "VWCE", 0), cost: { kind: "lot", lot: money(p, "EUR") } }, checking(-total)],
    });
  };
  const sellVwce = () => {
    const head = lots.VWCE?.[0];
    if (!head) return buyVwce();
    const units = Math.min(head.units, between(1, 10));
    const p = price2("VWCE");
    const proceeds = units * p;
    head.units -= units;
    if (head.units === 0) lots.VWCE?.shift();
    emit({
      payee: "Boursorama",
      note: "sell VWCE",
      posts: [
        { account: "assets:brokerage:vwce", amount: money(-units, "VWCE", 0), cost: { kind: "sell", lot: money(head.cost, "EUR"), price: money(p, "EUR") } },
        checking(proceeds, false),
        { account: "income:gains", only: "beancount" },
      ],
    });
  };
  /** Dollars on the brokerage cash account, bought with euros. Takes one transaction. */
  const topUp = (usd: number) => {
    const rate = price2("USD");
    const eur = usd * rate;
    cashUsd += usd * 100;
    emit({
      payee: "Transfer",
      note: "USD for the brokerage",
      posts: [{ account: "assets:brokerage:cash-usd", amount: money(usd * 100, "USD"), cost: { kind: "at", price: money(rate, "EUR") } }, checking(-eur)],
    });
  };
  const buyAapl = () => {
    const units = between(1, 8);
    const p = price2("AAPL");
    const total = units * p;
    if (cashUsd < total) return topUp(Math.ceil(total / 100) + between(100, 1000));
    cashUsd -= total;
    lots.AAPL?.push({ units, cost: p });
    emit({
      payee: "Interactive Brokers",
      note: "buy AAPL",
      posts: [
        { account: "assets:brokerage:aapl", amount: money(units, "AAPL", 0), cost: { kind: "lot", lot: money(p, "USD") } },
        { account: "assets:brokerage:cash-usd", amount: money(-total, "USD"), elide: chance(0.5) },
      ],
    });
  };
  const sellAapl = () => {
    const head = lots.AAPL?.[0];
    if (!head) return buyAapl();
    const units = Math.min(head.units, between(1, 5));
    const p = price2("AAPL");
    cashUsd += units * p;
    head.units -= units;
    if (head.units === 0) lots.AAPL?.shift();
    emit({
      payee: "Interactive Brokers",
      note: "sell AAPL",
      posts: [
        { account: "assets:brokerage:aapl", amount: money(-units, "AAPL", 0), cost: { kind: "sell", lot: money(head.cost, "USD"), price: money(p, "USD") } },
        { account: "assets:brokerage:cash-usd", amount: money(units * p, "USD") },
        { account: "income:gains", only: "beancount" },
      ],
    });
  };

  // The start: today's prices and the opening balances.
  if (count > 0) {
    for (const cur of Object.keys(rates)) quote(cur);
    bank = 2_000_000;
    emit({
      payee: "",
      note: "opening balances",
      posts: [
        { account: "assets:bank:checking", amount: money(bank, "EUR") },
        { account: "assets:bank:savings", amount: money(500_000, "EUR") },
        { account: "assets:cash", amount: money(20_000, "EUR") },
        { account: "equity:opening-balances", amount: money(-2_520_000, "EUR"), elide: true },
      ],
    });
  }
  while (txns < count) {
    if (chance(0.15)) {
      day += between(1, 2);
      if (chance(0.2)) walk(pick(Object.keys(rates)));
      if (chance(0.02)) entries.push({ kind: "balance", date: isoDate(day), account: "assets:bank:checking", amount: money(bank, "EUR") });
    }
    const x = r();
    if (txns % 40 === 12) salary();
    else if (txns % 45 === 20) rent();
    else if (x < 0.62) expense();
    else if (x < 0.66) cardPayment();
    else if (x < 0.69) toSavings();
    else if (x < 0.71) interest();
    else if (x < 0.72) gift();
    else if (x < 0.76) abroad();
    else if (x < 0.79) buyVwce();
    else if (x < 0.805) sellVwce();
    else if (x < 0.825) buyAapl();
    else if (x < 0.835) sellAapl();
    else expense();
  }
  return entries;
}

// ───────────── writing ─────────────
const NAMES: Record<string, string> = {};
/** `assets:bank:checking` as Beancount spells it: `Assets:Bank:Checking`. */
const beanAccount = (a: string) => (NAMES[a] ??= a.split(":").map((s) => s.charAt(0).toUpperCase() + s.slice(1)).join(":"));

// A `{}` lot is no cost to hledger: next to one, the amount of the other posting must be written.
function hledgerPosting(p: Post, elidable: boolean): string {
  let line = `    ${p.account}`;
  if (p.amount && !(p.elide && elidable)) {
    line += `  ${show(p.amount)}`;
    const c = p.cost;
    if (c?.kind === "at") line += ` @ ${show(c.price)}`;
    else if (c?.kind === "total") line += ` @@ ${show(c.total)}`;
    else if (c?.kind === "lot") line += ` {${show(c.lot)}}`;
    else if (c?.kind === "sell") line += ` {${show(c.lot)}} @ ${show(c.price)}`;
    if (p.assert) line += ` = ${show(p.assert)}`;
  }
  return p.comment ? `${line}  ; ${p.comment}` : line;
}

/** The text of an hledger journal with exactly `transactions` transactions. */
export function hledgerJournal(transactions: number, seed: number): string {
  const out: string[] = ["; Personal finances, generated by harness/bigjournal.ts", ""];
  out.push("commodity 1,000.00 EUR", "commodity 1,000.00 USD", "commodity 1,000.00 GBP", "commodity 1,000 VWCE", "commodity 1,000 AAPL", "");
  for (const a of ACCOUNTS) out.push(`account ${a}`);
  out.push("");
  for (const e of story(transactions, seed)) {
    if (e.kind === "price") out.push(`P ${e.date} ${e.cur} ${show(e.rate)}`);
    else if (e.kind === "txn") {
      const t = e.txn;
      const what = t.payee && t.note ? `${t.payee} | ${t.note}` : t.payee || t.note;
      const comment = [t.comment, t.tag && `topic:${t.tag}`].filter(Boolean).join(", ");
      out.push(`${t.date}${t.flag ? ` ${t.flag}` : ""}${t.code ? ` (${t.code})` : ""} ${what}${comment ? `  ; ${comment}` : ""}`);
      const elidable = !t.posts.some((p) => p.cost?.kind === "lot");
      for (const p of t.posts) if (p.only !== "beancount") out.push(hledgerPosting(p, elidable));
      out.push("");
    }
  }
  return `${out.join("\n")}\n`;
}

function beancountPosting(p: Post): string {
  let line = `  ${beanAccount(p.account)}`;
  if (p.amount && !p.elide) {
    line += `  ${show(p.amount)}`;
    const c = p.cost;
    if (c?.kind === "at") line += ` @ ${show(c.price)}`;
    else if (c?.kind === "total") line += ` @@ ${show(c.total)}`;
    else if (c?.kind === "lot") line += ` {${show(c.lot)}}`;
    else if (c?.kind === "sell") line += ` {} @ ${show(c.price)}`;
  }
  return p.comment ? `${line}  ; ${p.comment}` : line;
}

const quoteBean = (s: string) => `"${s.replaceAll('"', "'")}"`;

/** The text of a Beancount ledger with exactly `transactions` transactions. */
export function beancountLedger(transactions: number, seed: number): string {
  const out: string[] = ["; Personal finances, generated by harness/bigjournal.ts", ""];
  for (const c of COMMODITIES) out.push(`2015-01-01 commodity ${c}`);
  out.push("");
  for (const a of ACCOUNTS) {
    const booking = a.startsWith("assets:brokerage:") && !a.endsWith("cash-usd") ? ` ${a.endsWith("vwce") ? "VWCE" : "AAPL"} "FIFO"` : "";
    out.push(`2015-01-01 open ${beanAccount(a)}${booking}`);
  }
  out.push("");
  for (const e of story(transactions, seed)) {
    if (e.kind === "price") out.push(`${e.date} price ${e.cur} ${show(e.rate)}`);
    else if (e.kind === "balance") out.push(`${e.date} balance ${beanAccount(e.account)}  ${show(e.amount)}`);
    else {
      const t = e.txn;
      const text = t.payee ? `${quoteBean(t.payee)} ${quoteBean(t.note)}` : quoteBean(t.note);
      const tag = t.tag ? ` #${t.tag}` : "";
      const comment = t.comment ? ` ; ${t.comment}` : "";
      out.push(`${t.date} ${t.flag === "!" ? "!" : "*"} ${text}${tag}${comment}`);
      for (const p of t.posts) if (p.only !== "hledger") out.push(beancountPosting(p));
      out.push("");
    }
  }
  return `${out.join("\n")}\n`;
}
