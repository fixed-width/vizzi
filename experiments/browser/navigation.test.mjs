import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { writeFile, unlink } from "node:fs/promises";
import { join } from "node:path";
import { buildHost, browser, workspace, startHost, openWorkspace, paint } from "./support.mjs";

let chromium;
before(() => { buildHost(); });
before(async () => { chromium = await browser(); });
after(async () => { await chromium?.close(); });
async function fixture() {
  const code = "fn start() {}\n" + "// reading context\n".repeat(300) + "fn anchor() {}\n" + "// following context\n".repeat(100);
  const input = await workspace({ files: { "aa.rs": code, "bb.rs": "fn contract() {}\n" } });
  const host = await startHost(input), session = await openWorkspace(chromium, host);
  const page = session.page;
  async function file(path) {
    await page.locator("#filter").fill(path);
    await page.waitForFunction(path => document.querySelector("#counts").textContent.startsWith("1 of ") && document.querySelector("#filter").value === path, path);
    if (!await page.locator("details").evaluate(node => node.open)) await page.locator("summary").click();
    await page.locator("#file-list").getByRole("button", { name: path, exact: true }).click();
    await page.waitForFunction(path => document.querySelector("#file-title").textContent === path, path);
  }
  async function anchor() {
    await file("aa.rs");
    await page.locator('#outline button[data-declaration]').filter({ hasText: "anchor" }).click();
    await page.waitForFunction(() => document.querySelector('#outline [aria-current="true"]')?.textContent.startsWith("anchor"));
    await page.locator("#source").evaluate(node => { node.scrollTop += 88; });
    await paint(page);
    return page.locator("#source").evaluate(node => node.scrollTop);
  }
  return { page, input, host, session, code, file, anchor, async close() { await session.close(); await host.close(); await input.close(); } };
}
test("toolbar and browser history restore the declaration, filter and reading position; fresh file selection still resets", async () => {
  const f = await fixture(); const { page } = f;
  try {
    const top = await f.anchor();
    await f.file("bb.rs");
    await page.getByRole("button", { name: "Back", exact: true }).waitFor({ timeout: 1000 });
    await page.getByRole("button", { name: "Back", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#file-title").textContent === "aa.rs" && document.querySelector('#outline [aria-current="true"]')?.textContent.startsWith("anchor"));
    assert.equal(await page.locator("#filter").inputValue(), "bb.rs");
    assert.equal(await page.locator("#source").evaluate(node => node.scrollTop), top);
    await page.goForward();
    await page.waitForFunction(() => document.querySelector("#file-title").textContent === "bb.rs");
    await page.goBack();
    await page.waitForFunction(() => document.querySelector("#file-title").textContent === "aa.rs");
    assert.equal(await page.locator("#source").evaluate(node => node.scrollTop), top);
    await f.file("bb.rs");
    await f.file("aa.rs");
    await page.waitForFunction(() => !document.querySelector('#outline [aria-current="true"]'));
    assert.equal(await page.locator("#source").evaluate(node => node.scrollTop), 0);
    assert.equal(await page.getByRole("button", { name: "Forward", exact: true }).isDisabled(), true);
    assert.deepEqual(f.session.errors, []);
  } finally { await f.close(); }
});
test("changed declarations are revealed with a notice and missing files never substitute another file", async () => {
  const f = await fixture(); const { page } = f;
  try {
    await f.anchor(); await f.file("bb.rs");
    const old = await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(f.input.root, "aa.rs"), "// inserted\n" + f.code);
    await page.waitForFunction(old => document.querySelector("#snapshot").title !== old, old);
    await page.getByRole("button", { name: "Back", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#file-title").textContent === "aa.rs");
    await page.waitForFunction(() => document.querySelector("#navigation-notice").textContent.includes("changed"));
    assert.match(await page.locator('#outline [aria-current="true"]').getAttribute("title"), /Line 303/);
    await f.file("bb.rs");
    const previous = await page.locator("#snapshot").getAttribute("title");
    await unlink(join(f.input.root, "aa.rs"));
    await page.waitForFunction(previous => document.querySelector("#snapshot").title !== previous, previous);
    await page.getByRole("button", { name: "Back", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#navigation-notice").textContent.includes("Cannot restore"));
    assert.equal(await page.locator("#file-title").textContent(), "bb.rs");
    assert.deepEqual(f.session.errors, []);
  } finally { await f.close(); }
});

test("an unchanged target restores its position after another file changes; late returns cannot replace a fresh selection",async()=>{
  const f=await fixture();const {page}=f;try{
    const top=await f.anchor(),entity=await page.locator('#outline [aria-current="true"]').getAttribute("data-declaration");
    await f.file("bb.rs");
    const old=await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(f.input.root,"bb.rs"),"fn contract() {}\n// changed other file\n");
    await page.waitForFunction(old=>document.querySelector("#snapshot").title!==old,old);
    await page.getByRole("button",{name:"Back",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector("#file-title").textContent==="aa.rs");
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),top);
    assert.equal(await page.locator("#navigation-notice").isVisible(),false);
    await f.file("bb.rs");
    let held;let hold=true;
    await page.route("**/api/view?**",route=>{if(hold&&new URL(route.request().url()).searchParams.get("entity")===entity){held=route;}else return route.continue();});
    await page.getByRole("button",{name:"Back",exact:true}).click();
    for(let i=0;i<100&&!held;i++)await new Promise(resolve=>setTimeout(resolve,5));
    assert.ok(held);
    await f.file("bb.rs");
    await page.waitForFunction(()=>document.querySelector("#navigation").getAttribute("aria-busy")==="false");
    hold=false;await held.continue();await page.waitForTimeout(100);
    assert.equal(await page.locator("#file-title").textContent(),"bb.rs");
    assert.deepEqual(f.session.errors,[]);
  }finally{await f.close();}
});

test("a failed return keeps an explicit notice through resynchronisation and can be retried",async()=>{
  const f=await fixture();const {page}=f;try{
    await f.anchor();const entity=await page.locator('#outline [aria-current="true"]').getAttribute("data-declaration");
    await f.file("bb.rs");
    await page.route("**/api/view?**",route=>new URL(route.request().url()).searchParams.get("entity")===entity?route.abort():route.continue());
    await page.getByRole("button",{name:"Back",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector("#navigation-notice").textContent.includes("Could not restore"));
    assert.equal(await page.locator("#file-title").textContent(),"bb.rs");
    await page.unroute("**/api/view?**");
    await page.getByRole("button",{name:"Forward",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector("#navigation").getAttribute("aria-busy")==="false"&&document.querySelector("#navigation-notice").hidden);
    await page.getByRole("button",{name:"Back",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector('#outline [aria-current="true"]')?.textContent.startsWith("anchor"));
    assert.equal(await page.locator("#file-title").textContent(),"aa.rs");
    assert.deepEqual(f.session.errors,[]);
  }finally{await f.close();}
});
test("a removed declaration in the current file returns to the file start rather than an unrelated reading position",async()=>{
  const f=await fixture();const {page}=f;try{
    await f.anchor();
    await page.locator("#outline button[data-declaration]").filter({hasText:"start"}).click();
    await page.waitForFunction(()=>document.querySelector('#outline [aria-current="true"]')?.textContent.startsWith("start"));
    await page.locator("#source").evaluate(n=>{n.scrollTop=5000;});await paint(page);
    const old=await page.locator("#snapshot").getAttribute("title");
    await writeFile(join(f.input.root,"aa.rs"),f.code.replace("fn anchor","fn replacement"));
    await page.waitForFunction(old=>document.querySelector("#snapshot").title!==old,old);
    await page.getByRole("button",{name:"Back",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector("#navigation-notice").textContent.includes("declaration is no longer"));
    assert.equal(await page.locator("#file-title").textContent(),"aa.rs");
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),0);
    assert.equal(await page.locator('#outline [aria-current="true"]').count(),0);
  }finally{await f.close();}
});
