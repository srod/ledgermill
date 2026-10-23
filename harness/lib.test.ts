// bun test harness/lib.test.ts (needs the wasm built into pkg/, like the harness itself)
import { expect, test } from "bun:test";
import { join } from "node:path";
import {
  type Comparison,
  compareAll,
  compareCost,
  compareFirstError,
  compareRejection,
  compareValue,
  firstHledgerError,
  csvAccount,
  decimalString,
  decodeCsv,
  decodeHledgerPostings,
  decodePrices,
  diffPostings,
  diffPrices,
  givenName,
  rateCommodity,
  firstBeancountError,
  listsComparable,
  firstDiff,
  priceLines,
  summarize,
  valueQueries,
  projectBeancount,
  projectFlatPostings,
  projectHledger,
  withoutTrailingZeros,
} from "./lib.ts";

test("decimalString keeps places and sign exactly", () => {
  expect(decimalString(-1250n, 2)).toBe("-12.50");
  expect(decimalString(5n, 3)).toBe("0.005");
  expect(decimalString(0n, 2)).toBe("0.00");
  expect(decimalString(12n, 0)).toBe("12");
  expect(decimalString(123456789012345678901234567n, 2)).toBe("1234567890123456789012345.67");
});

const tx = (mantissa: string, places: number) =>
  `[{"tdate":"2026-01-01","tstatus":"Cleared","tdescription":"x","tcomment":" c \\n","tsourcepos":[{"sourceName":"/a/b.journal","sourceLine":1},{"sourceName":"/a/b.journal","sourceLine":3}],` +
  `"tpostings":[{"paccount":"a","ptype":"RegularPosting","pcomment":"","pamount":[{"acommodity":"EUR","aquantity":{"decimalMantissa":${mantissa},"decimalPlaces":${places},"floatingPoint":0}}]}]}]`;

test("projectHledger reads big and scientific mantissas exactly", () => {
  const [t] = projectHledger(tx("1.0e20", 2));
  expect(t?.postings[0]?.amounts[0]?.quantity).toBe("1000000000000000000.00");
  const [u] = projectHledger(tx("123456789012345678901", 0));
  expect(u?.postings[0]?.amounts[0]?.quantity).toBe("123456789012345678901");
  expect(u?.status).toBe("*");
  expect(u?.comment).toBe("c");
});

test("firstDiff names the first differing path", () => {
  expect(firstDiff({ a: [1, 2] }, { a: [1, 2] })).toBeNull();
  expect(firstDiff({ a: [1, { b: "x" }] }, { a: [1, { b: "y" }] })).toBe('.a[1].b: hledger="x" ours="y"');
  expect(firstDiff([1], [1, 2])).toBe("(root): length hledger=1 ours=2");
});

test("decodeCsv reads a quoted comma, a doubled quote and a newline in one field", () => {
  expect(decodeCsv('a,b\r\n"x, ""y""\nz",2\r\n')).toEqual([
    ["a", "b"],
    ['x, "y"\nz', "2"],
  ]);
});

test("decodeCsv takes CRLF and LF rows alike, with or without a final newline", () => {
  expect(decodeCsv("a,b\r\n1,2")).toEqual([["a", "b"], ["1", "2"]]);
  expect(decodeCsv("a,b\n1,\n")).toEqual([["a", "b"], ["1", ""]]);
});

const ENTRIES = "id,type,filename,lineno,date,flag,payee,narration";
const POSTINGS =
  "id,account,posting_flag,number,currency,cost_number,cost_currency,cost_date,cost_label,number(price),currency(price),filename,lineno";

test("projectBeancount trims the padding bean-query puts around numbers", () => {
  const entries = `${ENTRIES}\r\nt1,transaction,/a/b.beancount,4,2026-01-02,*,,pay\r\n`;
  const postings =
    `${POSTINGS}\r\n` +
    "t1,Assets:A,,   1   ,STK,  10.50  ,EUR,2026-01-02,,   2   ,EUR,/a/b.beancount,5\r\n";
  const [t] = projectBeancount(entries, postings);
  const amount = t?.postings[0]?.amounts[0];
  expect(amount?.quantity).toBe("1");
  expect(amount?.lot?.cost?.quantity).toBe("10.50");
  expect(amount?.cost).toEqual({ kind: "unit", amount: { commodity: "EUR", quantity: "2" } });
});

test("projectBeancount keeps a quoted narration with a comma, a quote and a newline", () => {
  const entries = `${ENTRIES}\r\nt1,transaction,b.beancount,4,2026-01-02,*,"Pay, Ltd","say ""hi""\nthere"\r\n`;
  const [t] = projectBeancount(entries, `${POSTINGS}\r\n`);
  expect(t?.payee).toBe("Pay, Ltd");
  expect(t?.description).toBe('say "hi"\nthere');
});

test("projectBeancount starts at the header when errors come first", () => {
  const noise = "/a/b.beancount:4: Transaction does not balance: (-0.4 EUR)\n\n   2026-01-02 * \"x\"\n\n";
  const entries = `${noise}${ENTRIES}\r\nt1,transaction,b.beancount,4,2026-01-02,*,,x\r\n`;
  const postings = `${noise}${POSTINGS}\r\nt1,Assets:A,,10,EUR,,,,,,,b.beancount,5\r\n`;
  const [t] = projectBeancount(entries, postings);
  expect(t?.description).toBe("x");
  expect(t?.postings.length).toBe(1);
  expect(() => projectBeancount("nothing here", postings)).toThrow("no CSV header");
});

test("projectBeancount keeps a transaction that has no postings", () => {
  const entries = `${ENTRIES}\r\nt1,transaction,b.beancount,4,2026-01-02,*,,empty\r\nt2,transaction,b.beancount,6,2026-01-03,!,,full\r\n`;
  const postings = `${POSTINGS}\r\nt2,Assets:A,,1,EUR,,,,,,,b.beancount,7\r\n`;
  const txs = projectBeancount(entries, postings);
  expect(txs.map((t) => t.postings.length)).toEqual([0, 1]);
  expect(txs[1]?.status).toBe("!");
});

test("projectBeancount keeps two postings with the same id and line (a sale over several lots)", () => {
  const entries = `${ENTRIES}\r\nt1,transaction,b.beancount,14,2026-01-10,*,,sell\r\n`;
  const postings =
    `${POSTINGS}\r\n` +
    "t1,Assets:Stock,,-1,BTC,10,EUR,2026-01-02,,,,b.beancount,15\r\n" +
    "t1,Assets:Stock,,-1,BTC,30,EUR,2026-01-03,,,,b.beancount,15\r\n";
  const [t] = projectBeancount(entries, postings);
  expect(t?.postings.map((p) => [p.line, p.amounts[0]?.lot?.cost?.quantity])).toEqual([
    [15, "10"],
    [15, "30"],
  ]);
});

const FLAT = "date,flag,payee,narration,account,number,currency,filename,lineno";

test("projectFlatPostings trims numbers and leaves a missing payee out, keeping rows in the order given", () => {
  const csv =
    `${FLAT}\r\n` +
    "2026-01-04,*,,pay,Assets:A,   8.50   ,EUR,/a/b.beancount,28\r\n" +
    '2026-01-05,!,"Shop, Ltd","say ""hi""",Assets:B,  -1  ,BTC,/a/b.beancount,36\r\n' +
    '2026-01-05,!,"Shop, Ltd","say ""hi""",Assets:B,  -2  ,BTC,/a/b.beancount,36\r\n';
  expect(projectFlatPostings(csv)).toEqual([
    { date: "2026-01-04", flag: "*", narration: "pay", account: "Assets:A", kind: "regular", number: "8.50", currency: "EUR", file: "/a/b.beancount", line: 28 },
    { date: "2026-01-05", flag: "!", payee: "Shop, Ltd", narration: 'say "hi"', account: "Assets:B", kind: "regular", number: "-1", currency: "BTC", file: "/a/b.beancount", line: 36 },
    { date: "2026-01-05", flag: "!", payee: "Shop, Ltd", narration: 'say "hi"', account: "Assets:B", kind: "regular", number: "-2", currency: "BTC", file: "/a/b.beancount", line: 36 },
  ]);
});

test("projectFlatPostings starts at the header when errors come first and takes an empty result", () => {
  const noise = "/a/b.beancount:4: Transaction does not balance: (-0.4 EUR)\n\n";
  expect(projectFlatPostings(`${noise}${FLAT}\r\n`)).toEqual([]);
  expect(() => projectFlatPostings("nothing here")).toThrow("no CSV header");
});

test("firstBeancountError takes the file and line of the first error, not the copy of the directive under it", () => {
  const output =
    "/tmp/a dir/check-order.beancount:39: Invalid account name: Foo:Bad\n\n   2026-01-03 open Foo:Bad\n\n" +
    "/tmp/a dir/check-order.beancount:0: Unbalanced pushed tag: 'left-open'\n";
  expect(firstBeancountError(output)).toEqual({ file: "check-order.beancount", line: 39 });
});

test("firstBeancountError reads a file-level error on line 0 and the include error on <load>", () => {
  expect(firstBeancountError("b.beancount:0: Unbalanced pushed tag: 'x'\n")).toEqual({ file: "b.beancount", line: 0 });
  expect(firstBeancountError('<load>:0: File glob "other.beancount" does not match any files\n')).toEqual({
    file: "<load>",
    line: 0,
  });
});

test("firstBeancountError skips the indented copy of a directive that looks like FILE:LINE and finds nothing in plain text", () => {
  expect(firstBeancountError("   2026-01-03 balance Assets:A  1 EUR: 5: x\n")).toBeNull();
  expect(firstBeancountError("")).toBeNull();
});

const finding = (file: string, line: number, severity: "error" | "warning" = "error", code = "assertion-failed") => ({
  code,
  severity,
  file,
  line,
  details: {},
});
const ref = (file: string, line: number) => ({ message: `${file}:${line}: x`, first: { file, line } });

test("compareFirstError passes on the same basename and line, fails on another line or no error", () => {
  expect(compareFirstError(ref("a.beancount", 5), [finding("/x/a.beancount", 5)], []).status).toBe("PASS");
  expect(compareFirstError(ref("a.beancount", 5), [finding("/x/a.beancount", 6)], []).status).toBe("FAIL");
  expect(compareFirstError(ref("a.beancount", 5), [finding("/x/b.beancount", 5)], []).status).toBe("FAIL");
  expect(compareFirstError(ref("a.beancount", 5), [], []).status).toBe("FAIL");
  // Only our first error counts.
  expect(compareFirstError(ref("a.beancount", 5), [finding("a.beancount", 6), finding("a.beancount", 5)], []).status).toBe("FAIL");
});

test("compareFirstError skips only a missing document that we read and warned about", () => {
  const warning = { ...finding("a.beancount", 2, "warning", "ignored-syntax"), details: { construct: "document" } };
  const missing = { message: 'a.beancount:2: File does not exist: "/x.pdf"', first: { file: "a.beancount", line: 2 } };
  expect(compareFirstError(missing, [], [warning]).status).toBe("SKIP");
  expect(compareFirstError(ref("a.beancount", 2), [], [warning]).status).toBe("FAIL");
});

test("listsComparable diffs bean-query's lists only for booked files whose rejected transactions it leaves out too", () => {
  const f = (code: string, reason?: string) => ({ ...finding("a.beancount", 3, "error", code), details: { reason } });
  expect(listsComparable([f("unused-pad"), f("assertion-failed"), f("unbalanced-transaction")])).toBe(true);
  expect(listsComparable([f("booking-error", "ambiguous-auto-posting")])).toBe(true);
  expect(listsComparable([f("booking-error", "ambiguous-lot-match")])).toBe(false);
  expect(listsComparable([f("account-not-open"), f("parse-error")])).toBe(false);
  for (const code of ["invalid-account-name", "unbalanced-push", "duplicate-metadata"]) expect(listsComparable([f(code)])).toBe(false);
});

test("firstHledgerError reads the file and line of a parse error or a failed assertion", () => {
  const stderr = "hledger: Error: /tmp/x/a b.journal:6:12:\n  | 2026-01-02 t\n6 |     a  5 EUR = 99 EUR\n";
  expect(firstHledgerError(stderr)).toEqual({ file: "a b.journal", line: 6 });
});

test("firstHledgerError takes the first line of a range for a transaction that does not balance", () => {
  expect(firstHledgerError("hledger: Error: /tmp/x/b.journal:5-7:\n5 | 2026-01-02 t\n")).toEqual({ file: "b.journal", line: 5 });
  // A conversion posting with a cost: the range has no last colon.
  expect(firstHledgerError("hledger: Error: /tmp/x/c.journal:2-4\nConversion postings must not have a cost:\n")).toEqual({
    file: "c.journal",
    line: 2,
  });
});

test("firstHledgerError gives null for an error that names no position", () => {
  expect(firstHledgerError("hledger: Error: Balance assignments and custom posting dates may not be combined.\n\n2026-01-01 t\n")).toBeNull();
  expect(firstHledgerError("")).toBeNull();
});

const failure = (file: string, line: number, code = "assertion-failed") =>
  ({ code, severity: "error", file, line, details: {} }) as Parameters<typeof compareRejection>[1][number];

test("compareRejection passes on the same basename and line, and fails on another line or file", () => {
  const ref = { message: "hledger: Error: /a/x.journal:6:12:", first: { file: "x.journal", line: 6 } };
  expect(compareRejection(ref, [failure("/other/dir/x.journal", 6)]).status).toBe("PASS");
  expect(compareRejection(ref, [failure("x.journal", 2)]).status).toBe("FAIL");
  expect(compareRejection(ref, [failure("y.journal", 6)]).status).toBe("FAIL");
  // Only the first error counts: hledger stops there.
  expect(compareRejection(ref, [failure("x.journal", 2), failure("x.journal", 6)]).status).toBe("FAIL");
});

test("compareRejection fails when we report no error, skips an error we cannot read yet, and matches 'both fail' without a position", () => {
  const ref = { message: "boom", first: { file: "x.journal", line: 1 } };
  expect(compareRejection(ref, []).status).toBe("FAIL");
  expect(compareRejection(ref, [failure("x.journal", 1, "unsupported-syntax")]).status).toBe("SKIP");
  expect(compareRejection({ message: "boom", first: null }, [failure("x.journal", 3)]).status).toBe("PASS");
});

test("withoutTrailingZeros keeps the value and drops only decimal zeros", () => {
  expect(withoutTrailingZeros("1.00")).toBe("1");
  expect(withoutTrailingZeros("-0.50")).toBe("-0.5");
  expect(withoutTrailingZeros("100")).toBe("100");
  expect(withoutTrailingZeros("0.00")).toBe("0");
});

test("priceLines reads the date, the commodity priced and the commodity of the price", () => {
  const text = [
    "P 2026-01-05 X 2 EUR",
    "P 2026/1/6 10:30 $ 1,000.50 USD",
    'P 2026.01.07 "A B" 3 "C D"',
    "P 2026-01-08 EUR 0.5 €",
    "P 2026-01-09 Y 2",
    "P 2026-02-30 X 2 EUR", // not a date
    "P 01-05 X 2 EUR", // no year
    "; P 2026-01-05 X 2 EUR",
  ].join("\n");
  expect(priceLines([text])).toEqual([
    { date: "2026-01-05", from: "X", to: "EUR" },
    { date: "2026-01-06", from: "$", to: "USD" },
    { date: "2026-01-07", from: "A B", to: "C D" },
    { date: "2026-01-08", from: "EUR", to: "€" },
    { date: "2026-01-09", from: "Y", to: "" },
  ]);
});

test("valueQueries asks the day before the first price, each price date and 30 days after the last, in four commodities", () => {
  const queries = valueQueries(["P 2026-01-05 X 2 GBP\nP 2026-01-09 Y 3 EUR\n"]);
  expect([...new Set(queries.map((q) => q.date))]).toEqual(["2026-01-04", "2026-01-05", "2026-01-09", "2026-02-08"]);
  expect([...new Set(queries.map((q) => q.commodity))]).toEqual(["EUR", "USD", "GBP", "X"]);
  expect(queries).toHaveLength(16);
});

test("valueQueries spreads six dates over many price dates, crosses a year and a leap day, and asks nothing without a price", () => {
  const lines = Array.from({ length: 10 }, (_, i) => `P 2026-01-${String(i + 1).padStart(2, "0")} X 2 EUR`);
  const dates = [...new Set(valueQueries([lines.join("\n")]).map((q) => q.date))];
  expect(dates).toEqual(["2025-12-31", "2026-01-01", "2026-01-02", "2026-01-04", "2026-01-06", "2026-01-08", "2026-01-10", "2026-02-09"]);
  const leap = [...new Set(valueQueries(["P 2024-02-29 X 2 EUR"]).map((q) => q.date))];
  expect(leap).toEqual(["2024-02-28", "2024-02-29", "2024-03-30"]);
  expect(valueQueries(["2026-01-05 t\n    a  1 X\n    b\n"])).toEqual([]);
});

const pass = { status: "PASS", note: "" } as const;
test("summarize shows the worst verdict and says whether it came from --cost or --value", () => {
  const all: Comparison = {
    raw: pass,
    cost: pass,
    values: [{ query: { date: "2026-01-01", commodity: "EUR" }, verdict: pass }],
    postings: pass,
    prices: pass,
  };
  expect(summarize(all)).toEqual(pass);
  const skipped: Comparison = { ...all, cost: { status: "SKIP", note: "unsupported: cost-conversion-postings" } };
  expect(summarize(skipped)).toEqual({ status: "SKIP", note: "cost: unsupported: cost-conversion-postings" });
  const failed: Comparison = {
    ...skipped,
    values: [{ query: { date: "2026-01-01", commodity: "EUR" }, verdict: { status: "FAIL", note: "[0].x: hledger=1 ours=2" } }],
  };
  expect(summarize(failed)).toEqual({ status: "FAIL", note: "value=2026-01-01,EUR: [0].x: hledger=1 ours=2" });
  expect(summarize({ raw: { status: "FAIL", note: "raw difference" }, cost: null, values: [], postings: null, prices: null })).toEqual({
    status: "FAIL",
    note: "raw difference",
  });
  expect(summarize({ ...all, postings: { status: "FAIL", note: "[1].number: hledger=\"5.00\" ours=\"5\"" } })).toEqual({
    status: "FAIL",
    note: 'postings: [1].number: hledger="5.00" ours="5"',
  });
  expect(summarize({ ...all, prices: { status: "FAIL", note: "length hledger=2 ours=1" } })).toEqual({
    status: "FAIL",
    note: "prices: length hledger=2 ours=1",
  });
});

// These two run hledger (on PATH, like the rest of the harness) on probes of the spec.
const probe = (name: string) => join(import.meta.dir, "../spec/hledger/probes", `${name}.journal`);
test("compareCost and compareValue pass on probes, and a refusal of ours is a SKIP", async () => {
  expect((await compareCost([probe("cost-lots")])).status).toBe("PASS");
  expect((await compareValue([probe("value-p-latest")], { date: "2026-01-05", commodity: "EUR" })).status).toBe("PASS");
  expect(await compareCost([probe("conversion-several")])).toEqual({ status: "SKIP", note: "unsupported: cost-conversion-postings" });
});

// ───────────── postings and prices ─────────────
test("csvAccount reads the kind from the brackets and strips one pair", () => {
  expect(csvAccount("a:b")).toEqual({ account: "a:b", kind: "regular" });
  expect(csvAccount("(v)")).toEqual({ account: "v", kind: "virtual" });
  expect(csvAccount("[bv]")).toEqual({ account: "bv", kind: "balanced-virtual" });
  expect(csvAccount("(((g)))")).toEqual({ account: "((g))", kind: "virtual" });
  expect(csvAccount("()")).toEqual({ account: "", kind: "virtual" });
  // brackets that do not match at both ends make a regular name
  expect(csvAccount("(a) b")).toEqual({ account: "(a) b", kind: "regular" });
  expect(csvAccount("a (b)")).toEqual({ account: "a (b)", kind: "regular" });
  expect(csvAccount("[x)")).toEqual({ account: "[x)", kind: "regular" });
});

test("givenName finds the name we were given: the same absolute path, or a relative one at the end of the resolved path", () => {
  expect(givenName("/tmp/a.journal", ["/tmp/a.journal", "/tmp/b.journal"])).toBe("/tmp/a.journal");
  expect(givenName("/work/dir/./a.journal", ["./a.journal"])).toBe("./a.journal");
  expect(givenName("/work/dir/../x/a.journal", ["../x/a.journal", "a.journal"])).toBe("../x/a.journal");
  expect(() => givenName("/other/c.journal", ["a.journal"])).toThrow("none of");
});

const HEAD = "txnidx,date,date2,status,code,description,comment,account,amount,commodity,credit,debit,posting-status,posting-comment";
const row = (idx: number, date: string, status: string, description: string, comment: string, account: string, amount: string, commodity: string) =>
  [idx, date, "", status, "", description, comment, account, amount, commodity, "", "", "", ""].map((f) => (/[",\n]/.test(String(f)) ? `"${String(f).replaceAll('"', '""')}"` : f)).join(",");

test("decodeHledgerPostings takes the location from the last line of a multi-line comment and resolves the file", () => {
  const csv = [
    HEAD,
    row(1, "2026-01-05", "*", "Shop | a, b", 'note\nlocation: fake\nlocation: /w/a.journal:3-8', "expenses:food", "10.50", "EUR"),
    row(1, "2026-01-05", "*", "Shop | a, b", 'note\nlocation: fake\nlocation: /w/a.journal:3-8', "(v)", "1,5", ""),
    row(3, "2026-01-06", "!", "x", "location: /w/sp ace:d-1/a b:3-4.journal:10-12", "[bv]", "-2", "A B"),
  ].join("\n");
  const rows = decodeHledgerPostings(csv, ["/w/a.journal", "/w/sp ace:d-1/a b:3-4.journal"]);
  expect(rows).toEqual([
    { txn: 1, date: "2026-01-05", status: "*", description: "Shop | a, b", account: "expenses:food", kind: "regular", number: "10.50", commodity: "EUR", file: "/w/a.journal", first: 3, last: 8 },
    { txn: 1, date: "2026-01-05", status: "*", description: "Shop | a, b", account: "v", kind: "virtual", number: "1.5", commodity: "", file: "/w/a.journal", first: 3, last: 8 },
    { txn: 3, date: "2026-01-06", status: "!", description: "x", account: "bv", kind: "balanced-virtual", number: "-2", commodity: "A B", file: "/w/sp ace:d-1/a b:3-4.journal", first: 10, last: 12 },
  ]);
});

test("diffPostings compares every field exactly and checks that the line is inside the transaction and never goes back", () => {
  const theirs = decodeHledgerPostings(
    [HEAD, row(1, "2026-01-05", "*", "x", "location: /w/a.journal:3-8", "a", "10.50", "EUR"), row(1, "2026-01-05", "*", "x", "location: /w/a.journal:3-8", "b", "-10.50", "EUR")].join("\n"),
    ["/w/a.journal"],
  );
  const mine = (over: Record<string, unknown> = {}, over2: Record<string, unknown> = {}) => [
    { date: "2026-01-05", flag: "*", narration: "x", account: "a", kind: "regular", number: "10.50", currency: "EUR", file: "/w/a.journal", line: 5, ...over },
    { date: "2026-01-05", flag: "*", narration: "x", account: "b", kind: "regular", number: "-10.50", currency: "EUR", file: "/w/a.journal", line: 6, ...over2 },
  ];
  expect(diffPostings(theirs, mine())).toBeNull();
  expect(diffPostings(theirs, mine({ number: "10.5" }))).toBe('[0].number: hledger="10.50" ours="10.5"');
  expect(diffPostings(theirs, mine({}, { kind: "virtual" }))).toBe('[1].kind: hledger="regular" ours="virtual"');
  expect(diffPostings(theirs, mine({ file: "/w/b.journal" }))).toContain("[0].file");
  expect(diffPostings(theirs, mine({ flag: "" }))).toContain("[0].flag");
  expect(diffPostings(theirs, mine({ payee: "p" }))).toContain("[0].payee");
  expect(diffPostings(theirs, mine({ line: 3 }))).toBe("[0].line: 3 is outside 4..=8 (x)");
  expect(diffPostings(theirs, mine({}, { line: 9 }))).toBe("[1].line: 9 is outside 4..=8 (x)");
  expect(diffPostings(theirs, mine({ line: 7 }, { line: 6 }))).toBe("[1].line: 6 goes back inside a transaction (after 7)");
  expect(diffPostings(theirs, mine().slice(0, 1))).toBe("length hledger=2 ours=1");
});

test("rateCommodity reads the commodity of a first-run row whichever side it is on, quotes kept", () => {
  expect(rateCommodity("P 2026-01-01 USD 0.8 EUR")).toBe("EUR");
  expect(rateCommodity("P 2026-01-01 USD $0.8")).toBe("$");
  expect(rateCommodity("P 2026-01-01 USD EUR 0,8")).toBe("EUR");
  expect(rateCommodity("P 2026-01-01 USD -3 EUR")).toBe("EUR");
  expect(rateCommodity('P 2026-01-05 "ACME INC" 12.5 "Big Bank Ltd"')).toBe('"Big Bank Ltd"');
  expect(rateCommodity("P 2026-01-17  3 EUR")).toBe("EUR");
  expect(rateCommodity("P 2026-01-08 USD 3")).toBe("");
  expect(rateCommodity("P 2026-01-08 USD 1.234,5")).toBe("");
});

test("decodePrices reads quoted names, an empty first commodity, a bare rate and a negative rate", () => {
  const text = [
    'P 2026-01-05 "ACME INC" 12.5 "Big Bank Ltd"',
    "P 2026-01-17  3 EUR",
    "P 2026-01-08 USD 3",
    "P 2026-01-19 USD -3 EUR",
    "P 2026-01-01 USD 0.123456789012345678901234 €",
    "",
  ].join("\n");
  expect(decodePrices(text)).toEqual([
    { date: "2026-01-05", commodity: "ACME INC", rate: { commodity: "Big Bank Ltd", quantity: "12.5" } },
    { date: "2026-01-17", commodity: "", rate: { commodity: "EUR", quantity: "3" } },
    { date: "2026-01-08", commodity: "USD", rate: { commodity: "", quantity: "3" } },
    { date: "2026-01-19", commodity: "USD", rate: { commodity: "EUR", quantity: "-3" } },
    { date: "2026-01-01", commodity: "USD", rate: { commodity: "€", quantity: "0.123456789012345678901234" } },
  ]);
  expect(() => decodePrices("not a row")).toThrow("not a P row");
});

test("diffPrices compares the rate text exactly, in order, and checks that the file is ours and the line holds a P", () => {
  const theirs = decodePrices("P 2026-01-01 USD 0.80 EUR\nP 2026-01-02 USD 7 EUR\n");
  const texts = ["P 2026-01-01 USD 0.80 EUR\n\nP 2026-01-02 USD 7 EUR\n"];
  const mine = (over: Record<string, unknown> = {}) => [
    { date: "2026-01-01", commodity: "USD", rate: { commodity: "EUR", quantity: "0.80" }, file: "/w/a.journal", line: 1 },
    { date: "2026-01-02", commodity: "USD", rate: { commodity: "EUR", quantity: "7" }, file: "/w/a.journal", line: 3, ...over },
  ];
  expect(diffPrices(theirs, mine(), ["/w/a.journal"], texts)).toBeNull();
  expect(diffPrices(theirs, [{ ...mine()[0], rate: { commodity: "EUR", quantity: "0.8" } } as never, mine()[1] as never], ["/w/a.journal"], texts)).toBe(
    '[0].rate.quantity: hledger="0.80" ours="0.8"',
  );
  expect(diffPrices(theirs, mine({ commodity: "GBP" }) as never, ["/w/a.journal"], texts)).toContain("[1].commodity");
  expect(diffPrices(theirs, mine({ line: 2 }) as never, ["/w/a.journal"], texts)).toBe("[1].line: line 2 of a.journal is not a P line");
  expect(diffPrices(theirs, mine({ file: "/w/zzz" }) as never, ["/w/a.journal"], texts)).toBe("[1].file: /w/zzz is none of the files given");
  expect(diffPrices(theirs, mine().slice(0, 1) as never, ["/w/a.journal"], texts)).toBe("length hledger=2 ours=1");
});

test("compareAll lists postings and prices on probes, and leaves them out of a case that is not compared", async () => {
  for (const name of ["post-basic", "post-elided", "post-elided-places", "post-virtual-nested", "post-default-commodity", "prices-order-a", "prices-rate-forms", "prices-names"]) {
    const c = await compareAll([probe(name)]);
    expect([name, c.raw.status, c.postings, c.prices]).toEqual([name, "PASS", pass, pass]);
  }
  const pair = await compareAll([probe("post-files-b"), probe("post-files-a"), probe("post-files-b")]);
  expect([pair.postings, pair.prices]).toEqual([pass, pass]);
  // ledgermill refuses a rate with a price: SKIP, and nothing else is asked
  const refused = await compareAll([join(import.meta.dir, "../spec/hledger/probes/refused/prices-rate-annotations.journal")]);
  expect(refused.raw.status).toBe("SKIP");
  expect([refused.postings, refused.prices]).toEqual([null, null]);
}, 60_000);
