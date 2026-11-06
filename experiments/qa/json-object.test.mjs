import assert from 'node:assert/strict';
import {generationRequest} from './model.mjs';
import {decodingPlan,compareDecoding,assertDecodingRequests} from './decoding-study.mjs';
import {json} from './support.mjs';
const library=await json('experiments/qa/library/evidence-use/development.json'),spec=await json('experiments/qa/library/json-object/cohort.json');
const context={snapshot:'fixture',evidence:[{label:1,path:'a.rs',start_line:1,end_line:1,text:'fn f() {}'}]},limits={output_tokens:1024},options={protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first'};
const item={repository:'fixture',question:'Which function runs?',required_facts:[{text:'HIDDEN_GOLD'}]};
const a=generationRequest(item,context,limits,options),b=generationRequest(item,context,limits,{...options,constraint:'json_object'});
assert.deepEqual(b,{...a,response_format:{type:'json_object'}});assert.ok(!('schema' in b.response_format));assert.ok(!JSON.stringify(b.messages).includes('HIDDEN_GOLD'));
assertDecodingRequests(a,b,spec.kind);
for(const change of [r=>{r.max_tokens=12;},r=>{delete r.response_format;},r=>{r.response_format.type='text';},r=>{r.response_format.schema={};},r=>{r.messages=[];}]){const r=structuredClone(b);change(r);assert.throws(()=>assertDecodingRequests(a,r,spec.kind));}
assert.throws(()=>assertDecodingRequests(a,b,'unknown'));
const plan=decodingPlan(library,spec);assert.equal(plan.length,48);assert.equal(new Set(plan.map(p=>p.id)).size,48);assert.ok(plan.every(p=>p.case.split==='development'));
assert.deepEqual(plan.slice(0,4).map(p=>p.arm.id),['schema','json_object','json_object','schema']);
for(const id of spec.caseIds){const rounds=[1,2].map(n=>plan.filter(p=>p.case.id===id && p.repeat===n).map(p=>p.arm.id));assert.deepEqual(rounds[1],rounds[0].reverse());}
const badSpec=structuredClone(spec);badSpec.arms[1].constraint='none';assert.throws(()=>decodingPlan(library,badSpec));
const rows=plan.map(p=>{const request=generationRequest(p.case,context,limits,{...options,constraint:p.arm.constraint});const answer={answer:'Fixture',status:p.case.expected_status,citations:p.case.expected_status==='answered'?[1]:[]};return {id:p.id,case:p.case.id,repeat:p.repeat,arm:p.arm.id,constraint:p.arm.constraint,split:'development',repository:'fixture',request,messages:request.messages,context,contextSha256:'same',renderedPrompt:'same',answer,raw:JSON.stringify(answer),provenanceValid:true,finish:'stop',usage:{prompt_tokens:12}};});
const grades={grader:'fixture',limitations:[],grades:plan.map(p=>({id:p.id,facts:p.case.required_facts.map(()=> 'supported'),unsupportedClaims:[],citationsSupport:true,roleConfusion:false,...(p.case.expected_status==='insufficient_evidence'?{rawAbstention:true}:{}),notes:'fixture'}))};
const result=compareDecoding(rows,library,spec,grades);assert.equal(result.pairs.length,24);assert.ok(result.pairs.every(p=>p.schemaTask && p.json_objectTask && p.schemaContent && p.json_objectContent));
assert.deepEqual(result.formats,spec.arms.map(a=>({arm:a.id,responses:24,jsonParsed:24,schemaValid:24,orderCompliant:24,nativeReferencesValid:24,contentCorrect:24,taskSuccess:24})));
const different=structuredClone(rows),r=different.find(r=>r.arm==='json_object');r.answer={result:'Correct fact'};r.raw=JSON.stringify(r.answer);r.provenanceValid=false;
const output=compareDecoding(different,library,spec,grades);assert.equal(output.pairs[0].json_objectTask,false);assert.equal(output.pairs[0].json_objectContent,true);assert.equal(output.formats[1].jsonParsed,24);assert.equal(output.formats[1].schemaValid,23);assert.equal(output.formats[0].taskSuccess,24);
console.log('generic JSON: only schema removed, paired admission/order and nonconforming object content kept separate');
