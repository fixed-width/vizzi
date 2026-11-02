import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { buildHost, browser, workspace, startHost, openWorkspace, paint } from "./support.mjs";

let chromium, input, host;
const text = `const HTML: &str = "<img src=x onerror='window.injected=true'>";\nfn first() {}\nfn anchor() {\n${"    // captured line\n".repeat(10000)}}\n`;
before(async () => {
  buildHost();
  input = await workspace({ files: { "src/a.rs": text, "src/b.rs": "fn beta() {}", "src/partial.rs": "fn broken( {", "src/encoding.rs": Buffer.from([255]), "src/z.py": "class Example:\n    def read(self): pass\n", "include/shared.h": "struct Header;", "include/device.cuh": "__device__ void kernel();", "include/encoding.h": Buffer.from([255]), "include/example.c++": "class Example {};" }, sourceArguments: ["--c-family-root", "include"] });
  host = await startHost(input);
  chromium = await browser();
});
after(async () => { await chromium?.close(); await host?.close(); await input?.close(); });

test("source-only headers expose reasons, encoding, and pinned metadata without invented outlines", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    await page.locator("summary").click();
    await page.getByRole("button", { name: "include/shared.h", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "include/shared.h");
    assert.ok((await page.locator("#coverage").textContent()).includes("Source only"));
    assert.ok((await page.locator("#diagnostics").textContent()).includes("language is unassigned"));
    assert.ok((await page.locator("#source").textContent()).includes("struct Header;"));
    assert.equal(await page.locator("#outline button").count(), 0);
    assert.ok((await page.locator("#counts").textContent()).includes("4 source only"));
    const view = await (await fetch(`${host.url}/api/view?filter=shared.h`)).json();
    assert.equal(view.schema, 2); assert.equal(view.analyzed, 5); assert.equal(view.source_only, 4);
    assert.equal(view.source.classification.reason, "language_unassigned");
    assert.equal(view.source.encoding, "utf8");
    const full = await (await fetch(`${host.url}/api/view`)).json();
    const tile = full.files.find(file => file.id === view.selection.file);
    await paint(page);
    const bounds = await page.locator("canvas").boundingBox();
    const camera = await page.evaluate(() => window.vizziMetrics.lastFrame.camera);
    await page.mouse.move(bounds.x + camera.x + (tile.rect.x + tile.rect.width / 2) * camera.scale, bounds.y + camera.y + (tile.rect.y + tile.rect.height / 2) * camera.scale);
    await page.waitForFunction(() => !document.getElementById("tooltip").hidden);
    assert.ok((await page.locator("#tooltip").textContent()).includes("source only"));
    const pinned = `${host.url}/api/source?snapshot=${view.snapshot}&file=${view.selection.file}`;
    const snapshot = await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(input.root, "include/shared.h"), "struct Updated;");
    await page.waitForFunction(previous => document.getElementById("snapshot").title !== previous, snapshot);
    assert.ok((await page.locator("#source").textContent()).includes("struct Updated;"));
    const old = await (await fetch(pinned)).json();
    assert.equal(old.schema, 2); assert.equal(old.text, "struct Header;");
    assert.deepEqual(old.classification, view.source.classification); assert.equal(old.policy, view.policy);
    await page.getByRole("button", { name: "include/device.cuh", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "include/device.cuh");
    assert.ok((await page.locator("#diagnostics").textContent()).includes("CUDA"));
    await page.getByRole("button", { name: "include/encoding.h", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "include/encoding.h");
    assert.ok((await page.locator("#diagnostics").textContent()).includes("UTF-8"));
    assert.ok((await page.locator("#diagnostics").textContent()).includes("language is unassigned"));
    await page.getByRole("button", { name: "include/example.c++", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "include/example.c++");
    assert.ok((await page.locator("#diagnostics").textContent()).includes("analyzer is unavailable"));
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("Python source and outline share selection and update through native notifications", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    await page.locator("summary").click();
    await page.getByRole("button", { name: "src/z.py", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "src/z.py");
    assert.ok((await page.locator("#coverage").textContent()).includes("python"));
    assert.ok((await page.locator("#counts").textContent()).includes("source files"));
    await page.getByRole("button", { name: /^read/ }).click();
    const selected = await page.locator('#outline [aria-current="true"]').textContent();
    const snapshot = await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(input.root, "src/z.py"), "# new position\nclass Example:\n    def read(self): pass\n");
    await page.waitForFunction(previous => document.getElementById("snapshot").title !== previous, snapshot);
    assert.ok((await page.locator('#outline [aria-current="true"]').textContent()).startsWith("read"));
    assert.ok(selected.startsWith("read"));
    assert.ok((await page.locator("#source").textContent()).includes("# new position"));
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("Chromium can paint a local page", async () => {
  const page = await chromium.newPage();
  try {
    await page.setContent('<canvas id="probe" width="10" height="10"></canvas>');
    const pixel = await page.evaluate(() => {
      const context = document.getElementById("probe").getContext("2d");
      context.fillStyle = "#265dc5";
      context.fillRect(0, 0, 10, 10);
      return [...context.getImageData(5, 5, 1, 1).data];
    });
    assert.deepEqual(pixel, [38, 93, 197, 255]);
    console.log(`Browser: ${chromium.version()}`);
  } finally { await page.close(); }
});

test("initial map paints readable file labels", async () => {
  const context = await chromium.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  await page.addInitScript(() => {
    window.paintedLabels = [];
    const fillText = CanvasRenderingContext2D.prototype.fillText;
    CanvasRenderingContext2D.prototype.fillText = function(text, ...args) { window.paintedLabels.push(String(text)); return fillText.call(this, text, ...args); };
  });
  try {
    await page.goto(host.url);
    await page.waitForFunction(() => document.getElementById("snapshot").textContent.length === 8);
    await paint(page);
    assert.ok(await page.evaluate(() => window.paintedLabels.includes("a.rs")), "the initial overview should have a readable file label");
  } finally { await context.close(); }
});

test("an edit preserves declaration selection and the reader's scroll position", async () => {
  await writeFile(join(input.root, "src/a.rs"), text);
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    await page.locator("summary").click();
    await page.getByRole("button", { name: "src/a.rs", exact: true }).click();
    await page.getByRole("button", { name: /^anchor/ }).click();
    await page.waitForFunction(() => document.querySelector('#outline button[aria-current="true"]')?.textContent.startsWith("anchor"));
    await page.locator("#source").evaluate(node => { node.scrollTop = 150000; });
    await paint(page);
    const before = await page.locator("#source").evaluate(node => node.scrollTop);
    assert.ok(before > 100000);
    assert.ok(await page.locator(".code-line").count() < 110, "source rendering must stay virtualised");
    const snapshot = await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(input.root, "src/a.rs"), "// edit probe\n" + text);
    await page.waitForFunction(previous => document.getElementById("snapshot").title !== previous, snapshot);
    await paint(page);
    assert.ok((await page.locator("#outline [aria-current=true]").textContent()).startsWith("anchor"));
    const after = await page.locator("#source").evaluate(node => node.scrollTop);
    assert.ok(Math.abs(after - before) <= 22, `background edit moved the reader from ${before} to ${after}`);
    await page.getByRole("button", { name: /^anchor/ }).click();
    await page.waitForFunction(() => document.getElementById("source").scrollTop < 100);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("map picking remains linked to source after zoom and keyboard pan", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    const view = await (await fetch(`${host.url}/api/view`)).json();
    const target = view.files.find(file => file.path === "src/b.rs");
    await paint(page);
    const bounds = await page.locator("canvas").boundingBox();
    const position = async () => {
      const camera = await page.evaluate(() => window.vizziMetrics.lastFrame.camera);
      return { x: bounds.x + camera.x + (target.rect.x + target.rect.width / 2) * camera.scale,
        y: bounds.y + camera.y + (target.rect.y + target.rect.height / 2) * camera.scale };
    };
    let point = await position();
    await page.mouse.move(point.x, point.y);
    await page.mouse.wheel(0, -200);
    await paint(page);
    await page.locator("canvas").focus(); await page.keyboard.press("ArrowRight");
    await paint(page);
    point = await position();
    await page.mouse.click(point.x, point.y);
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "src/b.rs");
    await paint(page);
    assert.ok((await page.locator("#source").textContent()).includes("fn beta() {}"));
    assert.ok((await page.locator("#outline").textContent()).includes("beta"));
    await page.mouse.move(point.x + 1, point.y);
    await page.waitForFunction(() => !document.getElementById("tooltip").hidden);
    assert.ok((await page.locator("#tooltip").textContent()).includes("src/b.rs"));
    await page.getByRole("button", { name: "Fit map", exact: true }).click();
    await paint(page);
    const frame = await page.evaluate(() => window.vizziMetrics.lastFrame);
    assert.equal(frame.visibleFiles, view.files.length);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("narrow viewport preserves a bounded source view without horizontal page overflow", async () => {
  const session = await openWorkspace(chromium, host, { width: 390, height: 844 });
  try {
    await paint(session.page);
    assert.ok(await session.page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "page overflows a narrow viewport");
    assert.ok(await session.page.locator(".code-line").count() < 110);
    const canvas = await session.page.locator("canvas").boundingBox();
    assert.ok(canvas.width > 250 && canvas.height >= 200, `canvas is only ${canvas.width} × ${canvas.height}`);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("opening the file list resizes the canvas with its viewport", async () => {
  const session = await openWorkspace(chromium, host);
  try {
    await paint(session.page);
    const original = await session.page.locator("canvas").evaluate(canvas => canvas.clientHeight);
    await session.page.locator("summary").click();
    await paint(session.page);
    const dimensions = await session.page.evaluate(() => ({ actual: document.getElementById("map").clientHeight, pixels: document.getElementById("map").height, dpr: Math.min(devicePixelRatio, 2), drawn: window.vizziMetrics.lastFrame.height }));
    assert.ok(dimensions.actual < original, "fixture must change the map viewport height");
    assert.equal(dimensions.drawn, dimensions.actual, "canvas drawing must follow its own viewport size");
    assert.equal(dimensions.pixels, Math.round(dimensions.actual * dimensions.dpr));
  } finally { await session.close(); }
});

test("a live client resynchronises after host restart without polling or losing selection", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    await page.locator("summary").click();
    await page.getByRole("button", { name: "src/b.rs", exact: true }).click();
    await page.getByRole("button", { name: /^beta/ }).click();
    await page.waitForFunction(() => document.querySelector('#outline [aria-current="true"]')?.textContent.startsWith("beta"));
    const before = await page.locator("#file-title").textContent();
    const port = new URL(host.url).port;
    await host.close();
    await page.locator("#error").waitFor({ state: "visible" });
    assert.equal(await page.locator("#file-title").textContent(), before);
    await writeFile(join(input.root, "src/b.rs"), "// changed while disconnected\nfn beta() {}");
    host = await startHost(input, 4, port);
    await page.locator("#error").waitFor({ state: "hidden", timeout: 5000 });
    await page.waitForFunction(() => document.getElementById("source").textContent.includes("changed while disconnected"));
    assert.ok((await page.locator("#outline [aria-current=true]").textContent()).startsWith("beta"));
    assert.equal(session.requests.filter(path => path === "/api/status").length, 0);
    assert.ok(session.requests.filter(path => path === "/api/events").length >= 2);
  } finally { await session.close(); }
});

test("source text stays text, and syntax/encoding limits are visible", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    assert.equal(await page.locator("#source img").count(), 0);
    assert.equal(await page.evaluate(() => window.injected), undefined);
    await page.locator("summary").click();
    await page.getByRole("button", { name: "src/partial.rs", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("coverage").textContent.includes("partial syntax"));
    assert.equal(await page.locator("#diagnostics").isVisible(), true);
    await page.getByRole("button", { name: "src/encoding.rs", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("coverage").textContent.includes("analysis unavailable"));
    assert.ok((await page.locator("#source").textContent()).includes("encoding"));
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});
