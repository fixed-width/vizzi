import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {assess} from './compact-assess.mjs';
import {json} from './support.mjs';

export function compareModels(baseline,candidate,library,baselineGrades,candidateGrades){
 const runs=[baseline,candidate],grades=[baselineGrades,candidateGrades],results=runs.map((rows,i)=>assess(rows,library,grades[i]));
 const ids=runs.map(rows=>new Set(rows.map(r=>r.model)));assert.ok(ids.every(set=>set.size===1));
 const models=ids.map(set=>[...set][0]);assert.notEqual(models[0],models[1]);
 const lookup=runs.map(rows=>new Map(rows.map(r=>[r.case,r]))),decisions=results.map(r=>new Map(r.decisions.map(d=>[d.case,d])));
 assert.ok(lookup.every(map=>map.size===library.cases.length));
 const pairs=[];
 for(const item of library.cases){
  const a=lookup[0].get(item.id),b=lookup[1].get(item.id);assert.ok(a && b,item.id);
  assert.equal(a.contextSha256,b.contextSha256,'model evidence must match');assert.deepEqual(a.messages,b.messages,'model messages must match');
  for(const row of [a,b]){assert.equal(row.fieldOrder,'answer_first');assert.equal(row.wordLimit,300);assert.equal(row.outputTokenLimit,1024);}
  pairs.push({case:item.id,granularity:item.granularity,baseline:decisions[0].get(item.id).taskSuccess,candidate:decisions[1].get(item.id).taskSuccess});
 }
 const summaries=[],paired=[];
 for(const granularity of ['all','broad','narrow']){
  const selected=pairs.filter(p=>granularity==='all' || p.granularity===granularity),known=new Set(selected.map(p=>p.case));
  paired.push({granularity,cases:selected.length,helped:selected.filter(p=>!p.baseline && p.candidate).length,hurt:selected.filter(p=>p.baseline && !p.candidate).length,bothSucceeded:selected.filter(p=>p.baseline && p.candidate).length,bothFailed:selected.filter(p=>!p.baseline && !p.candidate).length});
  for(let i=0;i<models.length;i++){
   const rows=runs[i].filter(r=>known.has(r.case)),d=results[i].decisions.filter(d=>known.has(d.case));
   summaries.push({model:models[i],granularity,cases:rows.length,successes:d.filter(x=>x.taskSuccess).length,supportedFacts:d.reduce((n,x)=>n+x.supportedFacts,0),facts:d.reduce((n,x)=>n+x.facts.length,0),falseRefusals:rows.filter(r=>r.answer?.status==='insufficient_evidence').length,unsupportedAnswers:d.filter(x=>x.unsupportedClaims.length).length,sourceChecks:rows.filter(r=>r.provenanceValid).length,orderCompliance:rows.filter(r=>r.orderCompliant).length});
  }
 }
 return {schema:1,models,graders:grades.map(g=>g.grader),limitations:grades.map(g=>g.limitations),summaries,paired,pairs,performance:results.map((r,i)=>({model:models[i],summary:r.summaries.find(s=>s.split==='all' && s.population==='all')})),decisions:results.map((r,i)=>({model:models[i],decisions:r.decisions}))};
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 const args=process.argv.slice(2),option=name=>{const i=args.indexOf(name);assert.ok(i>=0 && args[i+1],`missing ${name}`);return args[i+1];};
 const readRows=async path=>(await readFile(path,'utf8')).trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));
 const result=compareModels(await readRows(option('--baseline')),await readRows(option('--candidate')),await json(option('--library')),await json(option('--baseline-grades')),await json(option('--candidate-grades')));
 result.memory=[];
 for(const key of ['--baseline-memory','--candidate-memory']){const m=await json(option(key));result.memory.push({model:result.models[result.memory.length],observedPeakPssGiB:m.peak_pss_kib/1024**2,peakPssKiB:m.peak_pss_kib,peakRssKiB:m.peak_rss_kib,withinFourGiBTarget:m.peak_pss_kib<=4*1024**2,capVerified:m.cap_verified,intervalMs:m.interval_ms,success:m.success,errors:m.errors});}
 await writeFile(option('--output'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(result.summaries));
}
