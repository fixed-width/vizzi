import assert from "node:assert/strict";
import {spawn,spawnSync} from "node:child_process";
import {createInterface} from "node:readline";
import {once} from "node:events";
import {mkdtemp,readFile,writeFile,mkdir,rm,open} from "node:fs/promises";
import {tmpdir} from "node:os";
import {join,resolve} from "node:path";
import {createHash} from "node:crypto";
import {createReviewController} from "../../crates/vizzi/web/review-controller.js";

const fixture=resolve(process.argv[2]),output=resolve(process.argv[3]);
const manifest=JSON.parse(await readFile(join(fixture,"manifest.json"),"utf8"));
const oracle=JSON.parse(await readFile("experiments/review-workflow/fixtures/oracle.json","utf8"));
const root=join(fixture,"workspace");assert.ok(!output.startsWith(root+"/"));await mkdir(output,{recursive:false});
const verify=()=>{const r=spawnSync("python3",["-B","experiments/review-workflow/verify.py","--fixture",fixture],{stdio:"inherit"});assert.equal(r.status,0);};
verify();
const temporary=await mkdtemp(join(tmpdir(),"vizzi-review-interface-")),log=await open(join(output,"host.log"),"w");
const child=spawn(resolve("target/release/vizzi"),["--stdio","--root",root,"--cache",temporary,"--workers","1"],{stdio:["pipe","pipe",log.fd]});
const pending=new Map(),operations=[],steps=[],markers=[];let id=0,revision=0,view,status,controller,report;
const lines=createInterface({input:child.stdout});
lines.on("line",line=>{const packet=JSON.parse(line),request=pending.get(packet.id);pending.delete(packet.id);request?.resolve(packet);});
child.on("error",error=>{for(const p of pending.values())p.reject(error);});
async function transport(uri,options={}) {
  const key=++id;const packet={id:key,uri,method:options.method ?? "GET"};if(options.body)packet.body=JSON.parse(options.body);
  const response=await new Promise((resolve,reject)=>{pending.set(key,{resolve,reject});child.stdin.write(JSON.stringify(packet)+"\n");});
  operations.push({uri,method:packet.method,status:response.status});
  if(response.status>=400){let message=response.body;try{message=JSON.parse(message).message;}catch{}throw Object.assign(new Error(message),{status:response.status});}
  return response.content_type.startsWith("application/json")?JSON.parse(response.body):response.body;
}
const waitFor=async condition=>{const deadline=Date.now()+60000;while(!condition()){if(Date.now()>deadline)throw new Error("Controller operation timed out");await new Promise(r=>setTimeout(r,20));}};
const sourceCache=new Map();let checked=0;
async function checkSource(current) {
  for(const side of ["base","working"]) {
    const body=current.windows[side];if(!body?.window)continue;
    const path=current.file.path,key=side+path;
    if(!sourceCache.has(key)) {
      let bytes;
      if(side==="working")bytes=await readFile(join(root,path));
      else {
        const temp=await open(join(temporary,"source"),"w");
        const result=spawnSync("git",["-C",root,"show",`${oracle.base_revision}:${path}`],{stdio:["ignore",temp.fd,"inherit"],env:{...process.env,GIT_OPTIONAL_LOCKS:"0",GIT_CONFIG_GLOBAL:"/dev/null",GIT_CONFIG_NOSYSTEM:"1"}});await temp.close();assert.equal(result.status,0);bytes=await readFile(join(temporary,"source"));
      }
      sourceCache.set(key,bytes.toString("utf8").split("\n"));
    }
    for(const row of body.window.lines){const text=sourceCache.get(key)[row.number-1].replace(/\r$/,"");assert.equal(row.text,Buffer.from(text).subarray(row.column,row.end_column).toString("utf8"));checked++;}
  }
}
async function step(name) {
  await checkSource(view);steps.push({name,token:view.metadata.token,path:view.file?.path,selection:view.selection,source:Object.fromEntries(["base","working"].map(side=>[side,view.windows[side]?.window?.lines ?? []]))});
}
try {
  controller=createReviewController({transport,onView:next=>{view=next;},onStatus:next=>{status=next;},capture:()=>Object.fromEntries(["base","working"].map(side=>[side,view?.windows[side]?.position ?? {sourceTop:0,sourceLeft:0,outlineTop:0,cursorIndex:0,column:0}])),history:{push:m=>markers.push(m),replace:m=>markers.push(m),go(){}},onError:error=>{throw error;}});
  const subscription=controller.updates();
  async function synchronise() {const deadline=Date.now()+60000;do{assert.ok(Date.now()<deadline,"capture timed out");status=await transport("/api/comparisons/status");await new Promise(r=>setTimeout(r,20));}while(status.pending);assert.equal(status.error,null);subscription.receive({schema:1,stream:"finite-diagnostic",revision:String(++revision),status});await waitFor(()=>view?.metadata.token===status.active?.token);}
  const assets=[];for(const uri of ["/review","/review-app.js","/review-controller.js","/review-windows.js","/review.css","/windows.js","/inspector.js"]){const data=await transport(uri);assert.ok(data.length>100);assets.push({uri,sha256:createHash("sha256").update(data).digest("hex")});}
  await controller.create({revision:oracle.base_revision,scope:"."});await synchronise();
  assert.equal(view.files.total,2);assert.deepEqual(view.files.files.map(f=>f.path).sort(),[...oracle.changed_paths].sort());
  const compiler=view.files.files.find(f=>f.path===oracle.source_path);
  await controller.navigate({file:compiler});await step("changed-compiler");
  const header=h=>{const range=r=>`${r.end-r.start?r.start+1:r.start},${r.end-r.start}`;return `@@ -${range(h.base.lines)} +${range(h.working.lines)} @@`;};
  const headers=view.diff.hunks.map(header);assert.deepEqual(headers,oracle.variants[manifest.variant].base_to_working_hunks);
  const fact=name=>{const found=view.windows.working.window.outline.find(f=>f.name===name);assert.ok(found,name);return {id:found.id,index:found.index,start_line:found.start_line};};
  await controller.navigate({side:"working",entity:fact("read_file")});await step("read-policy");
  const policy=view.windows.working.window.lines;
  assert.ok(policy.some(r=>r.text.includes("ErrorKind::NotFound")));
  const widened=policy.some(r=>r.text.includes("ErrorKind::PermissionDenied"));assert.equal(widened,manifest.variant==="variant-a");
  const policyMarker=markers.at(-1);
  await controller.navigate({filter:"source_reader.rs",unchanged:true,offset:0});const reader=view.files.files.find(f=>f.path==="src/compiler/source_reader.rs");assert.ok(reader);await controller.navigate({file:reader});await step("reader-contract");
  assert.ok(view.windows.working.window.lines.some(r=>r.text.includes("trait SourceReader")));
  assert.equal(await controller.traverse(policyMarker),true);assert.equal(view.selection.entity,policyMarker?view.query.entity.id:null);await step("exact-review-return");
  await controller.navigate({filter:"src/compiler/source.rs",unchanged:true,offset:0});const identity=view.files.files.find(f=>f.path==="src/compiler/source.rs");assert.ok(identity);await controller.navigate({file:identity});await step("source-identity");
  await controller.navigate({file:compiler,filter:oracle.source_path,unchanged:true});
  await controller.navigate({side:"working",entity:fact("file_reads_stop_after_extension_fallback_and_preserve_other_io_errors")});await step("regression-test");
  assert.ok(view.windows.working.window.lines.some(r=>r.text.includes("PermissionDenied")));
  const token=view.metadata.token;await controller.refresh();await synchronise();assert.notEqual(view.metadata.token,token);const current=view;
  assert.equal(await controller.traverse(policyMarker),false);assert.equal(view,current);
  report={schema:1,variant:manifest.variant,verified:true,operator:"Reference-aware agent via the browser controller/native finite adapter, without a DOM or Chromium.",scope:"Controller/native integration only. Status is supplied by finite diagnostic probes; SSE wire delivery, DOM, keyboard, layout and browser walkthrough are unverified here.",assets,headers,regressionDetected:widened,verifiedSourceRows:checked,steps,operations,baseline:oracle.base_revision,binarySha256:createHash("sha256").update(await readFile("target/release/vizzi")).digest("hex")};
  await controller.close();
  console.log(`${manifest.variant}: controller/native source, hunks, unchanged context, regression evidence and expiry verified`);
} finally {
  const exited=once(child,"exit");child.stdin.end();const timer=setTimeout(()=>child.kill("SIGKILL"),10000);const [code,signal]=await exited;clearTimeout(timer);lines.close();await log.close();await rm(temporary,{recursive:true,force:true});verify();assert.equal(code,0);assert.equal(signal,null);
}
report.fixtureReverified=true;await writeFile(join(output,"interface.json"),JSON.stringify(report,null,2)+"\n");
