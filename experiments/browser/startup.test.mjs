import assert from "node:assert/strict";
import {rename} from "node:fs/promises";
import {buildHost,browser,startHost,workspace} from "./support.mjs";

if(!process.argv.includes("--no-build"))buildHost();
const input=await workspace({files:{"a.rs":"fn first() {}\n"}});
let chromium,host,context,moved=false,release;
const deferred=()=>{let resolve;const promise=new Promise(r=>{resolve=r;});return{promise,resolve};};
try {
  host=await startHost(input,1);chromium=await browser({desktopBus:false});context=await chromium.newContext();
  const page=await context.newPage(),errors=[],requests=[];
  page.on("pageerror",error=>errors.push(error.message));
  page.on("request",request=>{if(new URL(request.url()).pathname.startsWith("/api/"))requests.push(request);});
  const initial=await (await fetch(`${host.url}/api/status`)).json();
  assert.equal(initial.readiness,"not_started");assert.equal(initial.snapshot,null);assert.equal(initial.measurements,null);

  const gate=deferred(),entered=deferred();release=gate.resolve;
  let attempts=0;
  await page.route("**/api/workspace/start",async route=>{
    assert.equal(route.request().method(),"POST");assert.equal(route.request().postData(),"{}");
    if(++attempts===1)return route.fulfill({status:503,body:"Activation connection unavailable"});
    if(attempts===2){entered.resolve();await gate.promise;}
    return route.continue();
  });
  await page.goto(host.url);
  await page.getByRole("button",{name:"Start indexing",exact:true}).waitFor({state:"visible"});
  await page.waitForFunction(()=>!document.querySelector("#start-index").disabled && !document.querySelector("#error").hidden);
  assert.match(await page.locator("#error").textContent(),/No source observation is available/);
  assert.equal(await page.locator("#snapshot").textContent(),"");assert.equal(await page.locator("#empty").isVisible(),false);
  await page.getByRole("button",{name:"Start indexing",exact:true}).click();await entered.promise;
  assert.equal(await page.locator("#start-index").isDisabled(),true);assert.equal(attempts,2);

  // A missing source root produces a real initial capture failure, then recovers.
  await rename(input.root,`${input.root}-held`);moved=true;release();
  await page.getByRole("button",{name:"Retry indexing",exact:true}).waitFor({state:"visible"});
  await page.waitForFunction(()=>!document.querySelector("#start-index").disabled);
  assert.equal(await page.locator("#status").textContent(),"Initial indexing failed");
  assert.equal(await page.locator("#counts").textContent(),"Initial indexing failed");assert.equal(await page.locator("#snapshot").textContent(),"");
  const failed=await (await fetch(`${host.url}/api/status`)).json();assert.equal(failed.readiness,"failed");assert.equal(failed.snapshot,null);
  await rename(`${input.root}-held`,input.root);moved=false;
  await page.getByRole("button",{name:"Retry indexing",exact:true}).click();
  await page.waitForFunction(()=>document.querySelector("#snapshot").textContent.length===8);
  assert.equal(await page.locator("#file-title").textContent(),"a.rs");assert.equal(await page.locator("#start-index").isVisible(),false);assert.equal(await page.locator("#error").isVisible(),false);
  const ready=await (await fetch(`${host.url}/api/status`)).json();assert.equal(ready.readiness,"ready");assert.equal(ready.measurements.files,1);
  assert.equal(attempts,3);assert.equal(requests.filter(r=>new URL(r.url()).pathname==="/api/status").length,0,"the page uses subscriptions without polling");
  assert.deepEqual(errors,[]);console.log("startup browser: dormant metadata, failed transport, initial source failure, explicit retry and exploration passed");
} finally {
  release?.();await context?.close();await chromium?.close();await host?.close();
  if(moved)await rename(`${input.root}-held`,input.root);
  await input.close();
}
