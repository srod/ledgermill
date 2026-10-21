// bun harness/compare.ts [--quiet]
// Runs hledger (or bean-check and bean-query, for .beancount files) and our wasm on every corpus journal
// and multi-file pair and diffs typed values.
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { runCases } from "./lib.ts";

const ROOT = join(import.meta.dir, "..");
const PROTOTYPE = join(import.meta.dir, "journals/prototype");
const APP = join(import.meta.dir, "journals/app");
const DIRS = [
  ["prototype", PROTOTYPE, ".journal"],
  ["app", APP, ".journal"],
  ["spec", join(ROOT, "spec/hledger/probes"), ".journal"],
  ["app", APP, ".beancount"],
  ["spec-beancount", join(ROOT, "spec/beancount/probes"), ".beancount"],
] as const;

// Files loaded together, in this order (names without `.journal`).
const pairs: [string, string][] = [
  ["p4b", "p4a"],
  ["x6a", "x6b"],
  ["x6b", "x6a"],
  ["cd1", "cd2"],
  ["dd1", "dd2"],
  ["dm1", "dm2"],
  ["da", "db"],
  ["f1", "f2"],
  ["u9a", "u9b"],
  ["m1a", "m1b"],
  ["m2a", "m1b"],
  ["m3a", "m3b"],
  ["h1", "h2"],
  ["h3", "h4"],
  ["h5", "h6"],
  ["y8a", "y8b"],
  ["g1", "g2"],
  ["x13", "x13b"],
  ["val-f1", "val-f2"],
  ["val-f2", "val-f1"],
  ["val-f3", "val-f4"],
  ["val-f4", "val-f3"],
  ["val-f5", "val-f6"],
  ["val-f6", "val-f5"],
  ["val-ty9a", "val-ty9b"],
  ["val-ty9b", "val-ty9a"],
];
const fixturePairs: [string, string][] = [
  ["sibling-alias-scope", "sibling-alias-isolation"],
  ["sibling-d-presentation", "sibling-d-no-directive"],
];

const cases: { name: string; paths: string[] }[] = [];
for (const [label, dir, extension] of DIRS)
  for (const f of readdirSync(dir).filter((f) => f.endsWith(extension)).sort())
    cases.push({ name: `${label}/${f}`, paths: [join(dir, f)] });
for (const [dir, list] of [[PROTOTYPE, pairs], [APP, fixturePairs]] as const)
  for (const [a, b] of list)
    cases.push({ name: `pair ${a}+${b}`, paths: [join(dir, `${a}.journal`), join(dir, `${b}.journal`)] });

await runCases(cases, process.argv.includes("--quiet"));
