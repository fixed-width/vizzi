import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {assess} from './compact-assess.mjs';
import {json} from './support.mjs';

export function diagnose(rows,library,grading){
 const base=assess(rows,library,grading),decisions=new Map(base.decisions.map(d=>[d.id,d])),lookup=new Map(library.cases.map(c=>[c.id,c]));
 const families=new Map(),pairs=[];
 for(const item of library.cases){
  const pair=rows.filter(r=>r.case===item.id);assert.equal(pair.length,2,item.id);
  assert.ok(pair.some(r=>r.arm==='status_first') && pair.some(r=>r.arm==='answer_first'));
  assert.equal(pair[0].contextSha256,pair[1].contextSha256,'source must match within a pair');assert.deepEqual(pair[0].messages,pair[1].messages,'prompt must match within a pair');
  if(families.has(item.parent_case))assert.equal(families.get(item.parent_case),pair[0].contextSha256,'whole family must share source');else families.set(item.parent_case,pair[0].contextSha256);
  const status=pair.find(r=>r.arm==='status_first'),answer=pair.find(r=>r.arm==='answer_first');
  pairs.push({case:item.id,parent_case:item.parent_case,granularity:item.granularity,statusFirst:decisions.get(status.id).taskSuccess,answerFirst:decisions.get(answer.id).taskSuccess,statusFirstStatus:status.answer?.status,answerFirstStatus:answer.answer?.status});
 }
 const summaries=[],paired=[];
 for(const granularity of ['all','broad','narrow']){
  const cases=pairs.filter(p=>granularity==='all' || p.granularity===granularity);
  paired.push({granularity,cases:cases.length,helped:cases.filter(p=>!p.statusFirst && p.answerFirst).length,hurt:cases.filter(p=>p.statusFirst && !p.answerFirst).length,bothSucceeded:cases.filter(p=>p.statusFirst && p.answerFirst).length,bothFailed:cases.filter(p=>!p.statusFirst && !p.answerFirst).length});
  for(const arm of ['status_first','answer_first']){
   const group=rows.filter(r=>r.arm===arm && (granularity==='all' || lookup.get(r.case).granularity===granularity));
   const d=group.map(r=>decisions.get(r.id));
   summaries.push({granularity,arm,cases:group.length,successes:d.filter(x=>x.taskSuccess).length,supportedFacts:d.reduce((n,x)=>n+x.supportedFacts,0),facts:d.reduce((n,x)=>n+x.facts.length,0),falseRefusals:group.filter(r=>r.answer?.status==='insufficient_evidence').length,unsupportedAnswers:d.filter(x=>x.unsupportedClaims.length).length,sourceChecks:group.filter(r=>r.provenanceValid).length,orderCompliant:group.filter(r=>r.orderCompliant).length});
  }
 }
 return {schema:1,grader:grading.grader,limitations:grading.limitations,summaries,paired,pairs,orderCompliance:['status_first','answer_first'].map(arm=>({arm,responses:rows.filter(r=>r.arm===arm).length,compliant:rows.filter(r=>r.arm===arm && r.orderCompliant).length})),performance:base.summaries.filter(s=>s.split==='all' && s.population==='all'),decisions:base.decisions};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 const args=process.argv.slice(2),option=name=>{const i=args.indexOf(name);assert.ok(i>=0 && args[i+1],`missing ${name}`);return args[i+1];};
 const rows=(await readFile(option('--input'),'utf8')).trim().split('\n').filter(Boolean).map(line=>JSON.parse(line)),result=diagnose(rows,await json(option('--library')),await json(option('--grades')));
 if(args.includes('--memory')){const memory=await json(option('--memory'));result.memory={observedPeakPssGiB:memory.peak_pss_kib/1024**2,peakPssKiB:memory.peak_pss_kib,peakRssKiB:memory.peak_rss_kib,intervalMs:memory.interval_ms,capVerified:memory.cap_verified,success:memory.success,errors:memory.errors};}
 await writeFile(option('--output'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(result.summaries));
}
