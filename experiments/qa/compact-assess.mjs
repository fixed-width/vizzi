import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {json} from './support.mjs';
import {casesPath} from './compact-support.mjs';

const median=values=>{
 const sorted=values.filter(Number.isFinite).sort((a,b)=>a-b),middle=Math.floor(sorted.length/2);
 return sorted.length?(sorted.length%2?sorted[middle]:(sorted[middle-1]+sorted[middle])/2):null;
};
const percentile=(values,p)=>{const sorted=values.filter(Number.isFinite).sort((a,b)=>a-b);return sorted.length?sorted[Math.ceil(sorted.length*p)-1]:null;};
export function assess(rows,library,grading){
 const cases=new Map(library.cases.map(c=>[c.id,c])),grades=new Map(grading.grades.map(g=>[g.id,g]));
 assert.equal(grades.size,grading.grades.length,'duplicate grades');
 assert.equal(new Set(rows.map(r=>r.id)).size,rows.length,'duplicate responses');
 assert.equal(grades.size,rows.length,'grade every response exactly once');
 const decisions=rows.map(row=>{
  const item=cases.get(row.case),grade=grades.get(row.id);assert.ok(item && grade,row.id);
  assert.equal(grade.facts.length,item.required_facts.length,row.id);
  assert.ok(grade.facts.every(f=>['supported','missing','contradicted'].includes(f)),row.id);
  assert.ok(Array.isArray(grade.unsupportedClaims) && grade.unsupportedClaims.every(c=>typeof c==='string'));
  assert.equal(typeof grade.citationsSupport,'boolean');assert.equal(typeof grade.roleConfusion,'boolean');assert.ok(grade.notes);
  const supported=grade.facts.filter(f=>f==='supported').length,answerable=item.expected_status==='answered';
  const statusCorrect=row.answer?.status===item.expected_status;
  const taskSuccess=Boolean(statusCorrect && supported===grade.facts.length && !grade.unsupportedClaims.length && row.provenanceValid && (!answerable || grade.citationsSupport) && !row.runtimeError && !row.formatError && row.finish!=='length');
  return {...grade,id:row.id,case:row.case,arm:row.arm,split:row.split,answerable,statusCorrect,supportedFacts:supported,taskSuccess};
 });
 const summaries=[];
 for(const {split,population} of [{split:'all',population:'all'},{split:'development',population:'all'},{split:'held_out',population:'all'},{split:'all',population:'repository'},{split:'all',population:'temporal'}])for(const arm of new Set(rows.map(r=>r.arm))){
  const selected=rows.filter(r=>r.arm===arm && (split==='all' || r.split===split) && (population==='all' || (population==='temporal'?r.repository==='changing':r.repository!=='changing')));if(!selected.length)continue;
  const chosen=new Set(selected.map(r=>r.id)),group=decisions.filter(d=>chosen.has(d.id)),answerable=group.filter(d=>d.answerable),negative=group.filter(d=>!d.answerable);
  summaries.push({split,population,arm,cases:group.length,answerable:answerable.length,taskSuccess:answerable.filter(d=>d.taskSuccess).length,unanswerable:negative.length,correctRefusals:negative.filter(d=>d.taskSuccess).length,supportedFacts:answerable.reduce((n,d)=>n+d.supportedFacts,0),facts:answerable.reduce((n,d)=>n+d.facts.length,0),allFacts:answerable.filter(d=>d.supportedFacts===d.facts.length).length,statusCorrect:group.filter(d=>d.statusCorrect).length,unsupportedAnswers:group.filter(d=>d.unsupportedClaims.length).length,roleConfusions:group.filter(d=>d.roleConfusion).length,sourceChecks:selected.filter(r=>r.provenanceValid).length,answerableRefusals:selected.filter(r=>cases.get(r.case).expected_status==='answered' && r.answer?.status==='insufficient_evidence').length,truncated:selected.filter(r=>r.finish==='length').length,formatErrors:selected.filter(r=>r.formatError).length,runtimeErrors:selected.filter(r=>r.runtimeError).length,medianEvidenceBytes:median(selected.map(r=>r.evidenceBytes)),medianPromptTokens:median(selected.map(r=>r.usage?.prompt_tokens)),medianOutputTokens:median(selected.map(r=>r.usage?.completion_tokens)),medianRetrievalMs:median(selected.map(r=>r.retrievalMs)),medianFirstOutputMs:median(selected.map(r=>r.firstAnswerMs)),medianMs:median(selected.map(r=>r.totalMs)),p95Ms:percentile(selected.map(r=>r.totalMs),.95)});
 }
 return {schema:1,grader:grading.grader,limitations:grading.limitations,summaries,decisions};
}

if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 const args=process.argv.slice(2),option=name=>{const i=args.indexOf(name);assert.ok(i>=0 && args[i+1],`missing ${name}`);return args[i+1];};
 const rows=(await readFile(option('--input'),'utf8')).trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));
 const selectedLibrary=args.includes('--library')?option('--library'):casesPath;
 const result=assess(rows,await json(selectedLibrary),await json(option('--grades')));
 const memoryPath=args.includes('--memory')?option('--memory'):undefined;
 if(memoryPath){const memory=await json(memoryPath);result.memory={peakPssKiB:memory.peak_pss_kib,peakRssKiB:memory.peak_rss_kib,observedPeakPssGiB:memory.peak_pss_kib/1024**2,capVerified:memory.cap_verified,mode:memory.mode,intervalMs:memory.interval_ms,samples:memory.samples.length,observerErrors:memory.errors,success:memory.success,childExitCode:memory.child_exit_code};}
 await writeFile(option('--output'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify(result.summaries));
}
