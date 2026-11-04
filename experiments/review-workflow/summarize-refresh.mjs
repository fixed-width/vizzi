import assert from "node:assert/strict";
import {readFile,writeFile} from "node:fs/promises";
import {join} from "node:path";
import {createHash} from "node:crypto";
import {summary} from "../browser/statistics.mjs";

const [output,...inputs]=process.argv.slice(2);
assert.ok(output && inputs.length===2,"Use OUTPUT.json LINUX_RESULTS PYTORCH_RESULTS");
const oldBytes=await readFile("experiments/review-workflow/results/startup/summary.json"),baseline=JSON.parse(oldBytes);
const sha=b=>createHash("sha256").update(b).digest("hex");
const report={schema:1,baselineSha256:sha(oldBytes),cases:[]};
for(const input of inputs) {
  const bytes=await readFile(join(input,"measurements.json")),data=JSON.parse(bytes),sessions=data.sessions;
  const old=baseline.cases.find(c=>c.name===data.case);assert.ok(old);
  assert.equal(data.completed,true);assert.equal(data.originalCheckoutUnchanged,true);assert.equal(sessions.length,3);
  for(const k of ["revision","scope","indexPaths","scopePaths"])assert.equal(data[k],old[k]);
  for(const s of sessions){assert.equal(s.completed,true);assert.deepEqual(s.errors,[]);assert.equal(s.liveIndexStayedDormant,true);assert.equal(s.startupStatus.snapshot,null);assert.equal(s.finalLiveStatus.snapshot,null);}
  const refreshes=sessions.flatMap(s=>s.refreshes),proofs=sessions.flatMap(s=>s.proofs),phases=sessions.flatMap(s=>["inspection","scroll","refreshActivity"].map(k=>s[k]));
  assert.equal(refreshes.length,15);assert.ok(refreshes.every(r=>!r.selectionReset && r.oldToken!==r.newToken && r.oldVersion!==r.newVersion));
  const fetches=phases.flatMap(p=>p.resources).filter(r=>r.status>=200 && r.status<300 && r.endpoint!=="refresh");
  report.cases.push({name:data.case,revision:data.revision,scope:data.scope,indexPaths:data.indexPaths,scopePaths:data.scopePaths,binarySha256:data.binarySha256,rawReportSha256:sha(bytes),sourceSha256:data.sourceSha256,
    before:old.deferred,
    after:{hostReady:summary(sessions.map(s=>s.hostReadyMs)),firstComparison:summary(sessions.map(s=>s.firstComparisonMs)),spawnToFirstReview:summary(sessions.map(s=>s.firstReviewReadySinceSpawnMs)),
      refresh:summary(refreshes.map(r=>r.captureMs)),editedSourceReady:summary(refreshes.map(r=>r.editedSourceReadyMs)),selectionResets:refreshes.filter(r=>r.selectionReset).length,
      selection:Object.fromEntries(["file","hunk","declaration"].map(k=>[k,summary(sessions.flatMap(s=>s.latencies.filter(l=>l.kind===k).map(l=>l.ms)))])),
      frames:summary(phases.flatMap(p=>p.frameIntervals)),longTasks:phases.flatMap(p=>p.longTasks),longTaskObserverAvailable:phases.every(p=>p.longTasksSupported),
      projectionFetch:summary(fetches.map(r=>r.duration)),payloadBytesMax:Math.max(...fetches.map(r=>r.decodedBodySize)),
      nativeHighWaterBytes:Math.max(...sessions.map(s=>s.finalMemory.highWaterBytes)),rendererHeapAfterBytes:Math.max(...sessions.map(s=>s.heapAfter.usedSize)),
      sourceRowsPerPaneMax:Math.max(...proofs.map(p=>p.maxRenderedRows),...phases.flatMap(p=>p.dom.panes.map(x=>x.sourceRows))),outlineButtonsPerPaneMax:Math.max(...proofs.map(p=>p.maxOutlineButtons),...phases.flatMap(p=>p.dom.panes.map(x=>x.outlineButtons))),
      fileButtonsMax:Math.max(...phases.map(p=>p.dom.fileButtons)),hunkButtonsMax:Math.max(...phases.map(p=>p.dom.hunkButtons)),verifiedSourceRows:proofs.reduce((n,p)=>n+p.verifiedRows,0),liveIndexStayedDormant:true,originalCheckoutUnchanged:true},
    hardware:data.hardware,gpu:data.gpu});
}
assert.equal(new Set(report.cases.map(c=>c.name)).size,2);assert.equal(new Set(report.cases.map(c=>c.binarySha256)).size,1);
report.limits=["Sequential cohorts; warm filesystem caches, fresh analysis caches, one parser worker and uncontrolled external activity.","Action timings include automation observation and rAF waits. Extra edited-source verification/paint waits remain even when no re-selection is needed.","Duration over33ms fields are arithmetic counts; frame cadence comparisons apply only to rAF intervals.","Scoped finite samples and software rendering do not qualify timing tails, whole-repository review, physical presentation, GPU rendering, a 16-GB/editor envelope or human usability."];
await writeFile(output,JSON.stringify(report,null,2)+"\n",{flag:"wx"});
console.log(JSON.stringify(report.cases.map(c=>({name:c.name,refresh:c.after.refresh,editedSourceReady:c.after.editedSourceReady,selectionResets:c.after.selectionResets})),null,2));
