// bun harness/compare.ts [--quiet] [--only TEXT]
// Runs hledger (or bean-check and bean-query, for .beancount files) and our wasm on every corpus journal
// and multi-file pair and diffs typed values.
import { readdirSync } from "node:fs";
import { basename, join } from "node:path";
import { runCases } from "./lib.ts";

const ROOT = join(import.meta.dir, "..");
const PROTOTYPE = join(import.meta.dir, "journals/prototype");
const APP = join(import.meta.dir, "journals/app");
const SPEC = join(ROOT, "spec/hledger/probes");
const DIRS = [
  ["prototype", PROTOTYPE, ".journal"],
  ["app", APP, ".journal"],
  ["spec", SPEC, ".journal"],
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

// Journals where hledger adds the sub-accounts of an account in an order that follows no rule we
// know (spec/hledger/assignments.md): the values agree, the places of a sum can differ.
const placesMayDiffer = new Set(["assign-inclusive-order-names-x123.journal"]);

// Pairs of spec probes (spec/hledger/assertion-order.md, assertion-failures.md,
// assignment-processing.md): each file has its own running balances, and the first file with an
// error has the first error.
const specPairs: [string, string][] = [
  ["assert-file-a", "assert-file-b"],
  ["assert-file-b", "assert-file-a"],
  ["assert-file-a", "assert-file-c"],
  ["assert-file-c", "assert-file-c"],
  ["assert-file-late-fail", "assert-file-early-fail"],
  ["assert-file-early-fail", "assert-file-late-fail"],
  ["assert-file-late-fail", "assert-file-parse-error"],
  ["assert-file-parse-error", "assert-file-late-fail"],
  ["assert-file-late-fail", "assert-file-unbalanced"],
  ["assert-file-unbalanced", "assert-file-late-fail"],
  ["assert-file-a", "assign-file-b"],
  // Rules (spec/hledger/rules-ignored.md): a file is read to its end before the next starts.
  ["rule-file-a", "rule-file-b"],
  ["rule-file-b", "rule-file-a"],
  ["rule-file-c", "rule-file-a"],
];

// Sets of several ledgers (spec/beancount/several-files.md): each Beancount file is a ledger of its own, all the hledger files together
// are one. Files are loaded in the order listed; the reference runs per ledger and the results are merged (harness/README.md, "Several ledgers").
const SEVERAL = join(ROOT, "spec/beancount/probes/several-files");
const severalSets: string[][] = [
  // Two or three ledgers with an account, a commodity, a price and a day in common, in every position of the hledger file.
  ["a.beancount", "b.beancount"],
  ["b.beancount", "a.beancount"],
  ["a.beancount", "b.beancount", "c.journal"],
  ["b.beancount", "a.beancount", "c.journal"],
  ["c.journal", "a.beancount", "b.beancount"],
  ["c.journal", "b.beancount", "a.beancount"],
  ["a.beancount", "c.journal", "b.beancount"],
  ["b.beancount", "c.journal", "a.beancount"],
  ["a.beancount", "c.journal"],
  ["c.journal", "a.beancount"],
  // Hledger files with a Beancount file between them are still one ledger: the shared day goes c, a, d (and P lines of both).
  ["c.journal", "a.beancount", "d.journal"],
  ["d.journal", "b.beancount", "c.journal", "a.beancount"],
  ["a.beancount", "c.journal", "b.beancount", "d.journal"],
  ["d.journal", "c.journal", "a.beancount"],
  // Valid on their own, each with its own opens (a root renamed by an option is refused: `rename-a` is not here).
  ["open-a.beancount", "open-b.beancount"],
  ["open-a.beancount", "open-b.beancount", "c.journal"],
  // A ledger the reference rejects: the first error is that of the file first in the manifest.
  ["unbalanced.beancount", "unbalanced.journal"],
  ["unbalanced.journal", "unbalanced.beancount"],
  ["unbalanced.journal", "unbalanced-2.journal", "unbalanced.beancount"],
  ["unbalanced.beancount", "unbalanced.journal", "unbalanced-2.journal"],
  ["unbalanced-2.journal", "unbalanced.beancount", "unbalanced.journal"],
  ["unbalanced.beancount", "a.beancount"],
  ["a.beancount", "unbalanced.beancount", "b.beancount"],
  ["a.beancount", "unbalanced.journal"],
  ["unbalanced.journal", "a.beancount", "c.journal"],
  // A file ledgermill refuses next to good ones: the set is refused (a SKIP).
  ["refused.beancount", "a.beancount"],
  ["a.beancount", "c.journal", "refused.beancount"],
].map((files) => files.map((f) => join(SEVERAL, f)));
// Plainledger's own Beancount journals with each other and with its hledger ones (they share Assets:Bank and the like, so rows are added).
const appSets: string[][] = [
  ["proj002-price-unit.beancount", "verbatim-price-annotations.beancount"],
  ["beancount-lot-matrix.beancount", "generated-padding.beancount", "proj008-payee-narration.beancount"],
  ["valuation-dates.beancount", "valuation-fifo-components.beancount", "valuation-costs.beancount"],
  ["valuation-precedence.beancount", "valuation-graph.beancount", "valuation-dates.beancount"],
  ["proj007-flagged-unicode.beancount", "zero-posting.beancount", "proj002-price-unit.beancount"],
  ["valuation-costs.beancount", "valuation-costs.journal"],
  ["valuation-dates.journal", "valuation-dates.beancount"],
  ["valuation-graph.beancount", "valuation-graph.journal"],
  ["valuation-precedence.beancount", "valuation-precedence.journal", "beancount-lot-matrix.beancount"],
  ["proj002-price-unit.beancount", "proj002-price-unit.journal", "verbatim-price-annotations.beancount"],
  ["proj002-price-unit.journal", "proj004-balance-assertion.journal", "proj002-price-unit.beancount"],
  ["valuation-dates.journal", "valuation-costs.beancount", "valuation-precedence.journal", "valuation-dates.beancount"],
].map((files) => files.map((f) => join(APP, f)));
// Every Beancount journal of the app together, with three of its hledger ones in the middle.
const appBeancount = readdirSync(APP).filter((f) => f.endsWith(".beancount")).sort().map((f) => join(APP, f));
const appEverything = [...appBeancount.slice(0, 6), join(APP, "valuation-dates.journal"), join(APP, "proj002-price-unit.journal"), join(APP, "valuation-graph.journal"), ...appBeancount.slice(6)];

// The journals of `app` are Plainledger's own test journals: its holdings reads are compared on them too.
const cases: { name: string; paths: string[]; ignorePlaces?: boolean; plainledger?: boolean; formatOnly?: boolean }[] = [];
for (const [label, dir, extension] of DIRS)
  for (const f of readdirSync(dir).filter((f) => f.endsWith(extension)).sort())
    cases.push({
      name: `${label}/${f}`,
      paths: [join(dir, f)],
      ...(placesMayDiffer.has(f) ? { ignorePlaces: true } : {}),
      ...(dir === APP ? { plainledger: true } : {}),
    });
// The Beancount probes ledgermill refuses or cannot parse are not compared (see the README), but format() must leave them alone
// or still align them like bean-format.
const REFUSED = join(ROOT, "spec/beancount/probes/refused");
for (const f of readdirSync(REFUSED).filter((f) => f.endsWith(".beancount")).sort())
  cases.push({ name: `refused-beancount/${f}`, paths: [join(REFUSED, f)], formatOnly: true });
for (const [dir, list] of [[PROTOTYPE, pairs], [APP, fixturePairs], [SPEC, specPairs]] as const)
  for (const [a, b] of list)
    cases.push({
      name: `pair ${a}+${b}`,
      paths: [join(dir, `${a}.journal`), join(dir, `${b}.journal`)],
      ...(dir === APP ? { plainledger: true } : {}),
    });

for (const paths of [...severalSets, ...appSets, appEverything])
  cases.push({ name: `set ${paths.map((p) => basename(p)).join("+")}`, paths });

// `--only TEXT` runs the cases whose name has TEXT in it (`--only set` for the sets of several ledgers).
const only = process.argv[process.argv.indexOf("--only") + 1];
await runCases(process.argv.includes("--only") ? cases.filter((c) => c.name.includes(only ?? "")) : cases, process.argv.includes("--quiet"));
