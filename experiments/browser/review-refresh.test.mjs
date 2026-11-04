import assert from "node:assert/strict";
import {spawnSync} from "node:child_process";
import {writeFile,rm,mkdir,readFile} from "node:fs/promises";
import {join} from "node:path";
import {createHash} from "node:crypto";
import {buildHost,browser,startHost,workspace,paint} from "./support.mjs";

if(!process.argv.includes("--no-build"))buildHost();
const output=process.argv.includes("--output")?process.argv[process.argv.indexOf("--output")+1]:null;
if(output)await mkdir(output);
const baseline=Array.from({length:120},(_,i)=>`fn keep_${String(i).padStart(3,"0")}() {}\n// ${"wide context ".repeat(55)}\n\n`).join("");
let working="// initial working source\n"+baseline;
const note="begin "+"x".repeat(100000)+" TAIL-END\nsecond line\n";
const input=await workspace({files:{"a.rs":"fn baseline() {}\n","z.rs":baseline,"note.txt":note}});
const git=(...args)=>{const r=spawnSync("git",["-C",input.root,"-c","core.hooksPath=/dev/null","-c","user.name=Fixture","-c","user.email=fixture@example.invalid",...args],{encoding:"utf8",env:{...process.env,GIT_CONFIG_GLOBAL:"/dev/null",GIT_CONFIG_NOSYSTEM:"1"}});assert.equal(r.status,0,r.stderr);return r.stdout.trim();};
const report={schema:1,completed:false,steps:[],pageErrors:[],windows:0,windowRequests:[],windowResponses:[]};
let host,chromium,context,page;
try {
  git("init","--quiet","--initial-branch=main","--template=");git("add",".");git("commit","--quiet","-m","base");
  await writeFile(join(input.root,"a.rs"),"fn changed() {}\n");await writeFile(join(input.root,"z.rs"),working);await writeFile(join(input.root,"gone.txt"),"temporary addition\n");
  host=await startHost(input,1);chromium=await browser({desktopBus:false});context=await chromium.newContext({viewport:{width:1440,height:1000}});page=await context.newPage();
  page.setDefaultTimeout(10000);
  const bodies=[];page.on("pageerror",e=>report.pageErrors.push(e.message));
  page.on("request",request=>{if(new URL(request.url()).pathname.endsWith("/window"))report.windowRequests.push(new URL(request.url()).search);});
  page.on("response",response=>{if(new URL(response.url()).pathname.endsWith("/window") && response.status()===200)bodies.push(response.json().then(b=>{report.windowResponses.push({uri:new URL(response.url()).search,side:b.side,rows:b.window.lines.map(l=>({number:l.number,column:l.column,end:l.end_column,tail:l.text.slice(-25)}))});return b;}));});
  const ready=async()=>{await page.waitForFunction(()=>document.querySelector("#review-workspace").dataset.token && document.querySelector("#review-navigation").getAttribute("aria-busy")==="false");await paint(page);};
  const state=()=>page.evaluate(()=>({token:document.querySelector("#review-workspace").dataset.token,file:document.querySelector("#review-file-title").textContent,filter:document.querySelector("#review-filter").value,unchanged:document.querySelector("#include-unchanged").checked,panes:Object.fromEntries(["base","working"].map(side=>{const source=document.getElementById(`${side}-source`),outline=document.getElementById(`${side}-outline`),active=outline.querySelector('[aria-current="true"]');return [side,{version:document.getElementById(`${side}-pane`).dataset.version,sourceTop:source.scrollTop,sourceLeft:source.scrollLeft,outlineTop:outline.scrollTop,active:active?.dataset.declaration ?? null,title:active?.title ?? null,selected:document.getElementById(`select-${side}`).getAttribute("aria-pressed")}];}))}));
  const checkpoint=async name=>{await ready();const current=await state();report.steps.push({name,...current});if(output)await page.screenshot({path:join(output,`${name}.png`),fullPage:true});return current;};
  const choose=async path=>{await page.locator("#review-file-list button").filter({hasText:path}).click();await page.waitForFunction(path=>document.querySelector("#review-file-title").textContent===path && document.querySelector("#review-navigation").getAttribute("aria-busy")==="false",path);await paint(page);};
  const refresh=async()=>{const token=(await state()).token;await page.getByRole("button",{name:"Refresh working",exact:true}).click();await page.waitForFunction(token=>document.querySelector("#review-workspace").dataset.token!==token,token);await ready();};
  await page.goto(host.url+"/review");await page.getByRole("button",{name:"Compare with working tree",exact:true}).click();await ready();
  await choose("z.rs");await page.getByLabel("Include unchanged context").check();await page.getByLabel("Filter paths",{exact:true}).fill("z.rs");await page.waitForFunction(()=>document.querySelectorAll("#review-file-list button").length===1);await ready();
  await page.locator("#working-outline button").first().focus();await page.keyboard.press("End");await page.keyboard.press("Enter");await page.waitForFunction(()=>document.querySelector('#working-outline [aria-current="true"]')?.textContent.includes("keep_119"));await ready();
  await page.evaluate(()=>{for(const [side,top] of [["base",2200],["working",4400]]){document.getElementById(`${side}-source`).scrollTop=top;document.getElementById(`${side}-outline`).scrollTop=480;}});
  await page.waitForFunction(()=>[...document.querySelectorAll("#base-source .line-no")].some(n=>n.textContent==="101") && [...document.querySelectorAll("#working-source .line-no")].some(n=>n.textContent==="201"));
  await page.evaluate(()=>{document.querySelector("#base-source").scrollLeft=112;document.querySelector("#working-source").scrollLeft=144;});await paint(page);
  const before=await checkpoint("before-refresh");
  assert.ok(before.panes.base.sourceLeft>0);assert.ok(before.panes.working.sourceLeft>0);
  await refresh();const same=await checkpoint("unchanged-refresh");assert.equal(same.file,"z.rs");assert.equal(same.filter,"z.rs");assert.equal(same.unchanged,true);
  for(const side of ["base","working"])assert.deepEqual(same.panes[side],before.panes[side]);
  const selected=before.panes.working.active;assert.ok(selected);
  working="// moved\n".repeat(30)+working;await writeFile(join(input.root,"z.rs"),working);await refresh();const moved=await checkpoint("moved-declaration");
  assert.equal(moved.file,"z.rs");assert.equal(moved.panes.working.active,selected);assert.notEqual(moved.panes.working.version,before.panes.working.version);assert.match(moved.panes.working.title,/line 389$/);assert.equal(moved.panes.working.sourceLeft,0);assert.deepEqual(moved.panes.base,before.panes.base);
  assert.equal(await page.locator("#working-source").evaluate(n=>[...n.querySelectorAll(".code-line")].some(row=>{const b=row.getBoundingClientRect(),p=n.getBoundingClientRect();return row.textContent.includes("fn keep_119") && b.bottom>p.top && b.top<p.bottom;})),true);
  working=working.replace("fn keep_119()", "fn renamed_119()");await writeFile(join(input.root,"z.rs"),working);await refresh();const missing=await checkpoint("missing-declaration");
  assert.equal(missing.panes.working.active,null);assert.equal(missing.panes.working.sourceTop,0);assert.match(await page.locator("#review-notice").textContent(),/declaration.*available/);assert.deepEqual(missing.panes.base,before.panes.base);
  await rm(join(input.root,"z.rs"));await refresh();const removed=await checkpoint("removed-side");assert.equal(removed.file,"z.rs");assert.equal(removed.panes.base.selected,"true");assert.equal(removed.panes.working.version,"");assert.match(await page.locator("#working-evidence").textContent(),/Absent/);
  await page.getByRole("button",{name:"Compare with working tree",exact:true}).click();await page.waitForFunction(token=>document.querySelector("#review-workspace").dataset.token!==token,removed.token);await ready();const created=await checkpoint("new-capture");assert.equal(created.file,"a.rs");assert.equal(created.filter,"");assert.equal(created.unchanged,false);
  await page.getByLabel("Include unchanged context").check();await page.getByLabel("Filter paths",{exact:true}).fill("note.txt");await page.waitForFunction(()=>document.querySelectorAll("#review-file-list button").length===1 && document.querySelector("#review-file-list").textContent.includes("note.txt"));await choose("note.txt");
  await page.locator("#working-source").getByRole("button",{name:"Last text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#working-source").textContent.includes("TAIL-END"));await paint(page);const fragment=await checkpoint("expanded-fragment");
  assert.ok(fragment.panes.working.sourceLeft>0);await refresh();const retained=await checkpoint("retained-fragment");assert.equal(retained.file,"note.txt");assert.equal(retained.panes.working.sourceLeft,fragment.panes.working.sourceLeft);assert.equal((await page.locator("#working-source").textContent()).includes("TAIL-END"),true);
  await page.getByLabel("Filter paths",{exact:true}).fill("gone.txt");await page.waitForFunction(()=>document.querySelector("#review-file-list").textContent.includes("gone.txt"));await choose("gone.txt");await rm(join(input.root,"gone.txt"));await refresh();await page.waitForFunction(()=>document.querySelector("#review-file-title").textContent==="No file selected");await checkpoint("missing-file");assert.match(await page.locator("#review-notice").textContent(),/absent/);
  const live=await (await page.request.get(host.url+"/api/status")).json();assert.equal(live.readiness,"not_started");assert.equal(live.snapshot,null);
  for(const b of await Promise.all(bodies)){assert.ok(b.window.lines.length<=96);assert.ok(b.window.outline.length<=200);assert.ok(b.window.lines.reduce((n,l)=>n+new TextEncoder().encode(l.text).length,0)<=32768);report.windows++;}
  assert.deepEqual(report.pageErrors,[]);report.binarySha256=createHash("sha256").update(await readFile("target/release/vizzi")).digest("hex");report.completed=true;
  console.log("review refresh browser: same-version viewports, fresh declaration reveal, missing targets, new capture reset and expanded fragments passed");
} catch(error) {report.failure={message:error.message,stack:error.stack};if(output && page)await page.screenshot({path:join(output,"failure.png"),fullPage:true});throw error;}
finally {await context?.close();await chromium?.close();await host?.close();await input.close();if(output)await writeFile(join(output,"observations.json"),JSON.stringify(report,null,2)+"\n");}
