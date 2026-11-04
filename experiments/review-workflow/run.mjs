import assert from "node:assert/strict";
import {createHash} from "node:crypto";
import {spawnSync} from "node:child_process";
import {openSync,closeSync,readFileSync} from "node:fs";
import {mkdir,mkdtemp,readFile,writeFile,realpath,rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {basename,dirname,join,resolve,sep} from "node:path";
import {buildHost,browser,startHost,paint} from "../browser/support.mjs";
import {auditResponse,auditCheckpoint} from "./walkthrough-audit.js";

const options={};
for(let i=2;i<process.argv.length;i++) {
  const flag=process.argv[i];
  if(flag==="--review-walkthrough")continue;
  if(flag==="--no-build"){options.noBuild=true;continue;}
  assert.ok(["--fixture","--output"].includes(flag),`Unknown option ${flag}`);
  assert.ok(process.argv[i+1] && !process.argv[i+1].startsWith("--"),`Missing value for ${flag}`);
  options[flag.slice(2)]=process.argv[++i];
}
assert.ok(options.fixture && options.output,"Use --fixture and --output");
const fixture=await realpath(resolve(options.fixture)),root=await realpath(join(fixture,"workspace"));
const output=join(await realpath(dirname(resolve(options.output))),basename(options.output));
assert.ok(output!==fixture && !output.startsWith(fixture+sep),"Output must be outside the fixture");
assert.ok(output!==root && !output.startsWith(root+sep),"Output must be outside the workspace");
await mkdir(output);
const manifest=JSON.parse(await readFile(join(fixture,"manifest.json"),"utf8"));
const oracle=JSON.parse(await readFile("experiments/review-workflow/fixtures/oracle.json","utf8"));
assert.match(oracle.base_revision,/^[0-9a-f]{40}$/);
assert.ok(oracle.variants[manifest.variant],"Unknown fixture variant");
const temporary=await mkdtemp(join(tmpdir(),"vizzi-rendered-review-"));
const report={schema:1,completed:false,phase:"fixture_verification",variant:manifest.variant,
  operator:"Reference-aware guided automation; paths, declaration names and expected behaviour are known. No unfamiliar-operator or comprehension claim.",
  options,steps:[],outlineScans:[],responses:[],responseErrors:[],pageErrors:[],cleanupErrors:[],
  limitations:["Navigation uses rendered controls; response data is used only to audit identity, bounds and visible source.","Timing includes scripted pacing and is task friction evidence, not human task time or a latency qualification.","No analysed-project build, hooks or scripts execute. The prepared fixture is verified before and after execution."]};
const hash=bytes=>createHash("sha256").update(bytes).digest("hex");
let commandId=0;
function command(program,args,destination) {
  const fd=openSync(destination,"w");let result;
  try {result=spawnSync(program,args,{stdio:["ignore",fd,fd],env:{...process.env,GIT_OPTIONAL_LOCKS:"0",GIT_CONFIG_GLOBAL:"/dev/null",GIT_CONFIG_NOSYSTEM:"1"}});}
  finally {closeSync(fd);}
  assert.equal(result.status,0,`${program} failed: ${readFileSync(destination,"utf8").slice(-2000)}`);
  return readFileSync(destination);
}
function verify(name) {
  const result=JSON.parse(command("python3",["-B","experiments/review-workflow/verify.py","--fixture",fixture],join(output,`${name}-verification.json`)));
  assert.equal(result.verified,true);return result;
}
let host,chromium,context,page,video,traceStarted=false,plannedExpiryToken,expectedToken;
const jobs=new Set(),files=new Map(),windows=new Map(),sourceCache=new Map();
const paths=new Set([...oracle.evidence.map(e=>e.path),oracle.untracked_path]);
async function source(side,path) {
  assert.ok(paths.has(path),"Source audit path must be a frozen task input");
  const key=`${side}:${path}`;
  if(!sourceCache.has(key))sourceCache.set(key,side==="working"?await readFile(join(root,path)):command("git",["-C",root,"show",`${oracle.base_revision}:${path}`],join(temporary,`source-${++commandId}`)));
  return sourceCache.get(key);
}
async function drain() {await Promise.all([...jobs]);}
async function state() {
  return page.evaluate(()=>{
    const element=id=>document.getElementById(id),panes={};
    for(const side of ["base","working"]) {
      const source=element(`${side}-source`),outline=element(`${side}-outline`),bounds=source.getBoundingClientRect();
      panes[side]={version:element(`${side}-pane`).dataset.version,sourceTop:source.scrollTop,sourceLeft:source.scrollLeft,outlineTop:outline.scrollTop,
        declaration:outline.querySelector('[aria-current="true"]')?.dataset.declaration ?? null,
        renderedRows:source.querySelectorAll(".code-line").length,outlineButtons:outline.querySelectorAll("button").length,
        rows:[...source.querySelectorAll(".code-line")].filter(row=>{const b=row.getBoundingClientRect();return b.bottom>bounds.top && b.top<bounds.bottom;}).map(row=>({number:Number(row.querySelector(".line-no").textContent),text:row.querySelector(".source-text").textContent}))};
    }
    const selectedSides=["base","working"].filter(side=>element(`select-${side}`).getAttribute("aria-pressed")==="true");
    return {token:element("review-workspace").dataset.token,comparison:element("review-workspace").dataset.comparison,
      path:element("review-file-title").textContent,side:selectedSides[0],selectedSides,panes,
      labels:element("comparison-labels").textContent,counts:element("review-counts").textContent,filter:element("review-filter").value,
      notice:element("review-notice").hidden?null:element("review-notice").querySelector("span").textContent,
      fileButtons:element("review-file-list").querySelectorAll("button").length,hunkButtons:element("review-hunks").querySelectorAll("button").length,
      hunks:[...element("review-hunks").querySelectorAll("button")].map(button=>({id:button.dataset.hunk,label:button.textContent,selected:button.getAttribute("aria-current")==="true"})),history:history.state?.vizziReview?.key};
  });
}
async function ready() {await page.waitForFunction(()=>document.querySelector("#review-navigation").getAttribute("aria-busy")==="false");await paint(page);await drain();}
async function checkpoint(name,intent) {
  await ready();await page.waitForTimeout(120);await drain();const current=await state();
  if(current.token) {
    if(expectedToken)assert.equal(current.token,expectedToken,"Comparison stays pinned until explicit refresh");
    const file=files.get(`${current.token}:${current.path}`);assert.ok(file,"Displayed path must have inventory evidence");
    const bound=[...windows.values()].flat();
    const sources={};for(const side of ["base","working"])if(file[`${side}_version`] && current.panes[side].rows.length)sources[side]=await source(side,current.path);
    current.audit=auditCheckpoint(current,file,bound,sources);
    assert.ok(current.labels.includes(oracle.base_revision),"Resolved baseline is displayed");
  }
  const screenshot=`${String(report.steps.length).padStart(2,"0")}-${name}.png`;
  report.steps.push({name,intent,state:current,screenshot});await page.screenshot({path:join(output,screenshot),fullPage:true});
  console.log(`${name}: ${current.path}`);return current;
}
async function filter(path) {
  const response=page.waitForResponse(r=>{const u=new URL(r.url());return u.pathname.endsWith("/files") && u.searchParams.get("filter")===path && r.status()===200;});
  await page.getByLabel("Filter paths",{exact:true}).fill(path);await response;await ready();
}
async function choose(path) {
  const button=page.locator("#review-file-list button").filter({hasText:path});assert.equal(await button.count(),1,"Unambiguous rendered file choice");
  await button.click();await page.waitForFunction(path=>document.querySelector("#review-file-title").textContent===path,path);await ready();
}
async function backTo(key) {
  for(let i=0;i<12;i++) {
    const previous=await state();if(previous.history===key)return;
    await page.getByRole("button",{name:"Back",exact:true}).click();
    await page.waitForFunction(previous=>history.state?.vizziReview?.key!==previous,previous.history);await ready();
  }
  throw new Error("History return exceeded visit bound");
}
async function seek(side,name,fromEnd=false) {
  const outline=page.locator(`#${side}-outline`),scan={side,name,fromEnd,pages:[]};report.outlineScans.push(scan);
  await outline.locator("button").first().focus();await page.keyboard.press(fromEnd?"End":"Home");const seen=new Set();
  for(let attempt=0;attempt<100;attempt++) {
    await paint(page);
    await page.waitForFunction(side=>{const o=document.getElementById(`${side}-outline`),b=o.querySelector('button[tabindex="0"]');if(!b || b!==document.activeElement)return false;const r=b.getBoundingClientRect(),bounds=o.getBoundingClientRect();return r.top>=bounds.top-1 && r.bottom<=bounds.bottom+1;},side);
    const visible=await outline.evaluate(o=>{const bounds=o.getBoundingClientRect();return [...o.querySelectorAll("button")].filter(b=>{const r=b.getBoundingClientRect();return r.top>=bounds.top && r.bottom<=bounds.bottom;}).map(b=>({id:b.dataset.declaration,name:b.querySelector("span").textContent,index:Number(b.dataset.index)}));});
    scan.pages.push(visible);const target=visible.find(row=>row.name===name);
    if(target) {await outline.locator(`button[data-declaration="${target.id}"]`).click();await page.waitForFunction(({side,id})=>document.querySelector(`#${side}-outline [aria-current="true"]`)?.dataset.declaration===id,{side,id:target.id});await ready();return;}
    const key=visible.map(row=>row.index).join(",");assert.ok(!seen.has(key),`Declaration ${name} not found`);seen.add(key);await page.keyboard.press(fromEnd?"PageUp":"PageDown");
  }
  throw new Error(`Declaration search exceeded page limit: ${name}`);
}
try {
  report.fixtureBefore=verify("before");
  report.sourceSha256=Object.fromEntries(await Promise.all(["experiments/review-workflow/run.mjs","experiments/review-workflow/walkthrough-audit.js","crates/vizzi/web/review-app.js","crates/vizzi/web/review-controller.js","crates/vizzi/web/review-windows.js","crates/vizzi/web/inspector.js","crates/vizzi/web/review.html","crates/vizzi/web/review.css"].map(async name=>[name,hash(await readFile(name))])));
  report.vizziRevision=command("git",["rev-parse","HEAD"],join(temporary,"revision")).toString().trim();
  report.phase="build";if(!options.noBuild)buildHost();report.binarySha256=hash(await readFile("target/release/vizzi"));
  report.phase="host_start";let started=performance.now();host=await startHost({root,cache:join(temporary,"cache"),startupTimeoutMs:60000},1);report.hostReadyMs=performance.now()-started;
  report.phase="browser_start";chromium=await browser({desktopBus:false});report.browser=chromium.version();
  context=await chromium.newContext({viewport:{width:1440,height:1000},recordVideo:{dir:join(output,"video"),size:{width:1440,height:1000}}});
  await context.tracing.start({screenshots:true,snapshots:true,sources:false});traceStarted=true;page=await context.newPage();page.setDefaultTimeout(15000);video=page.video();
  page.on("pageerror",error=>report.pageErrors.push(error.message));
  page.on("response",response=>{
    const url=new URL(response.url());if(!url.pathname.startsWith("/api/"))return;
    if(url.pathname==="/api/comparisons/events"){report.reviewStreamObserved=true;return;}
    if(url.pathname.endsWith("/status"))report.responseErrors.push("Browser issued a status-polling request");
    const job=(async()=>{
      if(response.status()>=400) {
        if(response.status()===409 && plannedExpiryToken && decodeURIComponent(url.pathname.split("/")[3])===plannedExpiryToken){report.expectedConflicts=(report.expectedConflicts ?? 0)+1;return;}
        throw new Error(`Unexpected HTTP ${response.status()}: ${url.pathname}`);
      }
      if(response.status()!==200 || !/\/(files|window|diff)$/.test(url.pathname))return;
      const bytes=await response.body(),body=JSON.parse(bytes),audit=auditResponse(url.href,body);assert.ok(report.responses.length<1000,"Response-audit bound");report.responses.push({...audit,payloadBytes:bytes.length});
      if(audit.kind==="files")for(const row of body.files)if(paths.has(row.path))files.set(`${body.token}:${row.path}`,{...row,token:body.token,comparison:body.comparison});
      if(audit.kind==="window") {const key=[body.token,body.side,body.window.file,body.window.version].join(":");assert.ok(windows.has(key) || windows.size<32,"Window-binding cache bound");const list=windows.get(key) ?? [];list.push(body);if(list.length>8)list.shift();windows.set(key,list);}
    })().catch(error=>report.responseErrors.push(error.message));jobs.add(job);void job.finally(()=>jobs.delete(job));
  });
  await page.route("**/*",route=>new URL(route.request().url()).origin===host.url?route.continue():route.abort());
  report.phase="walkthrough";await page.goto(`${host.url}/review`);await checkpoint("initial","Open the review page without capturing an implicit baseline.");
  await page.getByLabel("Baseline revision",{exact:true}).fill(oracle.base_revision);await page.getByLabel("Subtree",{exact:true}).fill(".");
  started=performance.now();await page.getByRole("button",{name:"Compare with working tree",exact:true}).click();await page.waitForFunction(()=>Boolean(document.querySelector("#review-workspace").dataset.token));await ready();report.firstCaptureReadyMs=performance.now()-started;
  const initial=await checkpoint("comparison","Observe the frozen baseline, root scope and two changed paths.");expectedToken=initial.token;assert.equal(initial.fileButtons,oracle.changed_paths.length);
  await choose(oracle.source_path);const changed=await checkpoint("compiler-changes","Select the changed Rust source and inspect its textual hunks.");
  const headers=changed.hunks.map(h=>h.label.replace(/^\d+: /,""));assert.deepEqual(headers,oracle.variants[manifest.variant].base_to_working_hunks);
  for(let i=0;i<changed.hunks.length;i++) {const h=changed.hunks[i];await page.locator(`#review-hunks button[data-hunk="${h.id}"]`).click();await page.waitForFunction(id=>document.querySelector('#review-hunks [aria-current="true"]')?.dataset.hunk===id,h.id);await checkpoint(`hunk-${i+1}`,"Reveal the corresponding base and working ranges through the hunk control.");}
  await seek("working","read_file");await seek("base","read_file");const policy=await checkpoint("read-policy","Inspect both source-read policies without inferring semantic equivalence.");
  assert.ok(policy.panes.base.rows.some(row=>row.text.includes("ErrorKind::NotFound")));assert.ok(policy.panes.working.rows.some(row=>row.text.includes("ErrorKind::NotFound")));
  report.lookupRegressionObserved=policy.panes.working.rows.some(row=>row.text.includes("ErrorKind::PermissionDenied"));assert.equal(report.lookupRegressionObserved,oracle.variants[manifest.variant].verdict==="lookup_regression");
  await page.getByLabel("Include unchanged context").check();await ready();await filter("source_reader.rs");await choose("src/compiler/source_reader.rs");await seek("working","SourceReader");await checkpoint("reader-contract","Follow unchanged source and its raw-byte reader contract on the selected side.");
  await backTo(policy.history);const returned=await checkpoint("review-return","Restore the compiler policy and both captured reading positions.");
  assert.equal(returned.panes.base.declaration,policy.panes.base.declaration);assert.equal(returned.panes.base.sourceTop,policy.panes.base.sourceTop);assert.equal(returned.panes.working.sourceTop,policy.panes.working.sourceTop);
  await filter("src/compiler/source.rs");await choose("src/compiler/source.rs");await seek("working","CompilerSource");await checkpoint("source-identity","Inspect input/resolved path and diagnostic-name ownership.");
  await filter(oracle.source_path);await choose(oracle.source_path);await seek("working","file_reads_stop_after_extension_fallback_and_preserve_other_io_errors",true);await checkpoint("regression-test","Locate the existing lookup contract through keyboard declaration navigation.");
  for(let i=0;i<6;i++) {const current=await state();if(current.panes.working.rows.some(row=>row.text.includes("PermissionDenied")))break;await page.locator("#working-source").focus();await page.keyboard.press("PageDown");await page.waitForTimeout(120);}
  const cases=await checkpoint("permission-error-case","Read the existing permission-error read-attempt expectation.");assert.ok(cases.panes.working.rows.some(row=>row.text.includes("PermissionDenied")));
  await filter(oracle.untracked_path);await choose(oracle.untracked_path);const document=await checkpoint("untracked-document","Account for the added Markdown document without requiring an analyser.");assert.equal(document.panes.base.renderedRows,0);assert.ok(document.panes.working.rows.length>0);
  await page.goBack();await page.waitForFunction(path=>document.querySelector("#review-file-title").textContent===path,oracle.source_path);const nativeReturn=await checkpoint("browser-back","Use native browser history to return to the regression-test evidence.");assert.equal(nativeReturn.panes.working.sourceTop,cases.panes.working.sourceTop);
  plannedExpiryToken=nativeReturn.token;expectedToken=undefined;await page.getByRole("button",{name:"Refresh working",exact:true}).click();await page.waitForFunction(token=>document.querySelector("#review-workspace").dataset.token!==token,plannedExpiryToken);const refreshed=await checkpoint("refresh","Capture working source again against the same resolved baseline.");expectedToken=refreshed.token;assert.equal(refreshed.comparison,nativeReturn.comparison);
  await page.getByRole("button",{name:"Back",exact:true}).click();await page.waitForFunction(()=>!document.querySelector("#review-notice").hidden && document.querySelector("#review-notice").textContent.includes("expired"));const expired=await checkpoint("expired-return","Keep the fresh view while explaining that the old comparison visit expired.");assert.equal(expired.token,refreshed.token);
  await page.getByRole("button",{name:"Open current comparison",exact:true}).click();await checkpoint("recovery","Explicitly reopen the current comparison.");
  await page.getByRole("button",{name:"Close comparison",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#review-workspace").dataset.token==="");const closed=await checkpoint("closed","Retire the pair without displaying old source under an empty selection.");for(const pane of Object.values(closed.panes))assert.equal(pane.renderedRows,0);
  await drain();assert.deepEqual(report.pageErrors,[]);assert.deepEqual(report.responseErrors,[]);assert.equal(report.reviewStreamObserved,true);report.completed=true;report.phase="complete";
} catch(error) {report.failure={message:error.message,stack:error.stack};process.exitCode=1;}
finally {
  const cleanup=async(name,action)=>{try{await action();}catch(error){report.cleanupErrors.push({name,message:error.message});report.completed=false;process.exitCode=1;}};
  if(traceStarted)await cleanup("trace",()=>context.tracing.stop({path:join(output,"trace.zip")}));
  if(context)await cleanup("context",()=>context.close());
  if(video)await cleanup("video",async()=>{await video.saveAs(join(output,"walkthrough.webm"));await rm(join(output,"video"),{recursive:true,force:true});});
  if(chromium)await cleanup("browser",()=>chromium.close());
  if(host)await cleanup("host",()=>host.close());
  await cleanup("fixture",async()=>{report.fixtureAfter=verify("after");assert.deepEqual(report.fixtureAfter,report.fixtureBefore);});
  await cleanup("temporary",()=>rm(temporary,{recursive:true,force:true}));
  report.finished=new Date().toISOString();await writeFile(join(output,"observations.json"),JSON.stringify(report,null,2)+"\n",{flag:"wx"});
}
if(report.failure)console.error(`${report.phase}: ${report.failure.message}`);
