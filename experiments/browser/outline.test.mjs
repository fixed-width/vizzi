import { test, before, after, afterEach } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { browser, paint } from "./support.mjs";
import { INSTRUMENTER_CONSTANTS as stryker } from "@stryker-mutator/api/core";

let chromium, moduleUrl, css;
const errors = [];
before(async () => {
  chromium = await browser();
  moduleUrl = `data:text/javascript;base64,${Buffer.from(await readFile("crates/vizzi/web/outline.js", "utf8")).toString("base64")}`;
  css = await readFile("crates/vizzi/web/app.css", "utf8");
});
after(async () => chromium?.close());
afterEach(() => { assert.deepEqual(errors.splice(0), []); });

async function fixture(count = 50000, initialize = true) {
  const page = await chromium.newPage();
  page.on("pageerror", error => errors.push(error.message));
  await page.evaluate(({ namespace, field, active, hitCount, hitLimit }) => {
    globalThis[namespace] = { [field]: active, [hitCount]: 0, [hitLimit]: 1000000 };
  }, { namespace: stryker.NAMESPACE, field: stryker.ACTIVE_MUTANT, active: process.env[stryker.ACTIVE_MUTANT_ENV_VARIABLE], hitCount: stryker.HIT_COUNT, hitLimit: stryker.HIT_LIMIT });
  await page.setContent(`<style>${css}</style><nav class="outline" style="width:240px;height:400px;flex-shrink:0"><h3>Declarations</h3><div id="outline"></div></nav><button id="outside">Outside</button>`);
  await page.evaluate(async ({ url, count, initialize }) => {
    const { createOutline } = await import(url);
    window.activations = [];
    window.widget = createOutline({ viewport: document.getElementById("outline"), select: fact => window.activations.push(fact.id) });
    window.original = Array.from({ length: count }, (_, index) => ({ id: `f${index}`, name: `function_${index}`, kind: "function", start_line: index + 1, end_line: index + 1 }));
    window.source = { id: "file", version: "v1", outlines: [{ declarations: window.original }] };
    if (initialize) window.widget.update(window.source, null);
  }, { url: moduleUrl, count, initialize });
  await paint(page); return page;
}

async function checkGeometry(page) {
  const result = await page.evaluate(() => {
    const viewport = document.getElementById("outline"), bounds = viewport.getBoundingClientRect();
    const rows = [...viewport.querySelectorAll("[role=listitem]")];
    return { role: viewport.getAttribute("role"), label: viewport.getAttribute("aria-label"), tabIndex: viewport.tabIndex,
      misplaced: rows.filter(row => row.offsetTop !== (Number(row.getAttribute("aria-posinset")) - 1) * 48).length,
      focusedVisible: !viewport.contains(document.activeElement) || (document.activeElement.getBoundingClientRect().top >= bounds.top && document.activeElement.getBoundingClientRect().bottom <= bounds.bottom),
      tabStops: [...viewport.querySelectorAll("button")].filter(button => button.tabIndex >= 0).length };
  });
  assert.equal(result.role, "list"); assert.equal(result.label, "Declarations"); assert.equal(result.tabIndex, -1);
  assert.equal(result.misplaced, 0); assert.equal(result.focusedVisible, true); assert.equal(result.tabStops, 1);
}

test("logical keyboard navigation reaches offscreen rows with bounded DOM and full list positions", async () => {
  const page = await fixture();
  try {
    assert.ok(await page.locator("#outline button[data-declaration]").count() <= 200);
    assert.ok(await page.locator("#outline button[data-declaration]").count() <= await page.locator("#outline").evaluate(node => Math.ceil(node.clientHeight / 48) + 18));
    for (let index = 0; index < 7; index++) assert.equal(await page.locator(`#outline button[data-declaration][data-declaration="f${index}"]`).count(), 1);
    assert.equal(await page.locator("#outline [role=listitem]").first().getAttribute("aria-posinset"), "1");
    await page.locator("#outline button[data-declaration]").first().focus();
    await page.evaluate(() => {
      window.keyDefaults = [];
      document.getElementById("outline").addEventListener("keydown", event => window.keyDefaults.push({ key: event.key, prevented: event.defaultPrevented }));
    });
    await page.keyboard.press("End");
    await checkGeometry(page);
    assert.ok((await page.evaluate(() => document.activeElement.textContent)).startsWith("function_49999"));
    assert.equal(await page.locator('#outline button[data-declaration][data-declaration="f49999"]').locator("..").getAttribute("aria-setsize"), "50000");
    await page.keyboard.press("ArrowDown"); await page.keyboard.press("Enter");
    assert.deepEqual(await page.evaluate(() => window.activations), ["f49999"]);
    await page.keyboard.press("ArrowUp"); await page.keyboard.press("Space");
    await checkGeometry(page);
    assert.deepEqual(await page.evaluate(() => window.activations), ["f49999", "f49998"]);
    await page.keyboard.press("PageUp");
    const position = await page.evaluate(() => Number(document.activeElement.parentElement.getAttribute("aria-posinset")));
    const step = await page.locator("#outline").evaluate(node => Math.floor(node.clientHeight / 48));
    assert.equal(position, 49999 - step); await checkGeometry(page); await page.keyboard.press("PageDown");
    assert.ok((await page.evaluate(() => document.activeElement.textContent)).startsWith("function_49998"));
    await page.keyboard.press("Home"); await page.keyboard.press("ArrowUp");
    assert.ok((await page.evaluate(() => document.activeElement.textContent)).startsWith("function_0"));
    await checkGeometry(page);
    assert.match(await page.locator('#outline button[data-declaration][data-declaration="f0"]').getAttribute("title"), /function function_0 · line 1/);
    assert.match(await page.locator('#outline button[data-declaration][data-declaration="f0"]').textContent(), /function · 1/);
    assert.equal(await page.locator('#outline button[data-declaration][tabindex="0"]').count(), 1);
    const defaults = await page.evaluate(() => window.keyDefaults);
    assert.ok(defaults.filter(event => ["ArrowDown", "ArrowUp", "PageUp", "PageDown", "Home", "End"].includes(event.key)).every(event => event.prevented));
    assert.ok(defaults.filter(event => ["Enter", " "].includes(event.key)).every(event => !event.prevented));
  } finally { await page.close(); }
});

test("same-version selections reuse the index; background changes preserve scroll anchor and focused identity", async () => {
  const page = await fixture(1000);
  try {
    await page.locator("#outline").evaluate(node => { node.scrollTop = 500 * 48 + 7; }); await paint(page);
    await page.evaluate(() => window.widget.update({ id: "file", version: "v1", get outlines() { throw new Error("unchanged source must not be rescanned"); } }, "f505"));
    assert.ok((await page.locator('#outline [aria-current="true"]').textContent()).startsWith("function_505"));
    await page.locator('#outline button[data-declaration][data-declaration="f505"]').focus();
    assert.equal(await page.locator('#outline button[data-declaration][tabindex="0"]').getAttribute("data-declaration"), "f505");
    await page.keyboard.press("ArrowDown");
    assert.equal(await page.evaluate(() => document.activeElement.dataset.declaration), "f506");
    await page.keyboard.press("ArrowUp");
    const before = await page.locator("#outline").evaluate(node => node.scrollTop);
    await page.evaluate(() => window.widget.update({ id: "file", version: "v2", outlines: [{ declarations: [{ ...window.original[0], id: "insert", name: "inserted" }, ...window.original] }] }, "f510"));
    assert.equal(await page.locator("#outline").evaluate(node => node.scrollTop), before + 48);
    assert.ok((await page.evaluate(() => document.activeElement.textContent)).startsWith("function_505"));
    assert.equal(await page.locator('#outline button[data-declaration][tabindex="0"]').getAttribute("data-declaration"), "f505");
    await page.evaluate(() => window.widget.update({ id: "file", version: "v3", outlines: [{ declarations: window.original.filter(fact => !["f505", "f506"].includes(fact.id)) }] }, "f505"));
    assert.equal(await page.locator('#outline [aria-current="true"]').count(), 0);
    assert.ok((await page.evaluate(() => document.activeElement.textContent)).startsWith("function_507"));
    await page.keyboard.press("End");
    await page.evaluate(() => window.widget.update({ id: "file", version: "v4", outlines: [{ declarations: window.original.slice(0, 997) }] }, null));
    assert.equal(await page.evaluate(() => document.activeElement.dataset.declaration), "f996");
  } finally { await page.close(); }
});

test("scrolling and resize keep a bounded window while preserving focused controls", async () => {
  const page = await fixture();
  try {
    await page.locator("#outline button[data-declaration]").first().focus();
    await page.locator("#outline").evaluate(node => { node.scrollTop = 1000 * 48 + 7; }); await paint(page);
    assert.equal(await page.evaluate(() => document.activeElement.dataset.declaration), "f0");
    assert.ok(await page.locator("#outline button[data-declaration]").count() <= 200);
    await page.locator("nav").evaluate(node => { node.style.height = "20000px"; }); await paint(page);
    assert.ok(await page.locator("#outline button[data-declaration]").count() <= 200);
    assert.equal(await page.locator('#outline button[data-declaration][data-declaration="f1182"]').count(), 1);
    await page.locator("#outline").evaluate(node => { node.scrollTop = node.scrollHeight; }); await paint(page);
    assert.equal(await page.locator('#outline button[data-declaration][data-declaration="f49999"]').count(), 1);
    await page.keyboard.press("End"); assert.equal(await page.evaluate(() => window.scrollY), 0);
    await page.evaluate(() => window.widget.update({ id: "file", version: "v2", outlines: [{ declarations: window.original.slice(0, -1) }] }, null));
    assert.equal(await page.evaluate(() => document.activeElement.dataset.declaration), "f49998"); assert.equal(await page.evaluate(() => window.scrollY), 0);
  } finally { await page.close(); }
});

test("initial and emptied lists stay small; minimal reveal and background updates do not steal focus", async () => {
  const page = await fixture(50000, false);
  try {
    assert.match(await page.locator("#outline").textContent(), /No declarations/);
    await page.evaluate(() => window.widget.update(window.source, null));
    await page.locator("#outside").focus();
    const height = await page.locator("#outline").evaluate(node => node.clientHeight);
    await page.evaluate(() => window.widget.update(window.source, "f10", true));
    assert.equal(await page.locator("#outline").evaluate(node => node.scrollTop), 11 * 48 - height);
    assert.equal(await page.evaluate(() => document.activeElement.id), "outside");
    await page.locator("#outline").evaluate(node => { node.scrollTop = 1000 * 48; }); await paint(page);
    await page.evaluate(() => window.widget.update(window.source, "f1003", true));
    assert.equal(await page.locator("#outline").evaluate(node => node.scrollTop), 1000 * 48);
    await page.evaluate(() => window.widget.update(window.source, "f49999"));
    assert.equal(await page.locator("#outline").evaluate(node => node.scrollTop), 1000 * 48);
    await page.evaluate(() => window.widget.update({ id: "file", version: "v2", outlines: [{ declarations: [{ ...window.original[0], id: "insert" }, ...window.original] }] }, "f1003"));
    assert.equal(await page.evaluate(() => document.activeElement.id), "outside");
    assert.equal(await page.locator('#outline button[data-declaration][tabindex="0"]').getAttribute("data-declaration"), "f1000");
    await page.evaluate(() => window.widget.update({ id: "other", version: "v1", outlines: [{ declarations: window.original.map(fact => ({ ...fact, id: "other-" + fact.id })) }] }, null));
    assert.equal(await page.locator("#outline").evaluate(node => node.scrollTop), 0);
    await page.locator("#outline button[data-declaration]").first().focus();
    await page.evaluate(() => window.widget.update(null, null));
    assert.equal(await page.evaluate(() => document.activeElement.id), "outline");
    assert.ok(await page.locator("#outline").evaluate(node => node.scrollHeight <= node.clientHeight));
  } finally { await page.close(); }
});

test("backward focus recovery finds surviving earlier rows and collapsed viewports remain valid", async () => {
  const page = await fixture(20);
  try {
    await page.locator("#outline button[data-declaration]").first().focus(); await page.keyboard.press("End");
    await page.evaluate(() => window.widget.update({ id: "file", version: "v2", outlines: [{ declarations: [{ ...window.original[1], id: "inserted" }, window.original[0]] }] }, null));
    assert.equal(await page.evaluate(() => document.activeElement.dataset.declaration), "f0");
    await page.locator("#outside").focus();
    await page.locator("nav").evaluate(node => { node.style.height = "0px"; }); await paint(page);
    await page.locator("#outline").evaluate(node => { node.scrollTop = node.scrollHeight; }); await paint(page);
    assert.equal(await page.locator('#outline button[data-declaration][tabindex="0"]').count(), 1);
    await page.locator("nav").evaluate(node => { node.style.height = "400px"; }); await paint(page);
    await page.locator("#outline").click({ position: { x: 10, y: 300 } });
    assert.deepEqual(await page.evaluate(() => window.activations), []);
  } finally { await page.close(); }
});

test("empty, removed, unversioned, and replacement sources do not retain stale selections or interpret names as HTML", async () => {
  const page = await fixture(4);
  try {
    await page.evaluate(() => window.widget.update({ id: "file", outlines: [{ declarations: [{ ...window.original[0], name: "<img src=x onerror='window.injected=true'>" }] }] }, "f0", true));
    assert.equal(await page.locator("#outline img").count(), 0); assert.equal(await page.evaluate(() => window.injected), undefined);
    await page.evaluate(() => window.widget.update({ id: "file", outlines: [{ declarations: [{ ...window.original[1], name: "replacement" }] }] }, "f1", true));
    assert.ok((await page.locator('#outline [aria-current="true"]').textContent()).includes("replacement"));
    await page.evaluate(() => window.widget.update(null, null));
    assert.equal(await page.locator("#outline button[data-declaration]").count(), 0); assert.match(await page.locator("#outline").textContent(), /No declarations/);
    await page.locator("#outline").press("ArrowDown");
    await page.evaluate(() => window.widget.update({ id: "other", version: "v1", outlines: [{ declarations: window.original }] }, "f3", true));
    assert.ok((await page.locator('#outline [aria-current="true"]').textContent()).startsWith("function_3"));
    assert.equal(await page.locator("#outline button[data-declaration]").count(), 4); assert.ok(!(await page.locator("#outline").textContent()).includes("No declarations"));
    await page.locator('#outline button[data-declaration][data-declaration="f3"]').focus();
    await page.evaluate(() => window.widget.update(null, null));
    assert.equal(await page.evaluate(() => document.activeElement.id), "outline");
    assert.ok(await page.locator("#outline").evaluate(node => node.scrollHeight <= node.clientHeight));
    await page.locator("#outline").click({ position: { x: 10, y: 200 } });
  } finally { await page.close(); }
});

test("scroll events coalesce within a frame and initial offscreen selection is revealed", async () => {
  const page = await fixture(50000, false);
  try {
    await page.evaluate(() => window.widget.update({ ...window.source, id: "another" }, "f49999", true));
    const visible = await page.evaluate(() => {
      const viewport = document.getElementById("outline").getBoundingClientRect(), selected = document.querySelector('#outline [aria-current=true]').getBoundingClientRect();
      return { visible: selected.top >= viewport.top && selected.bottom <= viewport.bottom, top: selected.top, bottom: selected.bottom, viewportTop: viewport.top, viewportBottom: viewport.bottom, scrollTop: document.getElementById("outline").scrollTop };
    });
    assert.equal(visible.visible, true, JSON.stringify(visible));
    await page.locator('#outline button[data-declaration][data-declaration="f49999"]').focus(); await checkGeometry(page);
    await page.evaluate(() => {
      const original = window.requestAnimationFrame; window.renderRequests = 0;
      window.requestAnimationFrame = callback => { window.renderRequests++; return original(callback); };
      for (let i = 0; i < 10; i++) document.getElementById("outline").dispatchEvent(new Event("scroll"));
      return new Promise(resolve => original(() => original(resolve)));
    });
    assert.equal(await page.evaluate(() => window.renderRequests), 1);
    await page.locator("nav").evaluate(node => { node.style.height = "400px"; });
    await page.locator("#outline").evaluate(node => { node.scrollTop = 1000 * 48; }); await page.locator("h3").click(); await paint(page);
    await page.locator("#outline").dispatchEvent("scroll"); await paint(page);
    assert.equal(await page.locator('#outline button[data-declaration][tabindex="0"]').getAttribute("data-declaration"), "f1000");
  } finally { await page.close(); }
});

test("removed scroll anchors retain position and offscreen empty focus does not jump the page", async () => {
  const page = await fixture(1000);
  try {
    await page.locator("#outside").focus();
    await page.locator("#outline").evaluate(node => { node.scrollTop = 500 * 48 + 7; }); await paint(page);
    const before = await page.locator("#outline").evaluate(node => node.scrollTop);
    await page.evaluate(() => window.widget.update({ id: "file", version: "v2", outlines: [{ declarations: window.original.filter(fact => fact.id !== "f500") }] }, null));
    assert.equal(await page.locator("#outline").evaluate(node => node.scrollTop), before);
    await page.locator("nav").evaluate(node => { node.style.marginTop = "1000px"; });
    await page.locator('#outline button[data-declaration][tabindex="0"]').evaluate(node => node.focus({ preventScroll: true }));
    assert.equal(await page.evaluate(() => window.scrollY), 0);
    await page.evaluate(() => window.widget.update(null, null));
    assert.equal(await page.evaluate(() => document.activeElement.id), "outline");
    assert.equal(await page.evaluate(() => window.scrollY), 0);
  } finally { await page.close(); }
});
