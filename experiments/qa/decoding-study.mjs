import assert from 'node:assert/strict';
import {developmentRuns} from './evidence-study.mjs';
import {assess} from './compact-assess.mjs';

export function decodingPlan(library,spec){
 assert.equal(spec.schema,1);assert.equal(spec.kind,'response_format_ablation');assert.equal(spec.repeats,2);
 assert.deepEqual(spec.arms,[{id:'schema',constraint:'schema'},{id:'unconstrained',constraint:'none'}]);
 assert.ok(spec.caseIds.length && new Set(spec.caseIds).size===spec.caseIds.length);
 const cases=spec.caseIds.map(id=>{const item=library.cases.find(c=>c.id===id);assert.ok(item && item.split==='development','unknown/reserved decoding case');return item;});
 const runs=developmentRuns({cases},spec.repeats);
 return runs.flatMap(p=>{const index=cases.findIndex(c=>c.id===p.case.id),order=(index+p.repeat-1)%2?[...spec.arms].reverse():spec.arms;return order.map(arm=>({...p,id:p.id+'/'+arm.id,arm}));});
}
export function responseShape(answer){
 const object=answer!==null && typeof answer==='object' && !Array.isArray(answer),keys=object?Object.keys(answer):[];
 const orderCompliant=JSON.stringify(keys)===JSON.stringify(['answer','status','citations']);
 const schemaValid=Boolean(object && JSON.stringify([...keys].sort())===JSON.stringify(['answer','citations','status']) && typeof answer.answer==='string' && [...answer.answer].length>=1 && [...answer.answer].length<=2000 && ['answered','insufficient_evidence'].includes(answer.status) && Array.isArray(answer.citations) && answer.citations.length<=8 && answer.citations.every(n=>Number.isInteger(n) && n>=1 && n<=8));
 return {schemaValid,orderCompliant};
}
export function compareDecoding(rows,library,spec,grading){
 const plan=decodingPlan(library,spec);assert.equal(rows.length,plan.length);assert.equal(new Set(rows.map(r=>r.id)).size,rows.length);
 const selected={...library,cases:spec.caseIds.map(id=>library.cases.find(c=>c.id===id))};
 const assessment=assess(rows,selected,grading),decisions=new Map(assessment.decisions.map(d=>[d.id,d])),lookup=new Map(rows.map(r=>[r.id,r]));
 for(const p of plan){const row=lookup.get(p.id);assert.ok(row);assert.equal(row.case,p.case.id);assert.equal(row.repeat,p.repeat);assert.equal(row.arm,p.arm.id);assert.equal(row.constraint,p.arm.constraint);}
 const pairs=[];
 for(const p of plan.filter(p=>p.arm.id==='schema')){
  const a=lookup.get(p.id),b=lookup.get(`${p.case.id}/repeat-${p.repeat}/unconstrained`);
  assert.deepEqual(a.context,b.context);assert.equal(a.contextSha256,b.contextSha256);assert.deepEqual(a.messages,b.messages);assert.ok(typeof a.renderedPrompt==='string' && a.renderedPrompt.length);assert.equal(a.renderedPrompt,b.renderedPrompt);
  assert.ok(a.request.response_format);assert.ok(!Object.hasOwn(b.request,'response_format'));
  const {response_format,...request}=a.request;assert.deepEqual(request,b.request);
  for(const row of [a,b])assert.deepEqual(row.messages,row.request.messages);
  const content=row=>{
   const d=decisions.get(row.id);if(!d.answerable)assert.equal(typeof d.rawAbstention,'boolean');
   return Boolean(row.raw?.trim() && !d.unsupportedClaims.length && (d.answerable?d.supportedFacts===d.facts.length:d.rawAbstention));
  };
  const task=row=>decisions.get(row.id).taskSuccess && responseShape(row.answer).schemaValid;
  pairs.push({case:p.case.id,repeat:p.repeat,schemaTask:task(a),unconstrainedTask:task(b),schemaContent:content(a),unconstrainedContent:content(b),matchingPromptTokens:Number.isFinite(a.usage?.prompt_tokens) && Number.isFinite(b.usage?.prompt_tokens)?a.usage.prompt_tokens===b.usage.prompt_tokens:null});
 }
 const formats=spec.arms.map(arm=>{const selected=rows.filter(r=>r.arm===arm.id);return {arm:arm.id,responses:selected.length,jsonParsed:selected.filter(r=>r.answer!==undefined).length,schemaValid:selected.filter(r=>responseShape(r.answer).schemaValid).length,orderCompliant:selected.filter(r=>responseShape(r.answer).orderCompliant).length,nativeReferencesValid:selected.filter(r=>r.provenanceValid).length,contentCorrect:pairs.filter(p=>arm.id==='schema'?p.schemaContent:p.unconstrainedContent).length,taskSuccess:pairs.filter(p=>arm.id==='schema'?p.schemaTask:p.unconstrainedTask).length};});
 const checkedDecisions=assessment.decisions.map(d=>({...d,taskSuccess:d.taskSuccess && responseShape(lookup.get(d.id).answer).schemaValid}));
 return {schema:1,grader:assessment.grader,limitations:assessment.limitations,decisions:checkedDecisions,pairs,formats};
}
