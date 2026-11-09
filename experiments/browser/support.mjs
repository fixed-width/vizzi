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

export async function workspace({ files, source, name = "source", sourceArguments = [] } = {}) {
  const temporary = await mkdtemp(join(tmpdir(), "vizzi-browser-"));
  const root = join(temporary, name), cache = join(temporary, "cache");
  if (source) await cp(resolve(source), root, { recursive: true, dereference: false });
  else await mkdir(root);
  for (const [path, text] of Object.entries(files ?? {})) {
    const target = join(root, path);
    await mkdir(resolve(target, ".."), { recursive: true });
    await writeFile(target, text);
  }
  return { root, cache, sourceArguments, close: () => rm(temporary, { recursive: true, force: true }) };
}

export async function startHost(input, workers = 4, port = 0) {
  const child = spawn(resolve(input.executable ?? "target/release/vizzi"), ["--root", input.root, "--cache", input.cache, "--port", String(port), "--workers", String(workers), ...(input.sourceArguments ?? [])], { stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "";
  const url = await new Promise((accept, reject) => {
    const timeout = setTimeout(() => { child.kill("SIGKILL"); reject(new Error(`Host startup timed out: ${stderr}`)); }, input.startupTimeoutMs ?? 30000);
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
      const timeout = setTimeout(() => child.kill("SIGKILL"), input.stopTimeoutMs ?? 5000);
      child.kill("SIGINT");
      try {
        const [code, signal] = await exited;
        if (code !== 0 || signal !== null) throw new Error(`Host did not stop gracefully: ${code}/${signal}`);
      } finally { clearTimeout(timeout); }
    },
  };
}

export async function browser({ desktopBus = true } = {}) {
  return chromium.launch({ headless: !process.argv.includes("--headed"),
    env: desktopBus ? undefined : { ...process.env, DBUS_SESSION_BUS_ADDRESS: "unix:path=/dev/null" } });
}

export async function openWorkspace(browser, host, viewport = { width: 1440, height: 900 }) {
  const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const errors = [], requests = [];
  page.on("request", request => { const url = new URL(request.url()); if (url.pathname.startsWith("/api/")) requests.push(url.pathname); });
  page.on("pageerror", error => errors.push(error.message));
  await page.route("**/*", route => {
    if (new URL(route.request().url()).origin === host.url) return route.continue();
    return route.abort();
  });
  await page.goto(host.url+"/explore");
  await page.bringToFront();
  await page.waitForFunction(() => document.getElementById("snapshot").textContent.length === 8);
  return { page, errors, requests, close: () => context.close() };
}

export async function paint(page) {
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}
