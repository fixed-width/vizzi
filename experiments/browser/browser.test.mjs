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

test("opening a map tile keeps the cursor steady and fetches only the first source screen", async () => {
  const session = await openWorkspace(chromium, host);
  try {
    const { page, requests } = session;
    await page.getByRole("button", { name: "Fit map", exact: true }).click();
    await paint(page);
    const position = await page.evaluate(() => {
      const { rect } = vizziMetrics.scene.files.find(file => file.path === "src/b.rs");
      const { camera } = vizziMetrics.lastFrame;
      return {
        x: camera.x + (rect.x + rect.width / 2) * camera.scale,
        y: camera.y + (rect.y + rect.height / 2) * camera.scale,
      };
    });
    const map = page.locator("#map");
    await map.hover({ position });
    const hoverCursor = await map.evaluate(canvas => getComputedStyle(canvas).cursor);
    const start = requests.length;
    await page.mouse.down();
    const pressedCursor = await map.evaluate(canvas => getComputedStyle(canvas).cursor);
    await page.mouse.up();
    assert.equal(hoverCursor, "grab");
    assert.equal(pressedCursor, hoverCursor);
    await page.locator("#source .code-line").filter({ hasText: "fn beta() {}" }).waitFor();
    assert.equal(await page.locator("#file-title").textContent(), "src/b.rs");
    assert.match(await page.locator("#outline").textContent(), /beta/);
    assert.deepEqual(requests.slice(start).filter(path => path !== "/api/structure"), ["/api/inspection"]);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("a bundled inspection with a missing version keeps the current file visible", async () => {
  const session = await openWorkspace(chromium, host);
  try {
    const { page } = session;
    const title = page.locator("#file-title");
    const previous = await title.textContent();
    await page.route("**/api/inspection?**", async route => {
      const response = await route.fetch();
      const data = await response.json();
      data.source.version = "";
      data.window.version = "";
      await route.fulfill({ response, json: data });
    });
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.locator("#file-list").getByRole("button", { name: "src/b.rs", exact: true }).click();
    await page.locator("#error").getByText(/Invalid captured document summary/).waitFor({ timeout: 3000 });
    assert.equal(await title.textContent(), previous);
  } finally { await session.close(); }
});

test("map tiles retain their language fills through selection, recovery and theme changes", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  const legend = page.locator("#map-legend");
  const tileColors = () => page.evaluate(paths => {
    const { scene, lastFrame: { camera } } = vizziMetrics;
    const map = document.getElementById("map");
    const overview = document.getElementById("map-overview");
    const scale = Math.min((overview.clientWidth - 12) / scene.width, (overview.clientHeight - 12) / scene.height);
    const left = (overview.clientWidth - scene.width * scale) / 2;
    const top = (overview.clientHeight - scene.height * scale) / 2;
    function pixel(canvas, x, y) {
      const bytes = canvas.getContext("2d").getImageData(Math.floor(x), Math.floor(y), 1, 1).data;
      return "#" + Array.from(bytes.slice(0, 3), byte => byte.toString(16).padStart(2, "0")).join("");
    }
    return Object.fromEntries(paths.map(path => {
      const { rect } = scene.files.find(file => file.path === path);
      const x = camera.x + rect.x * camera.scale, y = camera.y + rect.y * camera.scale;
      const width = rect.width * camera.scale, height = rect.height * camera.scale;
      return [path, {
        fill: pixel(map, x + width * .5, y + height * .7),
        outline: pixel(map, x + 1, y + height * .7),
        corner: pixel(map, x + width - 5, y + height - 4),
        overview: pixel(overview, left + (rect.x + rect.width * .5) * scale, top + (rect.y + rect.height * .5) * scale),
      }];
    }));
  }, ["src/b.rs", "src/partial.rs", "src/encoding.rs", "src/z.py", "include/example.c++", "include/assigned/defs.h", "include/shared.h"]);
  const swatch = label => legend.getByText(label, { exact: true }).evaluate(item => getComputedStyle(item).getPropertyValue("--swatch"));
  try {
    await page.emulateMedia({ colorScheme: "light" });
    await page.getByText("Accessible file list", { exact: true }).click();
    const partial = page.getByRole("button", { name: "src/partial.rs", exact: true });
    await partial.click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "src/partial.rs");
    assert.equal(await partial.getAttribute("aria-description"), "Rust · Syntax with recovery");
    await page.getByRole("button", { name: "Fit map", exact: true }).click();
    await paint(page);
    const light = await tileColors();
    assert.equal(light["src/b.rs"].fill, await swatch("Rust"));
    assert.equal(light["src/partial.rs"].fill, light["src/b.rs"].fill);
    assert.notEqual(light["src/partial.rs"].outline, light["src/partial.rs"].fill);
    assert.equal(light["src/partial.rs"].corner, await swatch("Syntax recovery"));
    assert.equal(light["src/encoding.rs"].fill, light["src/b.rs"].fill);
    assert.equal(light["src/encoding.rs"].corner, await swatch("Analysis deferred or unavailable"));
    assert.equal(light["src/z.py"].fill, await swatch("Python"));
    assert.notEqual(light["src/z.py"].fill, light["src/b.rs"].fill);
    assert.equal(light["include/assigned/defs.h"].fill, await swatch("C++"));
    assert.equal(light["include/example.c++"].fill, light["include/assigned/defs.h"].fill);
    assert.equal(light["include/shared.h"].fill, await swatch("Other files"));
    for (const tile of Object.values(light)) assert.equal(tile.overview, tile.fill);
    await page.emulateMedia({ colorScheme: "dark" });
    await paint(page);
    const dark = await tileColors();
    assert.equal(dark["src/b.rs"].fill, await swatch("Rust"));
    assert.notEqual(dark["src/b.rs"].fill, light["src/b.rs"].fill);
    for (const tile of Object.values(dark)) assert.equal(tile.overview, tile.fill);
    await page.getByRole("searchbox", { name: "Find a file" }).fill("src/z.py");
    await legend.getByText("Syntax recovery", { exact: true }).waitFor({ state: "hidden" });
    assert.equal(await legend.getByRole("button", { name: "Rust" }).isVisible(), true);
    await page.getByRole("searchbox", { name: "Find a file" }).fill("no-matching-path");
    await page.locator("#empty").waitFor({ state: "visible" });
    assert.equal(await legend.isVisible(), true);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("a fitted dense map shows analysis on tiles under twelve pixels", async () => {
  const files = Object.fromEntries(Array.from({ length: 1024 }, (_, index) => [
    `normal-${String(index).padStart(4, "0")}.rs`, "",
  ]));
  const fixture = await workspace({ files: {
    ...files,
    "a-recovery.rs": "fn broken( {",
    "unavailable.rs": Buffer.from([255]),
  } });
  let localHost, session;
  try {
    localHost = await startHost(fixture);
    session = await openWorkspace(chromium, localHost, { width: 980, height: 720 });
    const { page } = session;
    assert.equal(await page.locator("#file-title").textContent(), "a-recovery.rs");
    await page.getByRole("button", { name: "Fit map", exact: true }).click();
    await paint(page);
    const tiles = await page.evaluate(() => {
      const { scene, lastFrame: { camera } } = vizziMetrics;
      const ctx = document.getElementById("map").getContext("2d");
      return Object.fromEntries([
        ["normal-0001.rs", "Rust"],
        ["a-recovery.rs", "Syntax recovery"],
        ["unavailable.rs", "Analysis deferred or unavailable"],
      ].map(([path, label]) => {
        const { rect } = scene.files.find(file => file.path === path);
        const x = camera.x + rect.x * camera.scale, y = camera.y + rect.y * camera.scale;
        const width = rect.width * camera.scale, height = rect.height * camera.scale;
        const swatch = [...document.querySelectorAll("#map-legend li")].find(item => item.textContent === label);
        const color = parseInt(swatch.style.getPropertyValue("--swatch").slice(1), 16);
        const left = Math.ceil(x), top = Math.ceil(y);
        const pixels = ctx.getImageData(left, top, Math.floor(x + width) - left, Math.floor(y + height) - top).data;
        let containsSwatch = false;
        for (let i = 0; i < pixels.length; i += 4) {
          if (pixels[i] * 65536 + pixels[i + 1] * 256 + pixels[i + 2] === color && pixels[i + 3] === 255) {
            containsSwatch = true;
            break;
          }
        }
        return [path, { containsSwatch, shortestSide: Math.min(width, height) }];
      }));
    });
    for (const [path, tile] of Object.entries(tiles)) {
      assert.ok(tile.shortestSide < 12, `${path} should be under twelve pixels in the fitted view`);
      assert.ok(tile.containsSwatch, `${path} should show the color from its legend swatch`);
    }
  } finally {
    await session?.close();
    await localHost?.close();
    await fixture.close();
  }
});

test("map legend toggles combine file types and Clear filters restores the full map", async () => {
  const session = await openWorkspace(chromium, host);
  try {
    const { page } = session;
    const legend = page.locator("#map-legend");
    const rust = legend.getByRole("button", { name: "Rust", exact: true });
    const python = legend.getByRole("button", { name: "Python", exact: true });
    const search = page.getByRole("searchbox", { name: "Find a file" });
    const clear = page.getByRole("group", { name: "Map filters and legend" })
      .getByRole("button", { name: "Clear filters" });
    const mapTypes = () => [...new Set(vizziMetrics.scene.files.map(file => file.type))].sort().join(",");
    const initialTypes = await page.evaluate(mapTypes);
    assert.equal(await clear.isVisible(), false);
    assert.equal(await python.getAttribute("aria-pressed", { timeout: 3000 }), "false");
    await python.press("Space");
    await page.waitForFunction(() => vizziMetrics.scene.files.map(file => file.path).join() === "src/z.py");
    await clear.waitFor({ state: "visible", timeout: 3000 });
    assert.equal(await python.getAttribute("aria-pressed"), "true");
    assert.equal(await python.evaluate(button => button === document.activeElement), true);
    await rust.click();
    await page.waitForFunction(() => new Set(vizziMetrics.scene.files.map(file => file.type)).size === 2);
    assert.equal(await page.evaluate(mapTypes), "python,rust");
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.locator("#file-list button").first().waitFor();
    assert.deepEqual(await page.locator("#file-list button").allTextContents(), [
      "src/a.rs", "src/b.rs", "src/encoding.rs", "src/partial.rs", "src/z.py", "zzz/large.rs",
    ]);
    await python.click();
    await page.waitForFunction(() => vizziMetrics.scene.files.length === 5 &&
      vizziMetrics.scene.files.every(file => file.type === "rust"));
    await search.fill("src/z.py");
    await page.locator("#empty").waitFor({ state: "visible" });
    assert.equal(await page.locator("#file-list button").count(), 0);
    assert.equal(await rust.getAttribute("aria-pressed"), "true");
    await clear.press("Enter");
    await page.locator("#empty").waitFor({ state: "hidden" });
    assert.equal(await page.evaluate(mapTypes), initialTypes);
    assert.equal(await search.inputValue(), "");
    assert.equal(await rust.getAttribute("aria-pressed"), "false");
    assert.equal(await clear.isVisible(), false);
    assert.equal(await legend.getByRole("button").first().evaluate(button => button === document.activeElement), true);

    await search.fill("src/z.py");
    await page.waitForFunction(() => vizziMetrics.scene.files.map(file => file.path).join() === "src/z.py");
    await clear.click();
    await page.waitForFunction(() => vizziMetrics.scene.files.some(file => file.type === "rust"));
    assert.equal(await search.inputValue(), "");
    assert.equal(await search.evaluate(input => input === document.activeElement), true);
    assert.deepEqual(session.errors, []);
  } finally { await session.close(); }
});

test("forced colors preserves the map swatches and adapts legend text", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    await page.emulateMedia({ forcedColors: "active" });
    await paint(page);
    const legend = page.locator("#map-legend");
    for (const label of ["Rust", "Syntax recovery", "Analysis deferred or unavailable"]) {
      const colors = await legend.getByText(label, { exact: true }).evaluate(item => {
        const hex = getComputedStyle(item).getPropertyValue("--swatch");
        const expected = hex.slice(1).match(/../g).map(byte => parseInt(byte, 16));
        return {
          expected: `rgb(${expected.join(", ")})`,
          swatch: getComputedStyle(item, "::before").backgroundColor,
          textAdjustment: getComputedStyle(item).forcedColorAdjust,
        };
      });
      assert.equal(colors.swatch, colors.expected, label);
      assert.equal(colors.textAdjustment, "auto", label);
    }
  } finally { await session.close(); }
});

test("a large repository exposes every directory, file page and local landmark", async () => {
  const files = Object.fromEntries(Array.from({ length: 2001 }, (_, index) => [
    `bulk/file-${String(index).padStart(4, "0")}.rs`, "",
  ]));
  const fixture = await workspace({ files: {
    ...files,
    "README.md": "# Project orientation\n",
    "z-late/README.md": "# Late subsystem\n",
    "z-late/Cargo.toml": '[package]\nname = "late"\nversion = "0.1.0"\n',
    "z-late/settings.toml": "enabled = true # Toggle\n",
    "z-late/guide.markdown": "See **this** and `🙂code`.\n",
    "z-late/target.rs": "fn late_target() {}\n",
  } });
  let localHost, session;
  try {
    localHost = await startHost(fixture);
    session = await openWorkspace(chromium, localHost);
    const { page } = session;
    await page.locator("#map-landmarks").getByRole("button", { name: "README.md" }).click({ timeout: 5000 });
    await page.locator("#source .syntax-keyword").getByText("# Project orientation", { exact: true }).waitFor();
    assert.match(await page.locator("#coverage").textContent(), /Source only · README/);
    const late = page.getByRole("button", { name: "Open directory z-late, 5 files", exact: true });
    await late.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("button", { name: "z-late", exact: true }).waitFor();
    await paint(page);
    assert.deepEqual(await page.locator("#map-legend li").allTextContents(), ["Rust", "Documentation", "TOML", "Project metadata"]);
    assert.equal(await page.locator("#map-breadcrumbs button[aria-current=location]").evaluate(button => button === document.activeElement), true);
    assert.equal(await page.locator("#file-title").textContent(), "README.md");
    await page.locator("#map-landmarks").getByRole("button", { name: "Cargo.toml" }).click();
    await page.locator("#source .syntax-type").getByText("[package]", { exact: true }).waitFor();
    assert.equal(await page.locator("#file-title").textContent(), "z-late/Cargo.toml");
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.locator("#file-list").getByRole("button", { name: "z-late/settings.toml", exact: true }).click();
    await page.locator("#source .syntax-constant").getByText("true", { exact: true }).waitFor();
    await page.locator("#file-list").getByRole("button", { name: "z-late/guide.markdown", exact: true }).click();
    await page.locator("#source .syntax-string").getByText("`🙂code`", { exact: true }).waitFor();
    assert.equal(await page.locator("#source .syntax-attribute").textContent(), "**this**");
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.getByRole("button", { name: "Repository root", exact: true }).click();
    await page.getByRole("button", { name: "Open directory bulk, 2,001 files", exact: true }).waitFor();
    await paint(page);
    const bulkCard = await page.evaluate(() => {
      const { rect } = vizziMetrics.scene.groups.find(group => group.path === "bulk");
      const { camera } = vizziMetrics.lastFrame;
      return { x: camera.x + (rect.x + rect.width / 2) * camera.scale,
        y: camera.y + (rect.y + rect.height / 2) * camera.scale };
    });
    await page.locator("#map").click({ position: bulkCard });
    await page.getByText("1–100 of 2,001 entries", { exact: true }).waitFor();
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.getByRole("button", { name: "Next page", exact: true }).click();
    await page.getByText("101–200 of 2,001 entries", { exact: true }).waitFor();
    assert.equal(await page.locator("#file-list button").count(), 100);
    await page.getByRole("button", { name: "bulk/file-0199.rs", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "bulk/file-0199.rs");
    assert.equal(await page.evaluate(() => vizziMetrics.scene.files.some(file => file.path === "bulk/file-0199.rs")), true);
    await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    await paint(page);
    const saved = await page.evaluate(() => vizziMetrics.lastFrame.camera);
    await page.getByRole("button", { name: "bulk/file-0198.rs", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "bulk/file-0198.rs");
    await page.getByRole("button", { name: "Previous page", exact: true }).click();
    await page.getByText("1–100 of 2,001 entries", { exact: true }).waitFor();
    await page.locator("#nav-back").click();
    await page.getByText("101–200 of 2,001 entries", { exact: true }).waitFor();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "bulk/file-0199.rs");
    await paint(page);
    const restored = await page.evaluate(() => vizziMetrics.lastFrame.camera);
    for (const key of ["x", "y", "scale"]) assert.ok(Math.abs(saved[key] - restored[key]) < 0.01, `camera ${key} restored`);
    await page.evaluate(() => {
      const filter = document.getElementById("filter");
      filter.value = ".rs";
      filter.dispatchEvent(new Event("input", { bubbles: true }));
      document.querySelector("#file-list button[aria-current=true]").click();
    });
    await page.getByText("1–100 of 2,002 matches", { exact: true }).waitFor();
    await page.getByRole("searchbox", { name: "Find a file" }).fill("target.rs");
    await page.getByRole("button", { name: "z-late/target.rs", exact: true }).click();
    await page.waitForFunction(() => document.getElementById("file-title").textContent === "z-late/target.rs");
    await page.getByRole("searchbox", { name: "Find a file" }).fill("");
    await page.getByText("1–100 of 2,001 entries", { exact: true }).waitFor();
    await page.getByRole("button", { name: "Next page", exact: true }).click();
    await page.getByRole("button", { name: "bulk/file-0199.rs", exact: true }).waitFor();
    await page.getByRole("button", { name: "Previous page", exact: true }).click();
    await page.getByRole("button", { name: "bulk/file-0099.rs", exact: true }).waitFor();
    await page.getByRole("button", { name: "Next page", exact: true }).click();
    await page.getByRole("button", { name: "bulk/file-0199.rs", exact: true }).waitFor();
    const legend = page.locator("#map-legend");
    const rust = legend.getByRole("button", { name: "Rust", exact: true });
    await rust.click();
    await page.getByText("1–100 of 2,001 entries", { exact: true }).waitFor();
    await page.getByRole("button", { name: "Repository root", exact: true }).click();
    const documentation = legend.getByRole("button", { name: "Documentation", exact: true });
    await documentation.click();
    await page.getByRole("button", { name: "Open directory z-late, 3 files", exact: true }).waitFor();
    await rust.click();
    const filteredDirectory = page.getByRole("button", { name: "Open directory z-late, 2 files", exact: true });
    await filteredDirectory.waitFor();
    assert.equal(await page.getByRole("button", { name: /Open directory bulk/ }).count(), 0);
    await filteredDirectory.click();
    await page.locator("#file-list").getByRole("button", { name: "z-late/guide.markdown", exact: true }).waitFor();
    assert.deepEqual(await page.locator("#file-list button").allTextContents(), ["z-late/README.md", "z-late/guide.markdown"]);
    await page.getByRole("button", { name: "Clear filters" }).click();
    await page.locator("#file-list").getByRole("button", { name: "z-late/Cargo.toml", exact: true }).waitFor();

    await page.getByRole("button", { name: "Repository root", exact: true }).click();
    await rust.click();
    await documentation.click();
    await page.getByRole("button", { name: "Open directory bulk, 2,001 files", exact: true }).click();
    await page.getByText("1–100 of 2,001 entries", { exact: true }).waitFor();
    assert.equal(await documentation.getAttribute("aria-pressed"), "true");
    await documentation.press("Space");
    await documentation.waitFor({ state: "hidden" });
    assert.equal(await rust.evaluate(button => button === document.activeElement), true,
      "Removing a type absent from the open directory moves focus to a remaining filter");
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await localHost?.close();
    await fixture.close();
  }
});

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
    await page.getByText("Accessible file list",{exact:true}).click();
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
    await page.getByText("Accessible file list",{exact:true}).click();
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
    assert.match(await page.locator("#tooltip").textContent(), /Other files · Source only/);
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
    await page.getByText("Accessible file list",{exact:true}).click();
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
    await page.getByText("Accessible file list",{exact:true}).click();
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
    await page.goto(host.url+"/explore");
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
    await page.getByText("Accessible file list",{exact:true}).click();
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
    await session.page.getByText("Accessible file list",{exact:true}).click();
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
    await page.getByText("Accessible file list",{exact:true}).click();
    await page.getByRole("button", { name: "src/b.rs", exact: true }).click();
    await page.getByRole("button", { name: /^Reveal function beta starting at Line/ }).click();
    await page.waitForFunction(() => document.querySelector('#outline [aria-current="true"]')?.textContent.startsWith("beta"));
    const before = await page.locator("#file-title").textContent();
    const port = new URL(host.url).port;
    await host.close();
    const alert = page.locator("#error");
    await alert.getByText(/Live updates disconnected/).waitFor();
    const alertLayers = await page.getByRole("separator").evaluateAll(splitters => {
      const alert = document.getElementById("error");
      const { top, height } = alert.getBoundingClientRect();
      return splitters.map(splitter => {
        const { left, width } = splitter.getBoundingClientRect();
        return alert.contains(document.elementFromPoint(left + width / 2, top + height / 2));
      });
    });
    assert.equal(await page.locator("#file-title").textContent(), before);
    await writeFile(join(input.root, "src/b.rs"), "// changed while disconnected\nfn beta() {}");
    host = await startHost(input, 4, port);
    await alert.waitFor({ state: "hidden", timeout: 5000 });
    await page.waitForFunction(() => document.getElementById("source").textContent.includes("changed while disconnected"));
    assert.ok((await page.locator("#outline [aria-current=true]").textContent()).startsWith("beta"));
    assert.equal(session.requests.filter(path => path === "/api/status").length, 0);
    assert.ok(session.requests.filter(path => path === "/api/events").length >= 2);
    assert.deepEqual(alertLayers, [true, true], "the disconnect alert should cover both pane separators");
  } finally { await session.close(); }
});

test("source text stays text, and syntax/encoding limits are visible", async () => {
  const session = await openWorkspace(chromium, host);
  const { page } = session;
  try {
    assert.equal(await page.locator("#source img").count(), 0);
    assert.equal(await page.evaluate(() => window.injected), undefined);
    await page.getByText("Accessible file list",{exact:true}).click();
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

test("JavaScript and TypeScript outlines and written calls work in the captured workspace", async () => {
  const fixture = await workspace({
    files: {
      "package.json": '{"name":"@demo/app"}',
      "format/package.json": '{"name":"@demo/format"}',
      "start.ts": 'import { renderCard } from "./view";\nexport function start(value: number) { return renderCard!(value); }\n',
      "view.tsx": 'import { formatCard } from "./format/format.mjs";\nexport const renderCard = ((value: number) => <Card title={(formatCard)(value)} />);\n',
      "format/format.mjs": "export function formatCard(value) { return String(value); }\n",
      "ui/node_modules/dep/fake.ts": "export function renderCard() {}\n",
    },
  });
  let live, session;
  try {
    live = await startHost(fixture);
    session = await openWorkspace(chromium, live);
    const { page } = session;
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.getByRole("button", { name: "view.tsx", exact: true }).click();
    await page.locator("#outline button[data-declaration]").filter({ hasText: "renderCard" }).waitFor();
    assert.match(await page.locator("#coverage").textContent(), /tsx: syntax parsed/);
    assert.equal(await page.locator("#source .syntax-attribute").textContent(), "title");
    assert.match(await page.locator("#counts").textContent(), /5 files/);

    await page.goto(live.url+"/workspace");
    await page.waitForFunction(() => document.querySelector("#visual-source-status").textContent.includes("ready"));
    await page.getByRole("button", { name: "Add bubble", exact: true }).click();
    await page.getByLabel("Find a function", { exact: true }).fill("start");
    await page.getByRole("button", { name: "Find functions", exact: true }).click();
    await page.locator('[data-add-symbol][data-name="start"]').click();
    const start = page.locator('[data-view][data-name="start"]');
    await start.getByRole("button", { name: "Calls", exact: true }).click();
    await page.getByRole("button", { name: "renderCard · line 2", exact: true }).click();
    const card = page.locator('[data-view][data-name="renderCard"]');
    await card.locator(".code-line").first().waitFor();
    await card.getByRole("button", { name: "Calls", exact: true }).click();
    await page.getByRole("button", { name: "formatCard · line 2", exact: true }).click();
    const format = page.locator('[data-view][data-name="formatCard"]');
    await format.locator(".code-line").first().waitFor();
    assert.equal(await page.locator("[data-view]").count(), 3);
    const candidates = await page.locator('#workspace-links [data-kind="candidate"]').evaluateAll(
      links => links.map(link => link.getAttribute("aria-label"))
    );
    assert.equal(candidates.length, 2);
    assert.match(candidates[0], /renderCard! → renderCard/);
    assert.match(candidates[1], /\(formatCard\) → formatCard/);
    const packageId = await start.getAttribute("data-module-id");
    assert.ok(packageId);
    assert.equal(await card.getAttribute("data-module-id"), packageId);
    assert.equal(await start.locator(".module-tag").textContent(), "@demo/app");
    assert.equal(await card.locator(".module-tag").textContent(), "@demo/app");
    assert.equal(await format.locator(".module-tag").textContent(), "@demo/format");
    assert.notEqual(await format.getAttribute("data-module-id"), packageId);
    await writeFile(join(fixture.root, "package.json"), '{"name":"@demo/renamed"}');
    await page.waitForFunction(() =>
      document.querySelector('[data-name="start"] .module-tag')?.textContent === "@demo/renamed"
      && document.querySelector('[data-name="renderCard"] .module-tag')?.textContent === "@demo/renamed"
    );
    assert.equal(await start.getAttribute("data-module-id"), packageId);
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await live?.close();
    await fixture.close();
  }
});

test("Java source outlines and call trails work without a language server", async () => {
  const fixture = await workspace({
    files: {
      "Caller.java": "class Caller {\n    int run() {\n        var pair = new Pair(1, 2);\n        return Counter.seed()\n            .next()\n            .finish();\n    }\n}\n",
      "Counter.java": "class Counter {\n    static Counter seed() { return new Counter(); }\n    Counter next() { return this; }\n    int finish() { return 1; }\n}\n",
      "Pair.java": "record Pair(int x, int y) {}\n",
    },
  });
  let live, session;
  try {
    live = await startHost(fixture);
    session = await openWorkspace(chromium, live);
    const { page } = session;
    await page.getByText("Accessible file list", { exact: true }).click();
    await page.getByRole("button", { name: "Caller.java", exact: true }).click();
    await page.locator("#outline button[data-declaration]").filter({ hasText: "run" }).waitFor();
    assert.match(await page.locator("#coverage").textContent(), /java: syntax parsed/);
    assert.match(await page.locator("#source .syntax-keyword").first().textContent(), /class/);

    await page.goto(live.url+"/workspace");
    await page.waitForFunction(() => document.querySelector("#visual-source-status").textContent.includes("ready"));
    await page.getByRole("button", { name: "Add bubble", exact: true }).click();
    await page.getByLabel("Find a function", { exact: true }).fill("run");
    await page.getByRole("button", { name: "Find functions", exact: true }).click();
    await page.locator('[data-add-symbol][data-name="run"]').click();
    const run = page.locator('[data-view][data-name="run"]');
    await run.getByRole("button", { name: "Calls", exact: true }).click();
    await run.getByRole("button", { name: "next · line 5", exact: true }).waitFor();
    await run.getByRole("button", { name: "finish · line 6", exact: true }).click();
    const finish = page.locator('[data-view][data-name="finish"]');
    await finish.locator(".code-line").first().waitFor();
    assert.match(await finish.textContent(), /return 1/);
    assert.deepEqual(await run.locator(".is-call").evaluateAll(lines => lines.map(line => line.dataset.line)), ["6"]);
    const link = page.locator('#workspace-links [data-kind="candidate"]');
    await link.waitFor({ state: "attached" });
    assert.match(await link.getAttribute("aria-label"), /→ Counter::finish/);
    await page.getByRole("button", { name: "Fit workspace", exact: true }).click();
    await finish.locator(".fragment-header").hover();
    await paint(page);
    assert.deepEqual(await run.locator(".is-linked-call").evaluateAll(lines => lines.map(line => line.dataset.line)), ["4", "5", "6"]);

    await run.getByRole("button", { name: "Calls", exact: true }).click();
    await run.getByRole("button", { name: "Pair · line 3", exact: true }).click();
    const pair = page.locator('[data-view][data-name="Pair"]');
    await pair.locator(".code-line").first().waitFor();
    assert.match(await pair.textContent(), /record Pair/);
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await live?.close();
    await fixture.close();
  }
});

test("Java package cues group calls and track captured module descriptors without JDT LS", async () => {
  const fixture = await workspace({
    files: {
      "module-info.java": "module example.app { exports demo; }\n",
      "demo/Caller.java": "package demo; class Caller { int startJava() { return Peer.finishJava(); } }\n",
      "demo/Peer.java": "package demo; class Peer { static int finishJava() { return 1; } }\n",
      "Loose.java": "package loose; class Loose { int standaloneJava() { return 2; } }\n",
    },
  });
  let host, session;
  try {
    host = await startHost(fixture);
    session = await openWorkspace(chromium, host);
    const { page } = session;
    await page.goto(host.url+"/workspace");
    await page.waitForFunction(() => document.querySelector("#visual-source-status").textContent.includes("ready"));
    const open = async name => {
      await page.getByRole("button", { name: "Add bubble", exact: true }).click();
      await page.getByLabel("Find a function", { exact: true }).fill(name);
      await page.getByRole("button", { name: "Find functions", exact: true }).click();
      await page.locator(`[data-add-symbol][data-name="${name}"]`).click();
      const bubble = page.locator(`[data-view][data-name="${name}"]`);
      await bubble.locator(".code-line").first().waitFor();
      return bubble;
    };
    const start = await open("startJava");
    await start.getByRole("button", { name: "Calls", exact: true }).click();
    await start.getByRole("button", { name: "finishJava · line 1", exact: true }).click();
    const finish = page.locator('[data-view][data-name="finishJava"]');
    await finish.locator(".code-line").first().waitFor();
    const id = await start.getAttribute("data-module-id");
    assert.ok(id);
    assert.equal(await finish.getAttribute("data-module-id"), id);
    assert.equal(await start.locator(".module-tag").textContent(), "example.app/demo");
    assert.equal(await finish.locator(".module-tag").textContent(), "example.app/demo");
    await start.locator(".fragment-header").focus();
    await page.keyboard.press("Tab");
    await paint(page);
    assert.equal(await page.locator(".fragment.is-module-peer").count(), 2);
    assert.equal(await page.locator("#overview-map .is-module-peer").count(), 2);
    await page.locator("#workspace-settings > summary").click();
    await page.getByLabel("Module outlines", { exact: true }).check();
    await page.keyboard.press("Escape");
    await page.locator("#module-outlines .module-outline").waitFor({ state: "attached" });

    await writeFile(join(fixture.root, "module-info.java"), "// generated descriptor\rmodule example.\\u0072enamed { exports demo; }\r");
    await page.waitForFunction(() =>
      document.querySelector('[data-name="startJava"] .module-tag')?.textContent === "example.renamed/demo"
      && document.querySelector('[data-name="finishJava"] .module-tag')?.textContent === "example.renamed/demo"
    );
    assert.equal(await start.getAttribute("data-module-id"), id);
    await page.reload();
    await start.locator(".code-line").first().waitFor();
    await finish.locator(".code-line").first().waitFor();
    assert.equal(await start.getAttribute("data-module-id"), id);
    assert.equal(await finish.locator(".module-tag").textContent(), "example.renamed/demo");

    const standalone = await open("standaloneJava");
    assert.equal(await standalone.locator(".module-tag").textContent(), "loose");
    assert.match(await standalone.locator(".module-tag").getAttribute("title"), /Java package\nRoot: \.$/);
    assert.notEqual(await standalone.getAttribute("data-module-id"), id);
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await host?.close();
    await fixture.close();
  }
});

const managedJava = process.env.VIZZI_JDTLS;
const gradleWrapperUrl = process.env.VIZZI_TEST_GRADLE_WRAPPER_URL;
const gradleJavaHome = process.env.VIZZI_TEST_GRADLE_JAVA_HOME;

test("the workspace action imports a Gradle Java classpath explicitly", {
  skip: !managedJava || !(gradleWrapperUrl || process.env.GRADLE_HOME),
}, async () => {
  const fixture = await workspace({
    files: {
      "settings.gradle": "rootProject.name = 'vizzi_browser'\n",
      "build.gradle": "plugins { id 'java' }\nsourceSets.main.java.setSrcDirs(['sources'])\n",
      ...(gradleWrapperUrl && { "gradle/wrapper/gradle-wrapper.properties": `distributionUrl=${gradleWrapperUrl}\n` }),
      ...(gradleJavaHome && { "gradle.properties": `org.gradle.java.home=${gradleJavaHome.replaceAll("\\", "\\\\")}\n` }),
      "sources/demo/Caller.java": "package demo; class Caller { int run() { return Counter.next(); } }\n",
      "sources/demo/Counter.java": "package demo; class Counter { static int next() { return 1; } }\n",
      "script.py": "def python_leaf(): pass\ndef invoke():\n" + "    python_leaf()\n".repeat(10),
    },
    sourceArguments: ["--jdtls", managedJava],
  });
  let host, context;
  try {
    host = await startHost(fixture);
    context = await chromium.newContext();
    const page = await context.newPage();
    const imports = [];
    page.on("request", request => {
      if (request.method() === "POST" && new URL(request.url()).pathname === "/api/java-project") {
        imports.push(request);
      }
    });
    await page.goto(host.url+"/workspace");
    await page.getByText("Workspace", { exact: true }).click();
    const action = page.getByRole("button", { name: "Import Gradle classpath", exact: true });
    await action.waitFor();
    assert.equal(imports.length, 0);
    const rejected = await page.request.post(host.url + "/api/java-project", { data: { snapshot: "foreign" } });
    assert.equal(rejected.status(), 400);
    assert.match(await rejected.text(), /same-origin/);
    const { snapshot } = await (await page.request.get(host.url + "/api/java-project")).json();
    const inventory = await page.request.post(host.url + "/api/structure", {
      data: { snapshot, query: { kind: "inventory", area: "script.py", search: "python_leaf", offset: 0, functions_only: true } },
    });
    assert.equal(inventory.status(), 200);
    const [{ symbol }] = (await inventory.json()).items;
    const refRequest = { snapshot, symbol: symbol.id, kind: "references" };
    const first = await page.request.post(host.url + "/api/references", { data: refRequest });
    assert.equal(first.status(), 200);
    const { query, next } = await first.json();
    assert.equal(next, 8);

    await page.route("**/api/java-project", route => route.fulfill({
      status: 503, json: { message: "Fixture import failed." },
    }), { times: 1 });
    await action.click();
    await page.getByText("Fixture import failed.", { exact: true }).waitFor();
    assert.equal(await page.locator("#visual-notice").isHidden(), true);
    await page.getByText("Workspace", { exact: true }).click();
    const completed = page.waitForResponse(response =>
      response.request().method() === "POST" && new URL(response.url()).pathname === "/api/java-project",
      { timeout: 120000 },
    );
    await action.click();
    const result = await completed;
    assert.equal(result.status(), 200, await result.text());
    await page.getByText("Java project classpath imported.", { exact: true }).waitFor({ timeout: 60000 });
    assert.equal(imports.length, 2);
    const nextPage = await page.request.post(host.url + "/api/references", {
      data: { ...refRequest, query, offset: next },
    });
    assert.equal(nextPage.status(), 200);
    assert.equal((await nextPage.json()).items.length, 2);

    await page.getByRole("button", { name: "Add bubble", exact: true }).click();
    await page.getByLabel("Find a function", { exact: true }).fill("run");
    await page.getByRole("button", { name: "Find functions", exact: true }).click();
    await page.locator('[data-add-symbol][data-name="run"]').click();
    const caller = page.locator('[data-view][data-name="run"]');
    await caller.getByRole("button", { name: "Calls", exact: true }).click();
    await caller.getByRole("button", { name: "next · line 1", exact: true }).click();
    await page.locator('[data-view][data-name="next"] .code-line').first().waitFor();
    assert.equal(await page.locator("#workspace-links .link-stroke").getAttribute("data-kind"), "definition");
  } finally {
    await context?.close();
    await host?.close();
    await fixture.close();
  }
});

test("managed Java resolves an overload and its references across source roots", {
  skip: !managedJava,
}, async () => {
  const fixture = await workspace({
    files: {
      "src/main/java/demo/Engine.java": `package demo;
class Engine {
    static int speak(int value) { return value; }
    static int speak(String value) { return value.length(); }
}
`,
      "src/test/java/demo/Caller.java": `package demo;
class Caller { int run() { return Engine.speak("hello") + "abc".length(); } }
`,
      "pom.xml": "Build metadata is outside the source-root profile.",
    },
    sourceArguments: [
      "--jdtls", managedJava,
      "--java-source-root", "src/main/java",
      "--java-source-root", "src/test/java",
    ],
  });
  let live, session;
  try {
    live = await startHost(fixture);
    session = await openWorkspace(chromium, live);
    const { page } = session;
    await page.goto(live.url+"/workspace");
    await page.waitForFunction(() => document.querySelector("#visual-source-status").textContent.includes("ready"));
    await page.getByRole("button", { name: "Add bubble", exact: true }).click();
    await page.getByLabel("Find a function", { exact: true }).fill("run");
    await page.getByRole("button", { name: "Find functions", exact: true }).click();
    await page.locator('[data-add-symbol][data-name="run"]').click();
    const caller = page.locator('[data-view][data-name="run"]');
    await caller.getByRole("button", { name: "Calls", exact: true }).click();
    await caller.getByRole("button", { name: "speak · line 2", exact: true }).click();
    const target = page.locator('[data-view][data-name="speak"]');
    await target.locator(".code-line").first().waitFor();
    assert.match(await target.locator(".code-pane").textContent(), /String value/);
    assert.equal(await page.locator("#workspace-links .link-stroke").getAttribute("data-kind"), "definition");
    await target.getByRole("button", { name: "Actions for speak bubble", exact: true }).click();
    await page.getByRole("menuitem", { name: "Find references", exact: true }).click();
    const stack = page.locator("[data-reference-stack]");
    await stack.getByRole("status").filter({ hasText: "Language references" }).waitFor();
    assert.equal(await stack.locator("[data-reference-row]").count(), 1);
    assert.match(await stack.locator("[data-reference-row]").textContent(), /Caller\.java/);
    await caller.getByRole("button", { name: "Calls", exact: true }).click();
    await caller.getByRole("button", { name: "length · line 2", exact: true }).click();
    const length = page.locator('[data-view][data-name="length"]');
    await length.locator(".code-line").first().waitFor();
    assert.match(await length.locator(".fragment-heading").textContent(), /java\.base.*String\.java/);
    assert.match(await length.locator(".code-pane").textContent(), /coder\(\)/);
    assert.equal(await length.getByRole("button", { name: /^Open in/ }).isDisabled(), true);

    await caller.getByRole("button", { name: "Close bubble", exact: true }).press("Enter");
    await target.getByRole("button", { name: "Close bubble", exact: true }).press("Enter");
    const port = Number(new URL(live.url).port);
    await live.close();
    live = await startHost(fixture, 4, port);
    await page.reload();
    await length.locator(".code-line").first().waitFor();
    assert.equal(await page.locator('[data-view]').count(), 1);
    await length.getByRole("button", { name: "Actions for coder", exact: true }).press("Control+Enter");
    const coder = page.locator('[data-view][data-name="coder"]');
    await coder.locator(".code-line").first().waitFor();
    assert.match(await coder.locator(".fragment-heading").textContent(), /java\.base.*String\.java/);
    assert.equal(await page.locator("#workspace-links .link-stroke").getAttribute("data-kind"), "definition");
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await live?.close();
    await fixture.close();
  }
});

const managedTypeScript = process.env.VIZZI_TYPESCRIPT_SERVER;

test("managed TypeScript opens installed package source beside its caller", {
  skip: !managedTypeScript || process.platform === "win32",
}, async () => {
  const fixture = await workspace({
    files: {
      "entry.ts": "import { packageEntry } from 'fixture-pkg';\nexport function launch() { return packageEntry(); }\n",
      "node_modules/fixture-pkg/package.json": '{"name":"fixture-pkg","version":"2.4.0","exports":"./index.ts"}',
      "node_modules/fixture-pkg/index.ts": 'import { finish } from "./helper";\nexport function packageEntry() { return finish(); }\n',
      "node_modules/fixture-pkg/helper.ts": "export function finish() { return 7; }\n",
    },
    sourceArguments: ["--typescript-server", managedTypeScript],
  });
  let live, session;
  try {
    live = await startHost(fixture);
    session = await openWorkspace(chromium, live);
    const { page } = session;
    assert.match(await page.locator("#counts").textContent(), /1 files?/);
    await page.goto(live.url+"/workspace");
    await page.waitForFunction(() => document.querySelector("#visual-source-status").textContent.includes("ready"));
    await page.getByRole("button", { name: "Add bubble", exact: true }).click();
    await page.getByLabel("Find a function", { exact: true }).fill("launch");
    await page.getByRole("button", { name: "Find functions", exact: true }).click();
    await page.locator('[data-add-symbol][data-name="launch"]').click();
    const launch = page.locator('[data-view][data-name="launch"]');
    await launch.getByRole("button", { name: "Calls", exact: true }).click();
    await page.getByRole("button", { name: "packageEntry · line 2", exact: true }).click();
    const entry = page.locator('[data-view][data-name="packageEntry"]');
    await entry.locator(".code-line").first().waitFor();
    assert.match(await entry.locator(".fragment-heading small").textContent(), /Dependency · fixture-pkg 2\.4\.0/);
    assert.equal(await entry.getByRole("button", { name: "Save location", exact: true }).isDisabled(), true);
    await entry.getByRole("button", { name: "Calls", exact: true }).click();
    await page.getByRole("button", { name: "finish · line 2", exact: true }).click();
    const finish = page.locator('[data-view][data-name="finish"]');
    await finish.locator(".code-line").first().waitFor();
    assert.match(await finish.locator(".fragment-heading small").textContent(), /fixture-pkg 2\.4\.0/);
    assert.equal(await page.locator("[data-view]").count(), 3);
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await live?.close();
    await fixture.close();
  }
});

test("managed TypeScript follows project aliases and exposes semantic references", {
  skip: !managedTypeScript,
}, async () => {
  const fixture = await workspace({
    files: {
      "tsconfig.json": JSON.stringify({ compilerOptions: {
        module: "ESNext", moduleResolution: "Bundler", jsx: "preserve",
        paths: { "@service": ["./service.ts"] },
      }}),
      "entry.tsx": "import { create as select } from '@service';\nexport function launch() { return <div>{select().run()}</div>; }\n",
      "service.ts": 'export class Service { "run"() { return 1; } }\nexport function create() { return new Service(); }\n',
      "decoy.ts": "export class Other { run() { return 2; } }\n",
    },
    sourceArguments: ["--typescript-server", managedTypeScript],
  });
  let live, session;
  try {
    live = await startHost(fixture);
    session = await openWorkspace(chromium, live);
    const { page } = session;
    await page.goto(live.url+"/workspace");
    await page.waitForFunction(() => document.querySelector("#visual-source-status").textContent.includes("ready"));
    await page.getByRole("button", { name: "Add bubble", exact: true }).click();
    await page.getByLabel("Find a function", { exact: true }).fill("launch");
    await page.getByRole("button", { name: "Find functions", exact: true }).click();
    await page.locator('[data-add-symbol][data-name="launch"]').click();
    await page.locator('[data-view][data-name="launch"]').getByRole("button", { name: "Calls", exact: true }).click();
    await page.getByRole("button", { name: "run · line 2", exact: true }).click();
    const method = page.locator('[data-view][data-name="run"]');
    await method.locator(".code-line").first().waitFor();
    assert.match(await method.locator(".fragment-heading").textContent(), /Service::run/);
    assert.equal(await page.locator('#workspace-links .link-stroke').getAttribute("data-kind"), "definition");
    await method.getByRole("button", { name: "Actions for run bubble", exact: true }).click();
    await page.getByRole("menuitem", { name: "Find references", exact: true }).click();
    const references = page.locator("[data-reference-stack]");
    await references.getByRole("status").filter({ hasText: "Language references" }).waitFor();
    assert.match(await references.locator("[data-reference-row]").textContent(), /entry\.tsx/);
    assert.deepEqual(session.errors, []);
  } finally {
    await session?.close();
    await live?.close();
    await fixture.close();
  }
});
