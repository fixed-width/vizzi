import assert from "node:assert/strict";
import { readFile, writeFile, mkdir, readdir, stat, access } from "node:fs/promises";
import { join, dirname, basename, extname } from "node:path";
import { cpus, totalmem, platform, release } from "node:os";
import { createHash } from "node:crypto";
import { setTimeout as delay } from "node:timers/promises";
import { spawnSync } from "node:child_process";
import { buildHost, browser, workspace, startHost, openWorkspace, paint } from "./support.mjs";
import { summary } from "./statistics.mjs";

const options = { duration: 4000, samples: 3 };
for (let i = 2; i < process.argv.length; i++) {
  const flag = process.argv[i];
  if (["--measure", "--headed"].includes(flag)) continue;
  if (!["--source", "--revision", "--output", "--duration", "--samples"].includes(flag)) throw new Error(`Unknown argument ${flag}`);
  const value = process.argv[++i];
  if (!value) throw new Error(`${flag} requires a value`);
  options[flag.slice(2)] = ["--duration", "--samples"].includes(flag) ? Number(value) : value;
}
assert.ok(options.output, "--output is required");
assert.ok(Number.isInteger(options.duration) && options.duration >= 1000 && options.duration <= 30000);
assert.ok(Number.isInteger(options.samples) && options.samples >= 1 && options.samples <= 10);
assert.ok(!options.source || options.revision, "a reference source needs --revision");
try { await access(options.output); throw new Error(`Refusing to overwrite ${options.output}`); } catch (error) { if (error.code !== "ENOENT") throw error; }

async function capturedFiles(root, directory = root, files = []) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (directory === root && ["target", ".git", "node_modules"].includes(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) await capturedFiles(root, path, files);
    else if (entry.isFile() && entry.name.endsWith(".rs")) files.push({ path, bytes: (await stat(path)).size });
  }
  return files;
}
async function settled(host) {
  const deadline = performance.now() + 30000;
  while (performance.now() < deadline) {
    const status = await (await fetch(`${host.url}/api/status`)).json();
    if (!status.pending) { assert.equal(status.error, null); return status; }
    await delay(20);
  }
  throw new Error("Refresh did not settle");
}
async function sampleRss(pid) {
  if (platform() !== "linux") return null;
  const status = await readFile(`/proc/${pid}/status`, "utf8");
  return { currentKiB: Number(status.match(/VmRSS:\s+(\d+)/)?.[1]), highWaterKiB: Number(status.match(/VmHWM:\s+(\d+)/)?.[1]) };
}

async function beginProbe(page) {
  await page.evaluate(() => {
    const probe = window.probe = { frames: [], paints: [], longFrames: [], active: true, previous: null, lastFrame: window.vizziMetrics.lastFrame };
    const tick = timestamp => {
      if (!probe.active) return;
      if (probe.previous !== null) probe.frames.push({ atEpochMs: performance.timeOrigin + timestamp, duration: timestamp - probe.previous });
      if (window.vizziMetrics.lastFrame !== probe.lastFrame) {
        probe.paints.push(performance.timeOrigin + timestamp);
        probe.lastFrame = window.vizziMetrics.lastFrame;
      }
      probe.previous = timestamp; requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
    if (PerformanceObserver.supportedEntryTypes.includes("long-animation-frame")) {
      probe.observer = new PerformanceObserver(list => probe.longFrames.push(...list.getEntries().map(entry => ({ duration: entry.duration, blockingDuration: entry.blockingDuration }))));
      probe.observer.observe({ type: "long-animation-frame" });
    }
    for (const key of ["drawCpuMs", "interactionFrameMs", "requestMs", "pickingMs"]) window.vizziMetrics[key].length = 0;
  });
}
async function endProbe(page) {
  return page.evaluate(() => {
    window.probe.active = false;
    const remaining = window.probe.observer?.takeRecords() ?? [];
    window.probe.observer?.disconnect();
    return { frames: window.probe.frames, paintTimesEpochMs: window.probe.paints, longFrames: [...window.probe.longFrames, ...remaining.map(entry => ({ duration: entry.duration, blockingDuration: entry.blockingDuration }))], metrics: window.vizziMetrics };
  });
}

async function interaction(page, host, edits) {
  const before = await settled(host);
  await beginProbe(page);
  let watching = true, pendingObservations = 0, changed = null;
  const statusSamples = [];
  const monitor = (async () => {
    while (watching) {
      const status = await (await fetch(`${host.url}/api/status`)).json();
      statusSamples.push({ atEpochMs: performance.timeOrigin + performance.now(), pending: status.pending });
      if (status.pending) pendingObservations++;
      if (status.snapshot !== before.snapshot && !changed) changed = status;
      await delay(25);
    }
  })();
  let started;
  const inputs = [];
  try {
    const bounds = await page.locator("canvas").boundingBox();
    const x = bounds.x + bounds.width / 2, y = bounds.y + bounds.height / 2;
    await page.mouse.move(x, y); await page.mouse.down();
    started = performance.now();
    if (edits) await edits();
    let step = 0, dragging = true, nextInput = started, inputFailure;
    const inFlight = new Set();
    const submit = (kind, action) => {
      const input = { atMs: performance.now() - started, kind }; inputs.push(input);
      const job = action().then(() => { input.completedAtMs = performance.now() - started; }, error => { inputFailure = error; }).finally(() => inFlight.delete(job));
      inFlight.add(job);
    };
    while (performance.now() - started < options.duration) {
      if (inputFailure) throw inputFailure;
      if (inFlight.size >= 6) await Promise.race(inFlight);
      if (performance.now() - started < options.duration / 2) {
        submit("pan", () => page.mouse.move(x + Math.sin(step / 30) * 70, y + Math.cos(step / 40) * 45));
      } else {
        if (dragging) { await Promise.all(inFlight); await page.mouse.up(); await page.mouse.move(x, y); dragging = false; }
        submit("zoom", () => page.mouse.wheel(0, Math.cos(step / 18) * 2));
      }
      nextInput += 1000 / 60;
      await delay(Math.max(0, nextInput - performance.now()));
      step++;
    }
    await Promise.all(inFlight);
    if (inputFailure) throw inputFailure;
    await page.mouse.up();
  } finally { watching = false; await monitor; }
  const sampled = await endProbe(page);
  const intervals = sampled.frames.map(frame => frame.duration);
  const drawIntervals = sampled.paintTimesEpochMs.slice(1).map((time, index) => time - sampled.paintTimesEpochMs[index]);
  const pendingIntervals = [];
  let statusIndex = 0;
  for (const frame of sampled.frames) {
    while (statusIndex + 1 < statusSamples.length && statusSamples[statusIndex + 1].atEpochMs <= frame.atEpochMs) statusIndex++;
    if (statusSamples[statusIndex]?.pending) pendingIntervals.push(frame.duration);
  }
  assert.ok(intervals.length > 10, "not enough browser frames");
  assert.ok(sampled.metrics.lastFrame.labels > 0, "measurement lost all readable labels");
  return { elapsedMs: performance.now() - started, pendingObservations, publishedRefresh: changed,
    inputs, inputTargetHz: 60, paintObservationTimesEpochMs: sampled.paintTimesEpochMs, drawIntervalsMs: drawIntervals, drawIntervalSummary: summary(drawIntervals),
    frameIntervalsMs: intervals, frameSummary: summary(intervals),
    statusSamples, observedPendingFrameIntervalsMs: pendingIntervals, observedPendingFrameSummary: summary(pendingIntervals),
    drawCpuMs: sampled.metrics.drawCpuMs, drawSummary: summary(sampled.metrics.drawCpuMs),
    longFrames: sampled.longFrames, finalFrame: sampled.metrics.lastFrame,
    requestMs: sampled.metrics.requestMs, nativeRss: await sampleRss(host.pid) };
}

async function inspectAndPick(page, host) {
  const view = await (await fetch(`${host.url}/api/view`)).json();
  const frame = await page.evaluate(() => window.vizziMetrics.lastFrame);
  const bounds = await page.locator("canvas").boundingBox();
  const target = view.files.find(file => file.rect.y * frame.camera.scale + frame.camera.y + file.rect.height * frame.camera.scale < frame.height - 20);
  assert.ok(target);
  const x = bounds.x + frame.camera.x + (target.rect.x + target.rect.width / 2) * frame.camera.scale;
  const y = bounds.y + frame.camera.y + (target.rect.y + target.rect.height / 2) * frame.camera.scale;
  for (let i = 0; i < 60; i++) { await page.mouse.move(x + (i % 2), y); }
  await page.mouse.click(x, y);
  await page.waitForFunction(path => document.getElementById("file-title").textContent === path, target.path);
  await paint(page);
  const picked = await page.evaluate(() => ({ samples: window.vizziMetrics.pickingMs, renderedLines: document.querySelectorAll(".code-line").length }));
  return { fileId: target.id, path: target.path, samples: picked.samples, summary: summary(picked.samples), renderedLines: picked.renderedLines };
}

async function singleEdits(page, input, host, picking) {
  const view = await (await fetch(`${host.url}/api/view?file=${encodeURIComponent(picking.fileId)}`)).json();
  const path = join(input.root, view.source.path), original = await readFile(path);
  const results = [];
  for (const offset of [0, 97, 211, 367, 463]) {
    await settled(host); await delay(offset);
    const marker = `// browser-visible-${offset}`;
    const started = performance.now();
    await writeFile(path, Buffer.concat([Buffer.from(marker + "\n"), original]));
    await page.waitForFunction(marker => [...document.querySelectorAll(".code-line")].some(line => line.textContent.includes(marker)), marker);
    await paint(page);
    results.push({ offsetMs: offset, writeToDomAndTwoRafMs: performance.now() - started });
  }
  return { samples: results, summary: summary(results.map(result => result.writeToDomAndTwoRafMs)) };
}

async function qualify(chromium, definition) {
  console.log(`Starting ${definition.name}`);
  const input = await workspace({ ...definition.input, name: definition.name });
  let host, session;
  try {
    host = await startHost(input);
    await settled(host);
    session = await openWorkspace(chromium, host);
    const files = await capturedFiles(input.root);
    const manifest = createHash("sha256");
    for (const file of files.toSorted((a, b) => a.path.localeCompare(b.path, "en"))) {
      manifest.update(JSON.stringify([file.path.slice(input.root.length + 1), createHash("sha256").update(await readFile(file.path)).digest("hex")]));
    }
    const heavy = files.toSorted((a, b) => b.bytes - a.bytes).slice(0, definition.heavyFiles);
    const originals = await Promise.all(heavy.map(file => readFile(file.path)));
    const runs = [];
    for (let sample = 0; sample < options.samples; sample++) {
      for (const condition of ["steady", "refresh"]) {
        await settled(host); await session.page.reload();
        await session.page.waitForFunction(() => window.vizziMetrics?.lastFrame?.snapshot);
        await paint(session.page);
        const run = await interaction(session.page, host, condition === "refresh" ? () => Promise.all(heavy.map((file, index) => writeFile(file.path, Buffer.concat([Buffer.from(`// background ${sample}\n`), originals[index]])))) : null);
        runs.push({ sample, condition, ...run });
        console.log(`${definition.name} ${condition} ${sample + 1}: ${run.frameSummary.count} rAF intervals; ${run.pendingObservations} pending observations`);
      }
    }
    assert.ok(runs.some(run => run.condition === "refresh" && run.pendingObservations > 0), "background analysis did not overlap interaction");
    await settled(host); await session.page.reload();
    await session.page.waitForFunction(() => window.vizziMetrics?.lastFrame?.snapshot); await paint(session.page);
    const picking = await inspectAndPick(session.page, host);
    const editLatency = await singleEdits(session.page, input, host, picking);
    const cdp = await session.page.context().newCDPSession(session.page);
    await cdp.send("Performance.enable");
    const performanceMetrics = await cdp.send("Performance.getMetrics");
    const heap = Object.fromEntries(performanceMetrics.metrics.filter(metric => ["JSHeapUsedSize", "JSHeapTotalSize", "Nodes", "Documents"].includes(metric.name)).map(metric => [metric.name, metric.value]));
    const webgpu = await session.page.evaluate(async () => {
      if (!navigator.gpu) return { available: false };
      const adapter = await navigator.gpu.requestAdapter();
      return adapter ? { available: true, info: { vendor: adapter.info.vendor, architecture: adapter.info.architecture, device: adapter.info.device, description: adapter.info.description, isFallbackAdapter: adapter.info.isFallbackAdapter } } : { available: false };
    });
    await mkdir(dirname(options.output), { recursive: true });
    const screenshot = join(dirname(options.output), `${basename(options.output, extname(options.output))}-${definition.name}-workspace.png`);
    await writeFile(screenshot, await session.page.screenshot({ fullPage: true }), { flag: "wx" });
    assert.deepEqual(session.errors, []);
    const final = await (await fetch(`${host.url}/api/view`)).json();
    return { name: definition.name, revision: definition.revision, originalSourceManifestSha256: manifest.digest("hex"), rustFiles: files.length, sourceBytes: files.reduce((sum, file) => sum + file.bytes, 0),
      overviewFiles: final.files.length, heavyFiles: heavy.map(file => ({ path: file.path.slice(input.root.length + 1), bytes: file.bytes })),
      runs, picking, editLatency, heap, webgpu, screenshot, nativeRss: await sampleRss(host.pid), errors: session.errors };
  } finally { await session?.close(); await host?.close(); await input.close(); }
}

buildHost();
const chromium = await browser();
try {
  const cdp = await chromium.newBrowserCDPSession();
  const system = await cdp.send("SystemInfo.getInfo");
  const synthetic = {};
  for (let i = 0; i < 10000; i++) synthetic[`src/group_${String(Math.floor(i / 100)).padStart(3, "0")}/file_${String(i).padStart(5, "0")}.rs`] = `pub fn file_${i}() -> usize { ${i} }\n`;
  const large = Array.from({ length: 16000 }, (_, i) => `fn work_${i}() -> usize { ${i} }\n`).join("");
  for (let i = 0; i < 8; i++) synthetic[`zzz_load/background_${i}.rs`] = large + `fn unique_${i}() {}\n`;
  const definitions = [{ name: "synthetic-10k", revision: "generated-v1:10000-small+8x16000-functions", input: { files: synthetic }, heavyFiles: 8 }];
  if (options.source) definitions.push({ name: "lpc-rs", revision: options.revision, input: { source: options.source }, heavyFiles: 32 });
  const results = [];
  for (const definition of definitions) results.push(await qualify(chromium, definition));
  const sha256 = path => readFile(path).then(bytes => createHash("sha256").update(bytes).digest("hex"));
  const report = { schema: 1, recordedUtc: new Date().toISOString(), browser: chromium.version(), node: process.version,
    playwright: JSON.parse(await readFile("node_modules/playwright/package.json", "utf8")).version,
    machine: { platform: platform(), release: release(), cpu: cpus()[0].model, logicalCpus: cpus().length, memoryBytes: totalmem() },
    gpu: { devices: system.gpu.devices, featureStatus: system.gpu.featureStatus, auxAttributes: system.gpu.auxAttributes },
    viewport: { width: 1440, height: 900, dpr: 1, headless: !process.argv.includes("--headed") },
    executor: { deviceNodes: await readdir("/dev/dri").catch(() => []), display: process.env.DISPLAY ?? null, wayland: process.env.WAYLAND_DISPLAY ?? null },
    nativeRevision: spawnSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).stdout.trim(),
    binarySha256: await sha256("target/release/vizzi"), appSha256: await sha256("crates/vizzi/web/app.js"),
    options, results,
    limitations: ["rAF intervals are scheduling observations, not GPU or physical presentation timings", "Edit observations end after matching visible DOM plus two rAF callbacks", "Pending-frame classification uses the latest status sample at approximately 25 ms resolution; status fetches may be delayed", "Source reads use warm OS caches; periodic reconciliation remains active", "Only bounded overview tiles are drawn; this does not render 10000 files simultaneously", "Heap metrics are point-in-time renderer heap, not peak browser memory", "No WebGPU/WebGL backend comparison or baseline laptop qualification"] };
  await writeFile(options.output, JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  console.log(`Saved ${options.output}`);
} finally { await chromium.close(); }
