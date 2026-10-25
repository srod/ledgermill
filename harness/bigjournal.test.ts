// bun test harness/bigjournal.test.ts (needs the wasm built into pkg/, like the harness itself)
import { expect, test } from "bun:test";
import { beancountLedger, hledgerJournal } from "./bigjournal.ts";
import { calls, formats, importBuild, load, makeCase, PKG, start } from "./engine.ts";

test("the generators give the same text for the same seed, another for another seed", () => {
  expect(hledgerJournal(200, 7)).toBe(hledgerJournal(200, 7));
  expect(beancountLedger(200, 7)).toBe(beancountLedger(200, 7));
  expect(hledgerJournal(200, 7)).not.toBe(hledgerJournal(200, 8));
  expect(beancountLedger(200, 7)).not.toBe(beancountLedger(200, 8));
});

test("ledgermill reads exactly N transactions and no error, in both syntaxes", async () => {
  const build = await importBuild(PKG, "bigjournal-test");
  start(build);
  for (const n of [0, 1, 2, 500]) {
    for (const format of formats) {
      const c = makeCase(format.file, format.generate(n, 3));
      const ledger = load(build.api, c);
      try {
        expect((JSON.parse(calls.transactions(ledger)) as unknown[]).length).toBe(n);
        const errors = (JSON.parse(calls.check(ledger)) as { severity: string }[]).filter((f) => f.severity === "error");
        expect(errors).toEqual([]);
      } finally {
        ledger.free();
      }
    }
  }
});

test("a journal has several commodities, prices, costs and balance checks", () => {
  const h = hledgerJournal(2000, 1);
  expect(h).toMatch(/^P \d{4}-\d\d-\d\d USD /m);
  expect(h).toMatch(/ @ \d/);
  expect(h).toMatch(/ \{\d+\.\d\d EUR\}/);
  expect(h).toMatch(/ = \d+\.\d\d EUR/);
  const b = beancountLedger(2000, 1);
  expect(b).toMatch(/^\d{4}-\d\d-\d\d price AAPL /m);
  expect(b).toMatch(/ \{\} @ /);
  expect(b).toMatch(/^\d{4}-\d\d-\d\d balance /m);
  expect(b.match(/^\d{4}-\d\d-\d\d open /gm)?.length).toBeGreaterThanOrEqual(30);
});
