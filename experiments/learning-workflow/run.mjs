import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, writeFile, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, dirname } from "node:path";
import { buildHost, browser, startHost, paint } from "../browser/support.mjs";

const options = {};
for (let i = 2; i < process.argv.length; i++) {
  const flag = process.argv[i];
  if (flag === "--learning-walkthrough") continue;
  assert.ok(["--source", "--revision", "--output"].includes(flag), `Unknown option ${flag}`);
  options[flag.slice(2)] = process.argv[++i];
}
assert.ok(options.source && options.output && /^[0-9a-f]{40}$/.test(options.revision ?? ""));
const output = resolve(options.output), source = resolve(options.source);
await mkdir(dirname(output), { recursive: true });
await mkdir(output);
const git = (...args) => {
  const result = spawnSync("git", ["-C", source, ...args], { encoding: "utf8", env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" } });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
};
const temporary = await mkdtemp(join(tmpdir(), "vizzi-learning-"));
const input = { root: join(temporary, "lpc-rs"), cache: join(temporary, "cache"), startupTimeoutMs: 300000 };
const report = {
  schema: 1, question: "How does the compiler obtain source, decide whether to retry with the c extension, and report a read failure?",
  operator: "Reference-aware agent; paths, target names and expected behaviour were known before replay. This is guided functional evidence, not a newcomer usability study.",
  options, steps: [], outlineScans: [], bindings: [], pageErrors: [], responseErrors: [],
  sourceWorktreeBefore: git("status", "--porcelain=v1"),
  limitations: ["Navigation uses UI controls and rendered DOM; response metadata is used only to audit snapshot binding/readiness.",
    "Actions are scripted and paced for recording; elapsed times are not human task-completion or performance measurements.",
    "Deliberate search/back probes are capability checks, not spontaneous user errors.",
    "No source is edited and no analysed project tooling is executed."],
};
let host, chromium, context, page, videoPath;
const responseJobs = [];
try {
  const archive = join(temporary, "source.tar");
  git("archive", "--format=tar", "--output", archive, options.revision);
  report.archiveSha256 = createHash("sha256").update(await readFile(archive)).digest("hex");
  await mkdir(input.root);
  const extracted = spawnSync("tar", ["-xf", archive, "-C", input.root], { encoding: "utf8" });
  assert.equal(extracted.status, 0, extracted.stderr);
  await rm(archive);
  buildHost();
  report.binarySha256 = createHash("sha256").update(await readFile("target/release/vizzi")).digest("hex");
  report.vizziRevision = spawnSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).stdout.trim();
  const launch = performance.now();
  host = await startHost(input);
  report.hostReadyMs = performance.now() - launch;
  chromium = await browser({ desktopBus: false });
  report.browser = chromium.version();
  context = await chromium.newContext({ viewport: { width: 1440, height: 1000 }, recordVideo: { dir: join(output, "video"), size: { width: 1440, height: 1000 } } });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: false });
  page = await context.newPage();
  videoPath = await page.video().path();
  page.on("pageerror", error => report.pageErrors.push(error.message));
  page.on("response", response => {
    const url = new URL(response.url());
    if (!["/api/view", "/api/window"].includes(url.pathname) || response.status() !== 200) return;
    responseJobs.push(response.json().then(data => report.bindings.push({
      endpoint: url.pathname, filter: url.searchParams.get("filter"), snapshot: data.snapshot,
      file: data.file ?? data.source?.id, version: data.version ?? data.source?.version,
      selection: data.selection, matching: data.matching,
    })).catch(error => report.responseErrors.push(error.message)));
  });
  await page.route("**/*", route => new URL(route.request().url()).origin === host.url ? route.continue() : route.abort());
  const opened = performance.now();
  await page.goto(host.url);
  await page.waitForFunction(() => document.querySelectorAll(".code-line").length > 0);
  report.pageReadyMs = performance.now() - opened;
  report.hostUrl = host.url;

  const state = () => page.evaluate(() => {
    const source = document.querySelector("#source"), outline = document.querySelector("#outline");
    if (!source) return { url: location.href, outsideWorkspace: true };
    const bounds = source.getBoundingClientRect();
    return { url: location.href, historyLength: history.length,
      filter: document.querySelector("#filter").value,
      file: document.querySelector("#file-title").textContent,
      observation: document.querySelector("#snapshot").title,
      status: document.querySelector("#status").textContent, observationMode: document.querySelector("#status").title,
      counts: document.querySelector("#counts").textContent,
      selectedFile: document.querySelector('#file-list button[aria-current="true"]')?.textContent ?? null,
      selectedDeclaration: outline.querySelector('[aria-current="true"]')?.title ?? null,
      sourceTop: source.scrollTop, outlineTop: outline.scrollTop,
      sourceRows: [...source.querySelectorAll(".code-line")].filter(row => {
        const r = row.getBoundingClientRect(); return r.bottom > bounds.top && r.top < bounds.bottom;
      }).map(row => ({ line: Number(row.querySelector(".line-no").textContent), text: row.querySelector(".source-text").textContent })),
      outlineButtons: outline.querySelectorAll("button").length,
      sourceLinks: source.querySelectorAll("a").length,
      searchInputs: [...document.querySelectorAll('input[type="search"]')].map(input => ({ id: input.id, placeholder: input.placeholder })),
    };
  });
  const checkpoint = async (name, intent) => {
    await paint(page);
    await page.waitForTimeout(350);
    const screenshot = `${String(report.steps.length).padStart(2, "0")}-${name}.png`;
    const step = { name, intent, state: await state(), screenshot };
    report.steps.push(step);
    await page.screenshot({ path: join(output, screenshot), fullPage: true });
    console.log(`${name}: ${step.state.file ?? step.state.url}`);
    return step.state;
  };
  const filter = async text => {
    const response = page.waitForResponse(response => {
      const url = new URL(response.url());
      return url.pathname === "/api/view" && url.searchParams.get("filter") === text && response.status() === 200;
    });
    await page.locator("#filter").fill(text);
    const data = await (await response).json();
    await page.waitForFunction(prefix => document.querySelector("#counts").textContent.startsWith(prefix), `${data.files.length} shown / ${data.matching} matching`);
    await paint(page);
  };
  const chooseFile = async path => {
    if (!await page.locator("details").evaluate(node => node.open)) await page.locator("summary").click();
    await page.locator("#file-list").getByRole("button", { name: path, exact: true }).click();
    await page.waitForFunction(path => document.querySelector("#file-title").textContent === path && document.querySelectorAll(".code-line").length > 0, path);
    await paint(page);
  };
  const visibleOutline = () => page.locator("#outline").evaluate(outline => {
    const bounds = outline.getBoundingClientRect();
    return [...outline.querySelectorAll("button")].filter(button => {
      const r = button.getBoundingClientRect(); return r.top >= bounds.top && r.bottom <= bounds.bottom;
    }).map(button => ({ id: button.dataset.declaration, name: button.querySelector("span").textContent,
      title: button.title, position: Number(button.parentElement.getAttribute("aria-posinset")) }));
  });
  const seek = async (name, fromEnd = false) => {
    const scan = { file: await page.locator("#file-title").textContent(), name, start: fromEnd ? "End" : "Home", pages: [] };
    report.outlineScans.push(scan);
    await page.locator("#outline button").first().focus();
    await page.keyboard.press(fromEnd ? "End" : "Home");
    const positions = new Set();
    for (let attempt = 0; attempt < 100; attempt++) {
      await paint(page);
      await page.waitForFunction(() => {
        const outline = document.querySelector("#outline"), button = outline.querySelector('button[tabindex="0"]');
        if (!button || document.activeElement !== button) return false;
        const r = button.getBoundingClientRect(), bounds = outline.getBoundingClientRect();
        return r.top >= bounds.top - 1 && r.bottom <= bounds.bottom + 1;
      });
      const visible = await visibleOutline();
      scan.pages.push(visible);
      const target = visible.find(button => button.name === name);
      if (target) {
        scan.found = target;
        await page.locator(`#outline button[data-declaration="${target.id}"]`).click();
        await page.waitForFunction(id => document.querySelector('#outline [aria-current="true"]')?.dataset.declaration === id, target.id);
        await paint(page);
        return true;
      }
      const signature = visible.map(button => button.position).join(",");
      if (positions.has(signature)) break;
      positions.add(signature);
      await page.keyboard.press(fromEnd ? "PageUp" : "PageDown");
    }
    scan.found = null;
    return false;
  };
  const compiler = "src/compiler/mod.rs";
  await checkpoint("initial", "Open the pinned Rust/Python workspace with a fresh analysis cache.");
  await filter("compile_file");
  await checkpoint("symbol-in-path-filter", "Deliberate capability probe: can the file filter locate a known declaration?");
  await filter("src/compiler/");
  await chooseFile(compiler);
  await checkpoint("compiler-module", "Choose the compiler module through the accessible file list.");
  assert.ok(await seek("compile_file"));
  const entry = await checkpoint("compile-entry", "Inspect the compilation entry point and its source-read call.");
  assert.ok(entry.sourceRows.some(row => row.text.includes("self.read_file")));
  assert.ok(await seek("read_file"));
  const read = await checkpoint("read-policy", "Inspect reader selection, the fallback condition and error propagation.");
  assert.ok(read.sourceRows.some(row => row.text.includes("ErrorKind::NotFound")));
  assert.ok(read.sourceRows.some(row => row.text.includes('with_extension("c")')));
  report.readAnchorBeforeLeaving = read;
  await filter("source_reader");
  await chooseFile("src/compiler/source_reader.rs");
  const traitFound = await seek("SourceReader");
  report.readerTraitInOutline = traitFound;
  await checkpoint("reader-contract", "Follow the known reader path and inspect its contract; no source identifier link is available.");
  const diskFound = await seek("DiskSourceReader");
  report.diskReaderInOutline = diskFound;
  await checkpoint("disk-reader", "Inspect the default disk reader and raw-byte read implementation.");
  await filter("src/compiler/mod.rs");
  await chooseFile(compiler);
  report.readAnchorAfterManualReturn = await checkpoint("manual-return", "Return through the file filter/list and inspect retained declaration/scroll context.");
  await filter("src/compiler/source.rs");
  await chooseFile("src/compiler/source.rs");
  assert.ok(await seek("CompilerSource"));
  await checkpoint("source-identity", "Inspect input, resolved path and diagnostic-name ownership.");
  assert.ok(await seek("name"));
  await checkpoint("diagnostic-name", "Locate the diagnostic-name accessor through the declaration outline.");
  await filter("src/compiler/mod.rs");
  await chooseFile(compiler);
  assert.ok(await seek("file_reads_stop_after_extension_fallback_and_preserve_other_io_errors", true));
  await checkpoint("regression-test", "Locate the existing source-read regression test using End and outline navigation.");
  await page.locator("#source").focus();
  await page.keyboard.press("PageDown");
  await page.waitForTimeout(250);
  const cases = await checkpoint("read-error-cases", "Read the expected read-attempt cases, including permission errors.");
  assert.ok(cases.sourceRows.some(row => row.text.includes("PermissionDenied")));
  report.beforeBrowserBack = await state();
  await page.goBack();
  report.afterBrowserBack = await checkpoint("browser-back", "Deliberate capability probe: does browser Back restore the previous source location?");

  // Independent checks follow the UI replay; they do not supply navigation targets.
  report.inputSha256 = {};
  for (const path of [compiler, "src/compiler/source_reader.rs", "src/compiler/source.rs"]) {
    report.inputSha256[path] = createHash("sha256").update(await readFile(join(input.root, path))).digest("hex");
  }
  let checked = 0;
  for (const step of report.steps) {
    if (!step.state.file || !report.inputSha256[step.state.file]) continue;
    const lines = (await readFile(join(input.root, step.state.file), "utf8")).split("\n");
    for (const row of step.state.sourceRows) {
      if (row.text.startsWith("…") || row.text.endsWith("…")) continue;
      assert.equal(row.text, lines[row.line - 1], `${step.name} line ${row.line}`);
      checked++;
    }
  }
  report.verifiedVisibleRows = checked;
  const observations = new Set(report.steps.map(step => step.state.observation).filter(Boolean));
  assert.equal(observations.size, 1);
  report.snapshotConsistent = true;
  assert.deepEqual(report.pageErrors, []);
  await Promise.all(responseJobs);
  report.sourceWorktreeAfter = git("status", "--porcelain=v1");
  assert.equal(report.sourceWorktreeAfter, report.sourceWorktreeBefore);
  report.completed = true;
} catch (error) {
  report.completed = false;
  report.failure = { message: error.message, stack: error.stack };
  throw error;
} finally {
  await context?.tracing.stop({ path: join(output, "trace.zip") });
  await context?.close();
  if (videoPath) await rename(videoPath, join(output, "walkthrough.webm"));
  await chromium?.close();
  await host?.close();
  await writeFile(join(output, "observations.json"), JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  await rm(temporary, { recursive: true, force: true });
}
