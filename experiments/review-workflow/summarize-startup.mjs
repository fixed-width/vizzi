// Aggregate a completed deferred-startup cohort against the recorded eager cohort.
import assert from "node:assert/strict";
import {readFile,writeFile} from "node:fs/promises";
import {createHash} from "node:crypto";
import {join} from "node:path";
import {summary} from "../browser/statistics.mjs";

const [output,...inputs]=process.argv.slice(2);
assert.ok(output && inputs.length===2,"Use OUTPUT.json LINUX_RESULTS PYTORCH_RESULTS");
const baselineBytes=await readFile("experiments/review-workflow/results/scale/summary.json");
const baseline=JSON.parse(baselineBytes),sha=b=>createHash("sha256").update(b).digest("hex");
const report={schema:1,baselineSha256:sha(baselineBytes),cases:[]};
for(const input of inputs) {
  const bytes=await readFile(join(input,"measurements.json")),data=JSON.parse(bytes),sessions=data.sessions;
  assert.equal(data.completed,true);assert.equal(data.originalCheckoutUnchanged,true);
  assert.equal(sessions.length,3);
  const old=baseline.cases.find(c=>c.name===data.case);assert.ok(old);
  for(const key of ["revision","scope","indexPaths","scopePaths"])assert.equal(data[key],old[key]);
  for(const s of sessions) {
    assert.equal(s.completed,true);assert.deepEqual(s.errors,[]);assert.equal(s.liveIndexStayedDormant,true);
    for(const status of [s.startupStatus,s.finalLiveStatus]) {
      assert.equal(status.readiness,"not_started");assert.equal(status.snapshot,null);assert.equal(status.measurements,null);
    }
  }
  const resources=sessions.flatMap(s=>["inspection","scroll","refreshActivity"].flatMap(p=>s[p].resources)).filter(r=>r.status>=200 && r.status<300);
  const proofs=sessions.flatMap(s=>s.proofs),refreshes=sessions.flatMap(s=>s.refreshes);
  const doms=sessions.flatMap(s=>["inspection","scroll","refreshActivity"].map(p=>s[p].dom));
  report.cases.push({
    name:data.case,revision:data.revision,scope:data.scope,indexPaths:data.indexPaths,scopePaths:data.scopePaths,
    binarySha256:data.binarySha256,rawReportSha256:sha(bytes),sourceSha256:data.sourceSha256,
    eager:{hostReady:old.startup,firstComparison:old.firstComparison,nativeHighWaterBytes:old.hostHighWaterBytesMax},
    deferred:{hostReady:summary(sessions.map(s=>s.hostReadyMs)),firstComparison:summary(sessions.map(s=>s.firstComparisonMs)),
      spawnToFirstReview:summary(sessions.map(s=>s.firstReviewReadySinceSpawnMs)),
      selection:Object.fromEntries(["file","hunk","declaration"].map(kind=>[kind,summary(sessions.flatMap(s=>s.latencies.filter(l=>l.kind===kind).map(l=>l.ms)))])),
      refresh:summary(refreshes.map(r=>r.captureMs)),editedSourceReady:summary(refreshes.map(r=>r.editedSourceReadyMs)),
      selectionResets:refreshes.filter(r=>r.selectionReset).length,
      frames:summary(sessions.flatMap(s=>["inspection","scroll","refreshActivity"].flatMap(p=>s[p].frameIntervals))),
      longTasks:sessions.flatMap(s=>["inspection","scroll","refreshActivity"].flatMap(p=>s[p].longTasks)),
      projectionFetch:summary(resources.filter(r=>r.endpoint!=="refresh").map(r=>r.duration)),
      nativeHighWaterBytes:Math.max(...sessions.map(s=>s.finalMemory.highWaterBytes)),
      rendererHeapAfterBytes:Math.max(...sessions.map(s=>s.heapAfter.usedSize)),
      payloadBytesMax:Math.max(...resources.map(r=>r.decodedBodySize)),
      sourceRowsPerPaneMax:Math.max(...proofs.map(p=>p.maxRenderedRows),...doms.flatMap(d=>d.panes.map(p=>p.sourceRows))),
      outlineButtonsPerPaneMax:Math.max(...proofs.map(p=>p.maxOutlineButtons),...doms.flatMap(d=>d.panes.map(p=>p.outlineButtons))),
      fileButtonsMax:Math.max(...doms.map(d=>d.fileButtons)),hunkButtonsMax:Math.max(...doms.map(d=>d.hunkButtons)),
      verifiedSourceRows:proofs.reduce((n,p)=>n+p.verifiedRows,0),
      allRefreshBindingsPinned:refreshes.every(r=>r.oldToken!==r.newToken && r.oldVersion!==r.newVersion && r.baseVersion),
      liveIndexStayedDormant:true,originalCheckoutUnchanged:true},
    hardware:data.hardware,gpu:data.gpu,
  });
}
assert.equal(new Set(report.cases.map(c=>c.name)).size,2);
assert.equal(new Set(report.cases.map(c=>c.binarySha256)).size,1);
report.limits=[
  "Sequential cohorts on the same host; fresh analysis caches, warm filesystem caches, uncontrolled external activity.",
  "Eager first comparison reused preceding live indexing; deferred first comparison has no live warmup. These are different task paths, not an isolated parser throughput comparison.",
  "Host readiness ends at the emitted URL. Spawn-to-first-review includes browser context/navigation/form automation and DOM/paint waits.",
  "Finite scoped samples, software-rendered Chromium and host-only RSS do not qualify whole-repository review, physical presentation, a 16-GB/editor envelope or human usability.",
  "Duration-summary over33ms/over50ms fields are arithmetic counts. Only rAF interval summaries compare frame cadence; action durations are not stutter counts.",
];
await writeFile(output,JSON.stringify(report,null,2)+"\n",{flag:"wx"});
console.log(JSON.stringify(report.cases.map(c=>({name:c.name,eager:c.eager,deferred:c.deferred})),null,2));
