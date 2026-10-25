// bun harness/device-bench/headless.ts
// Checks the bench on this machine: starts server.ts on a spare port (results go to a temp file, not to your
// results.jsonl), opens the page in Playwright's headless Chromium, waits for the verdict and prints it. Exit 0 when
// "device targets: met", 1 otherwise. Needs Chromium for Playwright (`bunx playwright install chromium`); Bun fetches
// the `playwright` module itself, since the repository has no node_modules. Desktop numbers say nothing about a phone.
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { chromium } from "playwright@1.64.1";

const port = 8791;
const tmp = mkdtempSync(join(tmpdir(), "device-bench-"));
const server = Bun.spawn(["bun", join(import.meta.dir, "server.ts"), "--port", String(port), "--results", join(tmp, "results.jsonl")], {
  stdout: "inherit",
  stderr: "inherit",
});
const url = `http://localhost:${port}/`;
let outcome = "no verdict";
try {
  for (let tries = 0; ; tries++) {
    if (await fetch(url).then((r) => r.ok, () => false)) break;
    if (tries > 100 || server.exitCode !== null) throw new Error("the bench server did not start");
    await Bun.sleep(100);
  }
  const browser = await chromium.launch();
  console.log(`Chromium ${browser.version()}`);
  try {
    const page = await browser.newPage();
    await page.goto(url);
    await page.waitForFunction(() => document.getElementById("verdict")?.textContent?.startsWith("device targets:"), undefined, { timeout: 120_000 });
    outcome = (await page.locator("#verdict").textContent()) ?? outcome;
  } finally {
    await browser.close();
  }
} finally {
  server.kill();
  rmSync(tmp, { recursive: true, force: true });
}
console.log(`headless: ${outcome}`);
process.exit(outcome === "device targets: met" ? 0 : 1);
