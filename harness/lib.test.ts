// bun test harness/lib.test.ts (needs the wasm built into pkg/, like the harness itself)
import { expect, test } from "bun:test";
import { decimalString, decodeCsv, firstDiff, projectBeancount, projectHledger } from "./lib.ts";

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
