import assert from "node:assert/strict";
import {readFile,writeFile,mkdir,realpath} from "node:fs/promises";
import {join,resolve,dirname,basename} from "node:path";
import {cpus,totalmem,platform,release,loadavg} from "node:os";
import {createHash,randomUUID} from "node:crypto";
import {setTimeout as delay} from "node:timers/promises";
import {spawnSync} from "node:child_process";
import {openSync,closeSync,readFileSync} from "node:fs";
import {buildHost,startHost,browser,paint} from "../browser/support.mjs";
import {summary} from "../browser/statistics.mjs";
import {auditResponse,auditCheckpoint} from "./walkthrough-audit.js";

const options={sessions:3};
for(let i=2;i<process.argv.length;i++) {
  const flag=process.argv[i];if(flag==="--review-scale")continue;
  if(flag==="--no-build"){options.noBuild=true;continue;}
  assert.ok(["--fixture","--output","--sessions"].includes(flag),`Unknown option ${flag}`);
  const value=process.argv[++i];assert.ok(value && !value.startsWith("--"),`Missing value for ${flag}`);options[flag.slice(2)]=flag==="--sessions"?Number(value):value;
}
assert.ok(options.fixture && options.output && Number.isInteger(options.sessions) && options.sessions>=1 && options.sessions<=5);
const fixture=await realpath(resolve(options.fixture)),manifest=JSON.parse(await readFile(join(fixture,"manifest.json"),"utf8"));
const root=join(fixture,manifest.workspace),output=join(await realpath(dirname(resolve(options.output))),basename(options.output));
assert.ok(output!==fixture && !output.startsWith(fixture+"/"));await mkdir(output);
const sha=value=>createHash("sha256").update(value).digest("hex");
const originals=new Map(await Promise.all(manifest.files.map(async file=>{const data=await readFile(join(fixture,file.baseline));assert.equal(sha(data),file.base_sha256);return[file.path,data];})));
const report={schema:1,case:manifest.name,revision:manifest.revision,scope:manifest.scope,indexPaths:manifest.index_paths,scopePaths:manifest.scope_paths,files:manifest.files,options,
  hardware:{platform:platform(),release:release(),cpu:cpus()[0].model,logicalCpus:cpus().length,totalMemory:totalmem(),initialLoad:loadavg()},sessions:[],completed:false,
  limitations:["Full working tree and Git index, but a focused review scope; this is not full-repository comparison or interaction.","Fresh analysis caches, but checkout/Git reads warm filesystem caches.","One parser worker; default resident source and disabled syntax/input retention options.","Primary latency starts at the browser trusted click and ends after matching DOM, automation observation and two rAF callbacks, not physical presentation. Command elapsed time is retained separately. Finite percentiles do not qualify timing tails.","Native RSS excludes Git children; renderer heap is a point sample, not browser RSS or a 16-GB/editor envelope.","Agent tests/mutations/other benchmarks do not overlap measurement; external host activity is uncontrolled."]};
const current=new Map();
function originalStatus(name) {
  const path=join(output,`original-status-${name}`),fd=openSync(path,"w");let result;
  const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith("GIT_")));Object.assign(env,{GIT_OPTIONAL_LOCKS:"0",GIT_CONFIG_GLOBAL:"/dev/null",GIT_CONFIG_NOSYSTEM:"1"});
  try{result=spawnSync("git",["-C",manifest.source,"-c","core.fsmonitor=false","status","--porcelain=v1","-z","--untracked-files=all"],{stdio:["ignore",fd,"inherit"],env});}finally{closeSync(fd);}
  assert.equal(result.status,0);return sha(readFileSync(path));
}
async function nativeMemory(pid) {
  if(platform()!=="linux")return null;
  const text=await readFile(`/proc/${pid}/status`,"utf8");return {rssBytes:Number(text.match(/VmRSS:\s+(\d+)/)[1])*1024,highWaterBytes:Number(text.match(/VmHWM:\s+(\d+)/)[1])*1024};
}
async function ready(page) {await page.waitForFunction(()=>document.getElementById("review-navigation").getAttribute("aria-busy")==="false");await paint(page);}
async function inputAge(page,at=null) {return page.evaluate(at=>{const start=at ?? window.vizziReviewClickAt;if(!Number.isFinite(start))throw new Error("Missing trusted click timestamp");return performance.now()-start;},at);}
async function choose(page,path) {await page.locator("#review-file-list button").filter({hasText:path}).click();await page.waitForFunction(path=>document.getElementById("review-file-title").textContent===path,path);await ready(page);}
async function markerVisible(page,path) {await page.waitForFunction(({path,marker})=>document.getElementById("review-file-title").textContent===path && [...document.querySelectorAll("#working-source .source-text")].some(n=>n.textContent===marker),{path,marker:current.get(path)});}
async function hunk(page,path) {await page.locator("#review-hunks button").first().click();await ready(page);await markerVisible(page,path);}
async function begin(page) {
  await page.evaluate(()=>{
    performance.clearResourceTimings();performance.setResourceTimingBufferSize(2000);
    const p=window.reviewScaleProbe={frames:[],longTasks:[],last:null,active:true};
    const tick=t=>{if(!p.active)return;if(p.last!==null)p.frames.push(t-p.last);p.last=t;p.raf=requestAnimationFrame(tick);};p.raf=requestAnimationFrame(tick);
    if(PerformanceObserver.supportedEntryTypes.includes("longtask")){p.observer=new PerformanceObserver(list=>p.longTasks.push(...list.getEntries().map(e=>e.duration)));p.observer.observe({type:"longtask"});}
  });
}
async function end(page) {
  return page.evaluate(()=>{
    const p=window.reviewScaleProbe;p.active=false;cancelAnimationFrame(p.raf);p.longTasks.push(...(p.observer?.takeRecords() ?? []).map(e=>e.duration));p.observer?.disconnect();
    return {frameIntervals:p.frames,longTasksSupported:Boolean(p.observer),longTasks:p.longTasks,resources:performance.getEntriesByType("resource").filter(e=>new URL(e.name).pathname.startsWith("/api/comparisons/")).map(e=>({endpoint:new URL(e.name).pathname.split("/").at(-1),status:e.responseStatus ?? null,duration:e.duration,encodedBodySize:e.encodedBodySize,decodedBodySize:e.decodedBodySize})),dom:{fileButtons:document.querySelectorAll("#review-file-list button").length,hunkButtons:document.querySelectorAll("#review-hunks button").length,panes:["base","working"].map(side=>({side,sourceRows:document.querySelectorAll(`#${side}-source .code-line`).length,styleSpans:document.querySelectorAll(`#${side}-source .source-text > span`).length,outlineButtons:document.querySelectorAll(`#${side}-outline button`).length}))}};
  });
}
function summarizeProbe(p) {
  assert.ok(p.frameIntervals.length>10,"enough rAF samples");assert.ok(p.dom.fileButtons<=200 && p.dom.hunkButtons<=200);
  for(const pane of p.dom.panes)assert.ok(pane.sourceRows<=96 && pane.outlineButtons<=200 && pane.styleSpans<=1024);
  return {...p,frames:summary(p.frameIntervals),fetches:summary(p.resources.filter(r=>r.status>=200 && r.status<300).map(r=>r.duration))};
}
async function verify(page,host,path) {
  await choose(page,path);await hunk(page,path);
  const state=await page.evaluate(()=>{
    const panes={};for(const side of ["base","working"]) {const source=document.getElementById(`${side}-source`),bounds=source.getBoundingClientRect();panes[side]={version:document.getElementById(`${side}-pane`).dataset.version,renderedRows:source.querySelectorAll(".code-line").length,styleSpans:document.querySelectorAll(`#${side}-source .source-text > span`).length,outlineButtons:document.querySelectorAll(`#${side}-outline button`).length,rows:[...source.querySelectorAll(".code-line")].filter(n=>{const b=n.getBoundingClientRect();return b.bottom>bounds.top && b.top<bounds.bottom;}).map(n=>({number:Number(n.querySelector(".line-no").textContent),text:n.querySelector(".source-text").textContent}))};}
    const selectedSides=["base","working"].filter(side=>document.getElementById(`select-${side}`).getAttribute("aria-pressed")==="true");
    return {token:document.getElementById("review-workspace").dataset.token,comparison:document.getElementById("review-workspace").dataset.comparison,path:document.getElementById("review-file-title").textContent,side:selectedSides[0],selectedSides,panes,fileButtons:document.querySelectorAll("#review-file-list button").length,hunkButtons:document.querySelectorAll("#review-hunks button").length};
  });
  const get=async uri=>{const response=await page.request.get(host.url+uri);assert.equal(response.status(),200);const body=await response.json();auditResponse(uri,body);return body;};
  const inventory=await get(`/api/comparisons/${encodeURIComponent(state.token)}/files?unchanged=true&filter=${encodeURIComponent(path)}`),file=inventory.files.find(f=>f.path===path);assert.ok(file);
  const windows=[];for(const side of ["base","working"])windows.push(await get(`/api/comparisons/${encodeURIComponent(state.token)}/window?file=${file.file}&side=${side}&version=${file[`${side}_version`]}&line_start=1&line_count=96&outline_count=200`));
  const proof=auditCheckpoint(state,{...file,token:state.token,comparison:state.comparison},windows,{base:originals.get(path),working:Buffer.concat([Buffer.from(current.get(path)+"\n"),originals.get(path)])});
  return {path,...proof,scopeTotals:inventory.totals,baseVersion:file.base_version,workingVersion:file.working_version,coverage:windows.map(w=>({side:w.side,coloring:w.window.coloring,styleSpans:w.window.lines.reduce((n,line)=>n+(line.styles?.length ?? 0),0),classification:w.classification,contexts:w.contexts.map(c=>({language:c.language,coverage:c.coverage,declarations:c.declarations}))}))};
}

let chromium;
try {
  report.sourceSha256=Object.fromEntries(await Promise.all(["crates/vizzi-core/src/coloring.rs","crates/vizzi-core/src/projection.rs","crates/vizzi-core/src/syntax.rs","crates/vizzi/web/presentation.js","crates/vizzi/web/camera.js","crates/vizzi/web/splitter.js","crates/vizzi/web/app.css","crates/vizzi/web/review.css","experiments/review-workflow/measure.mjs","experiments/review-workflow/scale_fixture.py","experiments/review-workflow/scale-profiles.json","experiments/review-workflow/walkthrough-audit.js","experiments/browser/statistics.mjs","crates/vizzi/web/review-app.js","crates/vizzi/web/review-controller.js","crates/vizzi/web/inspector.js","crates/vizzi/src/host.rs","crates/vizzi/src/application.rs","crates/vizzi/src/server.rs","crates/vizzi/src/main.rs","crates/vizzi/web/startup.js","crates/vizzi/src/review.rs","crates/vizzi/src/review_application.rs","crates/vizzi/src/review_application/projection.rs"].map(async name=>[name,sha(await readFile(name))])));
  report.originalStatusBefore=originalStatus("before");assert.equal(report.originalStatusBefore,manifest.original_status_sha256);
  if(!options.noBuild)buildHost();report.binarySha256=sha(await readFile("target/release/vizzi"));chromium=await browser({desktopBus:false});report.browser=chromium.version();
  const system=await (await chromium.newBrowserCDPSession()).send("SystemInfo.getInfo");report.gpu={devices:system.gpu.devices,featureStatus:system.gpu.featureStatus};
  for(let session=0;session<options.sessions;session++) {
    let host,context,page;const result={session,latencies:[],refreshes:[],errors:[],abortedRequests:0,retiredQueryConflicts:0,proofs:[],load:loadavg()};report.sessions.push(result);const retired=new Set();
    const id=randomUUID();
    for(const file of manifest.files){const marker=`${file.prefix} vizzi-review-scale ${id} initial`;current.set(file.path,marker);await writeFile(join(root,file.path),Buffer.concat([Buffer.from(marker+"\n"),originals.get(file.path)]));}
    try {
      const hostStarted=performance.now();let started=hostStarted;host=await startHost({root,cache:join(output,`cache-${session}`),sourceArguments:manifest.source_arguments,startupTimeoutMs:60000},1);result.hostReadyMs=performance.now()-started;
      context=await chromium.newContext({viewport:{width:1440,height:1000}});page=await context.newPage();page.setDefaultTimeout(30000);
      await page.addInitScript(()=>document.addEventListener("click",event=>{if(event.isTrusted)window.vizziReviewClickAt=performance.now();},true));
      page.on("pageerror",e=>result.errors.push(e.message));page.on("requestfailed",request=>{if(request.failure()?.errorText.includes("ERR_ABORTED"))result.abortedRequests++;else result.errors.push(request.failure()?.errorText);});
      page.on("response",response=>{
        const url=new URL(response.url());if(url.pathname.endsWith("/status"))result.errors.push("Page polled status");
        if(response.status()<400)return;const token=decodeURIComponent(url.pathname.split("/")[3] ?? "");
        if(response.status()===409 && retired.has(token))result.retiredQueryConflicts++;else result.errors.push(`${response.status()} ${url.pathname}`);
      });
      await page.route("**/*",route=>new URL(route.request().url()).origin===host.url?route.continue():route.abort());
      await page.goto(host.url+"/review");await page.bringToFront();await page.getByLabel("Base revision",{exact:true}).fill(manifest.revision);await page.getByLabel("Repository path",{exact:true}).fill(manifest.scope);
      result.startupStatus=await (await page.request.get(host.url+"/api/status")).json();
      assert.equal(result.startupStatus.readiness,"not_started");assert.equal(result.startupStatus.snapshot,null);assert.equal(result.startupStatus.measurements,null);
      started=performance.now();await page.getByRole("button",{name:"Compare with working tree",exact:true}).click();await page.waitForFunction(()=>Boolean(document.getElementById("review-workspace").dataset.token) || document.getElementById("review-status").textContent.startsWith("Capture failed"));assert.ok(await page.locator("#review-workspace").getAttribute("data-token"),await page.locator("#review-status").textContent());await ready(page);result.firstComparisonCommandMs=performance.now()-started;result.firstComparisonMs=await inputAge(page);
      result.firstReviewReadySinceSpawnMs=performance.now()-hostStarted;
      const initial=await page.evaluate(()=>({token:document.getElementById("review-workspace").dataset.token,labels:document.getElementById("comparison-labels").textContent}));assert.ok(initial.labels.includes(manifest.revision));result.afterCaptureMemory=await nativeMemory(host.pid);
      await page.getByLabel("Include unchanged context").check();await ready(page);
      for(const file of manifest.files)result.proofs.push(await verify(page,host,file.path));
      const cdp=await context.newCDPSession(page);result.heapBefore=await cdp.send("Runtime.getHeapUsage");
      await begin(page);
      for(let cycle=0;cycle<6;cycle++)for(const file of manifest.files) {
        started=performance.now();await choose(page,file.path);await markerVisible(page,file.path);result.latencies.push({kind:"file",path:file.path,cycle,commandMs:performance.now()-started,ms:await inputAge(page)});
        const button=page.locator("#working-outline button").first();
        if(await button.count()) {const entity=await button.getAttribute("data-declaration");started=performance.now();await button.click();await page.waitForFunction(id=>document.querySelector('#working-outline [aria-current="true"]')?.dataset.declaration===id,entity);await ready(page);result.latencies.push({kind:"declaration",path:file.path,cycle,commandMs:performance.now()-started,ms:await inputAge(page)});}
        started=performance.now();await hunk(page,file.path);result.latencies.push({kind:"hunk",path:file.path,cycle,commandMs:performance.now()-started,ms:await inputAge(page)});
      }
      result.inspection=summarizeProbe(await end(page));
      await begin(page);await page.locator("#working-source").hover();const until=performance.now()+3000;let step=0;
      while(performance.now()<until){await page.mouse.wheel(0,step++%40<20?66:-66);await delay(16);}
      await paint(page);result.scroll=summarizeProbe(await end(page));
      await begin(page);
      for(let edit=0;edit<5;edit++) {
        const file=manifest.files[edit%manifest.files.length];await choose(page,file.path);await hunk(page,file.path);
        const old=await page.locator("#review-workspace").getAttribute("data-token"),oldVersion=await page.locator("#working-pane").getAttribute("data-version"),marker=`${file.prefix} vizzi-review-scale ${id} edit-${edit}`;
        await writeFile(join(root,file.path),Buffer.concat([Buffer.from(marker+"\n"),originals.get(file.path)]));await paint(page);
        assert.equal(await page.locator("#review-workspace").getAttribute("data-token"),old);assert.equal(await page.locator("#working-pane").getAttribute("data-version"),oldVersion);await markerVisible(page,file.path);
        retired.add(old);current.set(file.path,marker);started=performance.now();await page.getByRole("button",{name:"Refresh working",exact:true}).click();
        const clickAt=await page.evaluate(()=>window.vizziReviewClickAt);
        let scrolling=true,scrollInputs=0;await page.locator("#working-source").hover();
        const scroll=(async()=>{while(scrolling && await page.locator("#review-workspace").getAttribute("data-token")===old){await page.mouse.wheel(0,44);scrollInputs++;await delay(16);}})();
        try {await page.waitForFunction(old=>document.getElementById("review-workspace").dataset.token!==old,old);await ready(page);}finally {scrolling=false;await scroll;}
        const captureCommandMs=performance.now()-started,captureMs=await inputAge(page,clickAt),selected=await page.locator("#review-file-title").textContent();
        assert.equal(selected,file.path,"Working refresh keeps the edited file selected");
        assert.equal(await page.getByLabel("Include unchanged context").isChecked(),true,"Refresh keeps context projection settings");
        await markerVisible(page,file.path);await paint(page);
        result.refreshes.push({edit,path:file.path,captureMs,captureCommandMs,editedSourceReadyMs:await inputAge(page,clickAt),editedSourceCommandMs:performance.now()-started,selectionReset:selected!==file.path,scrollInputs,oldToken:old,newToken:await page.locator("#review-workspace").getAttribute("data-token"),oldVersion,newVersion:await page.locator("#working-pane").getAttribute("data-version"),baseVersion:await page.locator("#base-pane").getAttribute("data-version")});
      }
      result.refreshActivity=summarizeProbe(await end(page));
      for(const file of manifest.files)result.proofs.push(await verify(page,host,file.path));
      result.heapAfter=await cdp.send("Runtime.getHeapUsage");result.finalMemory=await nativeMemory(host.pid);
      result.finalLiveStatus=await (await page.request.get(host.url+"/api/status")).json();
      assert.equal(result.finalLiveStatus.readiness,"not_started");assert.equal(result.finalLiveStatus.snapshot,null);assert.equal(result.finalLiveStatus.measurements,null);
      result.liveIndexStayedDormant=true;
      assert.deepEqual(result.errors,[]);for(const file of manifest.files){const proofs=result.proofs.filter(p=>p.path===file.path);assert.equal(proofs[0].baseVersion,proofs.at(-1).baseVersion);assert.notEqual(proofs[0].workingVersion,proofs.at(-1).workingVersion);}
      await page.screenshot({path:join(output,`session-${session}.png`),fullPage:true});
      result.latencySummaries=Object.fromEntries(["file","hunk","declaration"].map(kind=>[kind,summary(result.latencies.filter(s=>s.kind===kind).map(s=>s.ms))]));
      result.refreshSummary={capture:summary(result.refreshes.map(s=>s.captureMs)),editedSourceReady:summary(result.refreshes.map(s=>s.editedSourceReadyMs))};result.completed=true;
    } catch(error) {if(page){result.failureState=await page.evaluate(()=>({status:document.getElementById("review-status").textContent,notice:document.getElementById("review-notice").textContent}));await page.screenshot({path:join(output,`failure-${session}.png`),fullPage:true});}throw error;}
    finally {await context?.close();await host?.close();await writeFile(join(output,"measurements.json"),JSON.stringify(report,null,2)+"\n");}
  }
  report.completed=true;
} catch(error) {report.failure={message:error.message,stack:error.stack};process.exitCode=1;}
finally {
  await chromium?.close();
  try{report.originalStatusAfter=originalStatus("after");assert.equal(report.originalStatusAfter,report.originalStatusBefore);report.originalCheckoutUnchanged=true;}catch(error){report.failure={message:error.message};report.completed=false;process.exitCode=1;}
  report.finished=new Date().toISOString();await writeFile(join(output,"measurements.json"),JSON.stringify(report,null,2)+"\n");
}
console.log(JSON.stringify({case:report.case,completed:report.completed,sessions:report.sessions.map(s=>({session:s.session,firstComparisonMs:s.firstComparisonMs,selection:s.latencySummaries,refresh:s.refreshSummary,memory:s.finalMemory})),failure:report.failure},null,2));
