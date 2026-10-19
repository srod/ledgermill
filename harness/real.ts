// bun harness/real.ts [--quiet] JOURNAL...
// Same comparison for journals given on the command line (private journals stay outside the repo;
// never commit them). With several files: each alone, then all together in the order given.
import { resolve } from "node:path";
import { runCases } from "./lib.ts";

const args = process.argv.slice(2);
const files = args.filter((a) => !a.startsWith("--")).map((f) => resolve(f));
if (files.length === 0) {
  console.error("usage: bun harness/real.ts [--quiet] JOURNAL...");
  process.exit(2);
}
const sets = files.length > 1 ? [...files.map((f) => [f]), files] : [files];
await runCases(
  sets.map((paths) => ({ name: paths.join(" + "), paths })),
  args.includes("--quiet"),
);
