import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { buildHost, browser, workspace, startHost, paint } from "./support.mjs";

let chromium,input,host,moduleUrl;
const large=Array.from({length:50000},(_,i)=>`fn item_${String(i).padStart(5,"0")}() {}`).join("\n");
before(async()=>{
  moduleUrl="/inspector.js";
  buildHost();input=await workspace({files:{"big.rs":large,"long.rs":"// <img src=x onerror='window.injected=true'> "+"=".repeat(2_000_000)+"TAIL"}});host=await startHost(input);chromium=await browser();
});
after(async()=>{await chromium?.close();await host?.close();await input?.close();});
async function fixture(path="big.rs") {
  const page=await chromium.newPage();page.testErrors=[];page.on("pageerror",error=>page.testErrors.push(error.message));await page.route("**/app.js",route=>route.fulfill({body:"",contentType:"text/javascript"}));await page.goto(host.url);
  await page.evaluate(async({moduleUrl,path})=>{
    const {createInspector}=await import(moduleUrl);
    window.errors=[];window.activations=[];window.requests=[];
    window.transport=async(uri,options)=>{window.requests.push(uri);const response=await fetch(uri,options);if(!response.ok)throw new Error(await response.text());return response.json();};
    window.widget=createInspector({source:document.getElementById("source"),space:document.getElementById("source-space"),outline:document.getElementById("outline"),transport:window.transport,onError:e=>window.errors.push(e.message),select:async fact=>{
      window.activations.push(fact.id);const next=await window.transport(`/api/view?projection=summary&file=${window.current.source.id}&entity=${fact.id}`);window.current=await window.widget.prepare(next);window.widget.apply(window.current);await window.widget.revealActive();
    }});
    const view=await window.transport(`/api/view?projection=summary&filter=${path}`);window.current=await window.widget.prepare(view);window.widget.apply(window.current);
    window.refresh=async()=>{const next=await window.transport(`/api/view?projection=summary&file=${window.current.source.id}`);window.current=await window.widget.prepare(next);window.widget.apply(window.current);};
  },{moduleUrl,path});await paint(page);return page;
}
test("remote Home/End and source linking stay bounded, including activation during pending navigation",async()=>{
  const page=await fixture();try{
    assert.ok(await page.locator("#outline button").count()<=200);assert.ok(await page.locator(".code-line").count()<=96);
    await page.locator("#outline button").first().focus();
    let held,hold=true;await page.route("**/api/window?**",async route=>{if(hold && new URL(route.request().url()).searchParams.get("cursor_index")==="49999"){held=route;}else await route.continue();});
    await page.keyboard.press("End");await page.keyboard.press("Enter");
    await page.waitForFunction(()=>window.activations.length===0);assert.deepEqual(await page.evaluate(()=>window.activations),[]);
    for(let i=0;i<100&&!held;i++)await new Promise(resolve=>setTimeout(resolve,5));assert.ok(held);hold=false;await held.continue();
    await page.waitForFunction(()=>window.activations.length===1);await page.waitForFunction(()=>document.getElementById("source").textContent.includes("item_49999"));
    assert.equal(await page.locator('#outline [aria-current="true"]').locator("..").getAttribute("aria-posinset"),"50000");
    await page.keyboard.press("Home");await page.waitForFunction(()=>document.activeElement?.textContent.startsWith("item_00000"));
    assert.ok(await page.locator("#outline button").count()<=200);assert.deepEqual(await page.evaluate(()=>window.errors),[]);assert.deepEqual(page.testErrors,[]);
  }finally{await page.close();}
});
test("large lines remain literal and can reach their last fragment without full transfers",async()=>{
  const page=await fixture("long.rs");try{
    assert.equal(await page.locator("#source img").count(),0);assert.ok(!(await page.evaluate(()=>window.injected)));
    assert.ok((await page.locator("#source").textContent()).includes("<img src=x"));
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();
    await page.waitForFunction(()=>document.getElementById("source").textContent.includes("TAIL"));
    assert.ok((await page.locator("#source").textContent()).length<66000);
    await page.getByRole("button",{name:"First text on line 1",exact:true}).click();
    await page.waitForFunction(()=>document.getElementById("source").textContent.includes("<img src=x"));
    assert.ok(await page.locator(".code-line").count()<=96);assert.deepEqual(await page.evaluate(()=>window.errors),[]);assert.deepEqual(page.testErrors,[]);
  }finally{await page.close();}
});
test("background document replacement preserves focused identity",async()=>{
  const page=await fixture();try{
    await page.locator("#outline button").first().focus();await page.keyboard.press("ArrowDown");
    const focused=await page.evaluate(()=>document.activeElement.dataset.declaration);
    const previous=await page.evaluate(()=>window.current.snapshot);
    await writeFile(join(input.root,"big.rs"),"// position shift\n"+large);
    await page.waitForFunction(async previous=>(await (await fetch("/api/status")).json()).snapshot!==previous,previous);
    await page.evaluate(()=>window.refresh());
    assert.equal(await page.evaluate(()=>document.activeElement.dataset.declaration),focused);
    assert.ok(await page.locator("#outline button").count()<=200);assert.deepEqual(await page.evaluate(()=>window.errors),[]);assert.deepEqual(page.testErrors,[]);
  }finally{await page.close();}
});
