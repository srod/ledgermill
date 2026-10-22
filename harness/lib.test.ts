// bun test harness/lib.test.ts (needs the wasm built into pkg/, like the harness itself)
import { expect, test } from "bun:test";
import { join } from "node:path";
import {
  type Comparison,
  compareCost,
  compareFirstError,
  compareRejection,
  compareValue,
  firstHledgerError,
  decimalString,
  decodeCsv,
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
    { date: "2026-01-04", flag: "*", narration: "pay", account: "Assets:A", number: "8.50", currency: "EUR", file: "/a/b.beancount", line: 28 },
    { date: "2026-01-05", flag: "!", payee: "Shop, Ltd", narration: 'say "hi"', account: "Assets:B", number: "-1", currency: "BTC", file: "/a/b.beancount", line: 36 },
    { date: "2026-01-05", flag: "!", payee: "Shop, Ltd", narration: 'say "hi"', account: "Assets:B", number: "-2", currency: "BTC", file: "/a/b.beancount", line: 36 },
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
  const all: Comparison = { raw: pass, cost: pass, values: [{ query: { date: "2026-01-01", commodity: "EUR" }, verdict: pass }] };
  expect(summarize(all)).toEqual(pass);
  const skipped: Comparison = { ...all, cost: { status: "SKIP", note: "unsupported: cost-conversion-postings" } };
  expect(summarize(skipped)).toEqual({ status: "SKIP", note: "cost: unsupported: cost-conversion-postings" });
  const failed: Comparison = {
    ...skipped,
    values: [{ query: { date: "2026-01-01", commodity: "EUR" }, verdict: { status: "FAIL", note: "[0].x: hledger=1 ours=2" } }],
  };
  expect(summarize(failed)).toEqual({ status: "FAIL", note: "value=2026-01-01,EUR: [0].x: hledger=1 ours=2" });
  expect(summarize({ raw: { status: "FAIL", note: "raw difference" }, cost: null, values: [] })).toEqual({ status: "FAIL", note: "raw difference" });
});

// These two run hledger (on PATH, like the rest of the harness) on probes of the spec.
const probe = (name: string) => join(import.meta.dir, "../spec/hledger/probes", `${name}.journal`);
test("compareCost and compareValue pass on probes, and a refusal of ours is a SKIP", async () => {
  expect((await compareCost([probe("cost-lots")])).status).toBe("PASS");
  expect((await compareValue([probe("value-p-latest")], { date: "2026-01-05", commodity: "EUR" })).status).toBe("PASS");
  expect(await compareCost([probe("conversion-several")])).toEqual({ status: "SKIP", note: "unsupported: cost-conversion-postings" });
});
