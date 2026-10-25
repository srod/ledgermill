import { expect, test } from "bun:test";
import { FORMATS, type Format, line, overall, RUNS, rows, type Run, TARGETS } from "./report.ts";

const atTarget: Run = { ...TARGETS };
const samples = (f: (format: Format, run: number) => Run): Record<Format, Run[]> => ({
  hledger: Array.from({ length: RUNS }, (_, i) => f("hledger", i)),
  beancount: Array.from({ length: RUNS }, (_, i) => f("beancount", i)),
});

test("a result exactly at every target is met", () => {
  expect(overall(rows(samples(() => atTarget)))).toBe("device targets: met");
});

test("the worst of the three runs decides, and the verdict names what missed", () => {
  const rs = rows(samples((format, run) => (format === "beancount" && run === 2 ? { ...atTarget, check: 500.1, memory: 50_000_001 } : atTarget)));
  expect(overall(rs)).toBe("device targets: not met (beancount check, beancount memory)");
  expect(rs.find((r) => r.format === "beancount" && r.metric === "check")?.worst).toBe(500.1);
});

test("a bench with a missing run is not met", () => {
  const partial = { hledger: [atTarget], beancount: [] as Run[] };
  expect(rows(partial).every((r) => !r.met)).toBe(true);
});

test("lines show the runs, the worst, the target and the verdict", () => {
  const rs = rows(samples((_, run) => ({ ...atTarget, start: 100 + run, memory: 31_500_000 })));
  expect(line(rs[0] as (typeof rs)[number])).toBe("hledger start: 100.0 / 101.0 / 102.0 ms, worst 102.0, target ≤ 300.0 ms: met");
  expect(line(rs.find((r) => r.metric === "memory") as (typeof rs)[number])).toBe("hledger memory: 31.5 / 31.5 / 31.5 MB, worst 31.5, target ≤ 50.0 MB: met");
  expect(rs.length).toBe(FORMATS.length * 6);
});
