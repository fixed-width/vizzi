import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { buildHost, browser, workspace, startHost, openWorkspace, paint } from "./support.mjs";

let chromium, input, host;
const text = `const HTML: &str = "<img src=x onerror='window.injected=true'>";\nfn first() {}\nfn anchor() {\n${"    // captured line\n".repeat(10000)}}\n`;
const largeOutline = Array.from({ length: 50000 }, (_, i) => `fn item_${String(i).padStart(5, "0")}() {}`).join("\n");
before(async () => {
  buildHost();
  input = await workspace({ files: { "src/a.rs": text, "src/b.rs": "fn beta() {}", "src/partial.rs": "fn broken( {", "src/encoding.rs": Buffer.from([255]), "src/z.py": "class Example:\n    def read(self): pass\n", "include/shared.h": "struct Header;", "include/device.cuh": "__device__ void kernel();", "include/encoding.h": Buffer.from([255]), "include/example.c++": "class Example {};", "include/sample.c": "int sum(int a, int b) { return a + b; }", "include/assigned/defs.h": "class Definition { public: int read() { return 1; } };", "zzz/large.rs": largeOutline }, sourceArguments: ["--c-family-root", "include", "--syntax-language", "include/assigned=cpp", "--syntax-cache-mib", "512"] });
  host = await startHost(input);
  chromium = await browser();
});
after(async () => { await chromium?.close(); await host?.close(); await input?.close(); });

test("a benchmark browser can isolate its desktop bus and still load the workspace", { skip: process.platform !== "linux" }, async () => {
  const parentBus = process.env.DBUS_SESSION_BUS_ADDRESS;
  const isolated = await browser({ desktopBus: false });
  let session;
  try {
    const cdp = await isolated.newBrowserCDPSession();
    const info = await cdp.send("SystemInfo.getProcessInfo");
    const pid = info.processInfo.find(item => item.type === "browser").id;
    const expectedScope = await readFile("/proc/self/cgroup", "utf8");
    assert.equal(await readFile(`/proc/${pid}/cgroup`, "utf8"), expectedScope);
    assert.equal(process.env.DBUS_SESSION_BUS_ADDRESS, parentBus);
    session = await openWorkspace(isolated, host);
    await session.page.waitForFunction(() => document.querySelectorAll(".code-line").length > 0);
    assert.ok((await session.page.locator("#file-title").textContent()).length > 0);
    assert.deepEqual(session.errors, []);
    assert.equal(await readFile(`/proc/${pid}/cgroup`, "utf8"), expectedScope);
  } finally { await session?.close(); await isolated.close(); }
});

test("a large outline keeps bounded DOM and keyboard/source identity across edits", async () => {
  const session = await openWorkspace(chromium, host); const { page } = session;
  try {
    await page.locator("summary").click();
    await page.getByRole("button", { name: "zzz/large.rs", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "zzz/large.rs");
    assert.ok(await page.locator("#outline button[data-declaration]").count() <= 200);
    await page.locator("#outline button[data-declaration]").first().focus();
    await page.keyboard.press("End");
    await page.waitForFunction(() => document.activeElement?.textContent.startsWith("item_49999"));
    assert.equal(await page.locator("#outline [role=listitem]").last().getAttribute("aria-setsize"), "50000");
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => document.querySelector('#outline [aria-current=true]')?.textContent.startsWith("item_49999"));
    assert.ok((await page.locator("#source").textContent()).includes("item_49999"));
    const snapshot = await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(input.root, "zzz/large.rs"), "// moved\n" + largeOutline);
    await page.waitForFunction(previous => document.getElementById("snapshot").title !== previous, snapshot);
    assert.ok((await page.locator('#outline [aria-current=true]').textContent()).startsWith("item_49999"));
    assert.ok(await page.locator("#outline button[data-declaration]").count() <= 200);
    await page.keyboard.press("Home");
    await page.waitForFunction(() => document.activeElement?.textContent.startsWith("item_00000"));
    await page.keyboard.press("ArrowDown"); await page.keyboard.press("Space");
    await page.waitForFunction(() => document.querySelector('#outline [aria-current=true]')?.textContent.startsWith("item_00001"));
    assert.ok((await page.locator("#source").textContent()).includes("item_00001"));
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

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
    assert.equal(await page.locator("#outline button[data-declaration]").count(), 0);
    assert.ok((await page.locator("#counts").textContent()).includes("12 files"));
    const view = await (await fetch(`${host.url}/api/view?filter=shared.h`)).json();
    assert.equal(view.schema, 2); assert.equal(view.analyzed, 9); assert.equal(view.source_only, 3);
    assert.equal(view.source.classification.reason, "language_unassigned");
    assert.equal(view.source.encoding, "utf8");
    const full = await (await fetch(`${host.url}/api/view`)).json();
    const tile = await page.evaluate(id=>window.vizziMetrics.scene.files.find(file=>file.id===id),view.selection.file);
    await paint(page);
    const bounds = await page.locator("#map").boundingBox();
    const camera = await page.evaluate(() => window.vizziMetrics.lastFrame.camera);
    await page.mouse.move(bounds.x + camera.x + (tile.rect.x + tile.rect.width / 2) * camera.scale, bounds.y + camera.y + (tile.rect.y + tile.rect.height / 2) * camera.scale);
    await page.waitForFunction(() => !document.getElementById("tooltip").hidden);
    assert.ok((await page.locator("#tooltip").textContent()).toLowerCase().includes("source only"));
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
    assert.ok((await page.locator("#coverage").textContent()).includes("cpp"));
    assert.ok((await page.locator("#outline").textContent()).includes("Example"));
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("C/C++ definitions and explicitly assigned headers share source selection across edits", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    await page.locator("summary").click();
    await page.getByRole("button", { name: "include/sample.c", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "include/sample.c");
    assert.ok((await page.locator("#coverage").textContent()).includes("c: syntax parsed"));
    assert.ok((await page.locator("#outline").textContent()).includes("sum"));
    await page.getByRole("button", { name: "include/assigned/defs.h", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "include/assigned/defs.h");
    assert.ok((await page.locator("#coverage").textContent()).includes("assigned for include/assigned"));
    assert.ok((await page.locator("#coverage").textContent()).includes("configuration unknown"));
    await page.getByRole("button", { name: /^Reveal (?:function|method) read starting at Line/ }).click();
    const snapshot = await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(input.root, "include/assigned/defs.h"), "// shifted\nclass Definition { public: int read() { return 1; } };");
    await page.waitForFunction(previous => document.getElementById("snapshot").title !== previous, snapshot);
    assert.ok((await page.locator('#outline [aria-current="true"]').textContent()).startsWith("read"));
    assert.ok((await page.locator("#source").textContent()).includes("// shifted"));
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
    assert.ok((await page.locator("#counts").textContent()).includes("files"));
    await page.getByRole("button", { name: /^Reveal (?:function|method) read starting at Line/ }).click();
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
    await page.getByRole("button", { name: /^Reveal function anchor starting at Line/ }).click();
    await page.waitForFunction(() => document.querySelector('#outline button[data-declaration][aria-current="true"]')?.textContent.startsWith("anchor"));
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
    await page.getByRole("button", { name: /^Reveal function anchor starting at Line/ }).click();
    await page.waitForFunction(() => document.getElementById("source").scrollTop < 100);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("map picking remains linked to source after zoom and keyboard pan", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    const view = await (await fetch(`${host.url}/api/view`)).json();
    const target = await page.evaluate(()=>window.vizziMetrics.scene.files.find(file=>file.path==="src/b.rs"));
    await paint(page);
    const bounds = await page.locator("#map").boundingBox();
    const position = async () => {
      const camera = await page.evaluate(() => window.vizziMetrics.lastFrame.camera);
      return { x: bounds.x + camera.x + (target.rect.x + target.rect.width / 2) * camera.scale,
        y: bounds.y + camera.y + (target.rect.y + target.rect.height / 2) * camera.scale };
    };
    let point = await position();
    await page.mouse.move(point.x, point.y);
    await page.mouse.wheel(0, -200);
    await paint(page);
    await page.locator("#map").focus(); await page.keyboard.press("ArrowRight");
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
    const canvas = await session.page.locator("#map").boundingBox();
    assert.ok(canvas.width > 250 && canvas.height >= 200, `canvas is only ${canvas.width} × ${canvas.height}`);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("opening the file list resizes the canvas with its viewport", async () => {
  const session = await openWorkspace(chromium, host);
  try {
    await paint(session.page);
    const original = await session.page.locator("#map").evaluate(canvas => canvas.clientHeight);
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
    await page.getByRole("button", { name: /^Reveal function beta starting at Line/ }).click();
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

test("deferred files stay readable and resume outlines after an admitted edit", async () => {
  const large = "// deferred source stays readable\nfn original() {}\n" + " ".repeat(800000);
  const fixture = await workspace({files:{"a.rs":large},sourceArguments:["--analysis-input-mib","1","--source-spool-mib","16"]});
  let live,session;
  try {
    live=await startHost(fixture);session=await openWorkspace(chromium,live);const {page}=session;
    await page.waitForFunction(()=>document.getElementById("coverage").textContent.includes("analysis deferred"));
    assert.ok((await page.locator("#source").textContent()).includes("deferred source stays readable"));
    assert.ok((await page.locator("#diagnostics").textContent()).includes("per-file"));
    assert.ok((await page.locator("#counts").textContent()).includes("1 awaiting analysis"));
    assert.equal(await page.locator("#outline button[data-declaration]").count(),0);
    const old=await (await fetch(`${live.url}/api/view`)).json();
    assert.equal(old.deferred,1);assert.equal(old.source.classification.kind,"assigned");
    assert.equal(old.source.outlines[0].coverage,"deferred");
    await writeFile(join(fixture.root,"a.rs"),"fn resumed() {}\n");
    await page.waitForFunction(()=>document.getElementById("coverage").textContent.includes("syntax parsed"));
    assert.ok((await page.locator("#outline").textContent()).includes("resumed"));
    assert.ok(!(await page.locator("#counts").textContent()).includes("awaiting analysis"));
    const pin=await (await fetch(`${live.url}/api/source?snapshot=${old.snapshot}&file=${old.selection.file}`)).json();
    assert.equal(pin.text,large);
    const oldView=await (await fetch(`${live.url}/api/view?snapshot=${old.snapshot}&file=${old.selection.file}`)).json();
    assert.equal(oldView.source.outlines[0].coverage,"deferred");
    assert.deepEqual(session.errors,[]);
  } finally {await session?.close();await live?.close();await fixture.close();}
});
