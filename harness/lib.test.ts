// bun test harness/lib.test.ts (needs the wasm built into pkg/, like the harness itself)
import { expect, test } from "bun:test";
import { decimalString, firstDiff, projectHledger } from "./lib.ts";

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
