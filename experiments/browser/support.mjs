import { spawn, spawnSync } from "node:child_process";
import { mkdtemp, mkdir, writeFile, rm, cp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { once } from "node:events";
import { chromium } from "playwright";

export function buildHost() {
  const result = spawnSync("cargo", ["build", "--release", "--offline", "-p", "vizzi"], { stdio: "inherit" });
  if (result.status !== 0) throw new Error("Cannot build the native browser host");
}

export async function workspace({ files, source, name = "source" } = {}) {
  const temporary = await mkdtemp(join(tmpdir(), "vizzi-browser-"));
  const root = join(temporary, name), cache = join(temporary, "cache");
  if (source) await cp(resolve(source), root, { recursive: true, dereference: false });
  else await mkdir(root);
  for (const [path, text] of Object.entries(files ?? {})) {
    const target = join(root, path);
    await mkdir(resolve(target, ".."), { recursive: true });
    await writeFile(target, text);
  }
  return { root, cache, close: () => rm(temporary, { recursive: true, force: true }) };
}

export async function startHost(input, workers = 4) {
  const child = spawn(resolve("target/release/vizzi"), ["--root", input.root, "--cache", input.cache, "--port", "0", "--workers", String(workers)], { stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "";
  const url = await new Promise((accept, reject) => {
    const timeout = setTimeout(() => { child.kill("SIGKILL"); reject(new Error(`Host startup timed out: ${stderr}`)); }, 30000);
    const fail = error => { clearTimeout(timeout); reject(error); };
    child.once("error", fail);
    child.once("exit", code => fail(new Error(`Host exited (${code}): ${stderr}`)));
    child.stderr.on("data", chunk => {
      stderr = (stderr + chunk).slice(-12000);
      const match = stderr.match(/Vizzi: (http:\/\/127\.0\.0\.1:\d+)/);
      if (match) { clearTimeout(timeout); accept(match[1]); }
    });
  });
  return {
    url, pid: child.pid,
    async close() {
      if (child.exitCode !== null) return;
      const exited = once(child, "exit");
      const timeout = setTimeout(() => child.kill("SIGKILL"), 5000);
      child.kill("SIGINT");
      try { await exited; } finally { clearTimeout(timeout); }
    },
  };
}

export async function browser() {
  return chromium.launch({ headless: !process.argv.includes("--headed") });
}

export async function openWorkspace(browser, host, viewport = { width: 1440, height: 900 }) {
  const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.route("**/*", route => {
    if (new URL(route.request().url()).origin === host.url) return route.continue();
    return route.abort();
  });
  await page.goto(host.url);
  await page.waitForFunction(() => document.getElementById("snapshot").textContent.length === 8);
  return { page, errors, close: () => context.close() };
}

export async function paint(page) {
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}
