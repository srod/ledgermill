// bun test harness/interface.test.ts (needs the wasm built into pkg/)
// The release gate asks for every method of the interface to be compared with the reference tools. This fails when the
// wasm exports a function or a `Ledger` method that the comparison in lib.ts never calls.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const dts = readFileSync(join(root, "pkg/ledgermill_wasm.d.ts"), "utf8");
const lib = readFileSync(join(root, "harness/lib.ts"), "utf8");

// Not questions about a ledger: memory and start-up.
const notCompared = new Set(["free", "initSync", "default", "constructor"]);

function exported(): { functions: string[]; methods: string[] } {
  const functions = [...dts.matchAll(/^export function (\w+)\(/gm)].map((m) => m[1] ?? "");
  const body = dts.match(/^export class Ledger \{([\s\S]*?)^\}/m)?.[1] ?? "";
  const methods = [...body.matchAll(/^ {4}(\w+)\(/gm)].map((m) => m[1] ?? "");
  return {
    functions: functions.filter((name) => !notCompared.has(name)),
    methods: methods.filter((name) => !notCompared.has(name)),
  };
}

test("the comparison calls every exported function and Ledger method", () => {
  const { functions, methods } = exported();
  // A d.ts this reader no longer understands would list nothing and pass.
  expect(functions).toContain("load");
  expect(methods).toContain("transactions");
  const missing = [
    ...functions.filter((name) => !lib.includes(`wasm.${name}(`)),
    ...methods.filter((name) => !lib.includes(`ledger.${name}(`)),
  ];
  expect(missing).toEqual([]);
});
