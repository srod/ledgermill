// The targets and the verdict of the device bench, shared by the page (page.ts) and report.test.ts. Pure: no DOM.

export const FORMATS = ["hledger", "beancount"] as const;
export type Format = (typeof FORMATS)[number];

/** Runs per format. Each run is a new worker, so each starts cold. */
export const RUNS = 3;

/** What one run measured: milliseconds, except `memory` (bytes of the wasm memory after the run). */
export type Run = {
  start: number;
  "load+transactions": number;
  check: number;
  value: number;
  holdings: number;
  memory: number;
};
export type Metric = keyof Run;
export const METRICS: readonly Metric[] = ["start", "load+transactions", "check", "value", "holdings", "memory"];

/** The ceiling of each metric, 10,000 transactions on the phone: ms, and bytes for memory (50 MB = 50,000,000, the stricter reading of "MB"). */
export const TARGETS: Record<Metric, number> = {
  start: 300,
  "load+transactions": 500,
  check: 500,
  value: 200,
  holdings: 200,
  memory: 50_000_000,
};

export type Row = { format: Format; metric: Metric; runs: number[]; worst: number; target: number; met: boolean };

/** One row per format and metric over that format's runs. A format with fewer than RUNS runs is not met: the bench did not finish. */
export function rows(samples: Record<Format, Run[]>): Row[] {
  return FORMATS.flatMap((format) =>
    METRICS.map((metric) => {
      const runs = samples[format].map((run) => run[metric]);
      const worst = Math.max(...runs);
      const target = TARGETS[metric];
      return { format, metric, runs, worst, target, met: runs.length === RUNS && worst <= target };
    }),
  );
}

const show = (metric: Metric, x: number) => (metric === "memory" ? (x / 1e6).toFixed(1) : x.toFixed(1));
const unit = (metric: Metric) => (metric === "memory" ? "MB" : "ms");

/** `hledger check: 120.3 / 118.9 / 121.0 ms, worst 121.0, target ≤ 500 ms: met` */
export function line(r: Row): string {
  const u = unit(r.metric);
  return `${r.format} ${r.metric}: ${r.runs.map((x) => show(r.metric, x)).join(" / ")} ${u}, worst ${show(r.metric, r.worst)}, target ≤ ${show(r.metric, r.target)} ${u}: ${r.met ? "met" : "NOT MET"}`;
}

/** `device targets: met`, or `not met (hledger check, beancount memory)`. */
export function overall(rs: Row[]): string {
  const missed = rs.filter((r) => !r.met).map((r) => `${r.format} ${r.metric}`);
  return missed.length === 0 ? "device targets: met" : `device targets: not met (${missed.join(", ")})`;
}
