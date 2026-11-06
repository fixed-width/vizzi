import assert from 'node:assert/strict';
import {generationRequest} from './model.mjs';
import {decodingPlan,responseShape,compareDecoding} from './decoding-study.mjs';
import {json} from './support.mjs';

const library=await json('experiments/qa/library/evidence-use/development.json'),spec=await json('experiments/qa/library/decoding/cohort.json');
const context={snapshot:'s',evidence:[{label:1,path:'a.rs',start_line:1,end_line:1,text:'fn check() {}'}]},limits={output_tokens:1024};
const options={protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first'};
const item={repository:'fixture',question:'Where is the check?',required_facts:[{text:'HIDDEN_GOLD'}]};
const control=generationRequest(item,context,limits,options),unconstrained=generationRequest(item,context,limits,{...options,constraint:'none'});
assert.deepEqual(generationRequest(item,context,limits,{...options,constraint:'schema'}),control);
const {response_format,...rest}=control;assert.deepEqual(unconstrained,rest);assert.ok(!('response_format' in unconstrained));assert.ok(response_format.schema);
assert.equal(response_format.type,'json_object');assert.deepEqual(response_format.schema.required,['answer','status','citations']);assert.equal(response_format.schema.properties.answer.maxLength,2000);assert.equal(response_format.schema.properties.citations.maxItems,8);
assert.ok(!JSON.stringify(control.messages).includes('HIDDEN_GOLD'));assert.equal(control.max_tokens,1024);assert.equal(control.temperature,0);assert.equal(control.seed,42);assert.equal(control.cache_prompt,true);assert.deepEqual(control.chat_template_kwargs,{enable_thinking:false});
assert.equal(control.stream,true);assert.deepEqual(control.stream_options,{include_usage:true});
for(const constraint of ['invalid','',false,0])assert.throws(()=>generationRequest(item,context,limits,{...options,constraint}));
assert.deepEqual(generationRequest(item,context,limits,{...options,constraint:null}),control);
assert.equal(generationRequest(item,context,limits).response_format.schema.properties.citations.maxItems,4);

const plan=decodingPlan(library,spec);assert.equal(plan.length,48);assert.equal(new Set(plan.map(p=>p.id)).size,48);
assert.deepEqual([...new Set(plan.map(p=>p.case.family))],['routing','versions','pins']);
for(const id of spec.caseIds){
 const rounds=[1,2].map(repeat=>plan.filter(p=>p.case.id===id && p.repeat===repeat));
 assert.equal(rounds[0].length,2);assert.deepEqual(rounds[1].map(p=>p.arm.id),rounds[0].map(p=>p.arm.id).reverse());
}
assert.equal(plan.filter((p,i)=>i%2===0 && p.arm.id==='schema').length,12);
assert.deepEqual(plan.slice(0,4).map(p=>p.arm.id),['schema','unconstrained','unconstrained','schema']);
for(const repeat of [1,2])assert.equal(plan.filter(p=>p.repeat===repeat).filter((p,i)=>i%2===0 && p.arm.id==='schema').length,6);
for(const mutate of [s=>{s.schema=2;},s=>{s.kind='other';},s=>{s.repeats=1;},s=>{s.caseIds=[];},s=>{s.caseIds.push(s.caseIds[0]);},s=>{s.caseIds[0]='reserved.hooks.original';},s=>{s.arms[0].constraint='none';},s=>{s.arms.reverse();}]){const s=structuredClone(spec);mutate(s);assert.throws(()=>decodingPlan(library,s));}
const reservedLibrary=structuredClone(library);reservedLibrary.cases[0].split='reserved';assert.throws(()=>decodingPlan(reservedLibrary,spec));

const valid={answer:'The check is present.',status:'answered',citations:[1]};
assert.deepEqual(responseShape(valid),{schemaValid:true,orderCompliant:true});
assert.deepEqual(responseShape({status:valid.status,answer:valid.answer,citations:valid.citations}),{schemaValid:true,orderCompliant:false});
for(const value of [undefined,null,'prose',[],{}, {...valid,extra:1},{...valid,answer:''},{...valid,answer:'a'.repeat(2001)},{...valid,status:'unknown'},{...valid,citations:['1']},{...valid,citations:[0]},{...valid,citations:[9]},{...valid,citations:[1.5]},{...valid,citations:Array(9).fill(1)}])assert.equal(responseShape(value).schemaValid,false);
for(const answer of [null,1,false,['Y'],{}])assert.equal(responseShape({...valid,answer}).schemaValid,false);
assert.equal(responseShape({...valid,answer:'😀'.repeat(2000),citations:Array(8).fill(8)}).schemaValid,true);
assert.equal(responseShape({...valid,answer:'Y'}).schemaValid,true);
assert.equal(responseShape({...valid,status:'insufficient_evidence',citations:[]}).schemaValid,true);

const rows=plan.map(p=>{
 const answer={answer:'Fixture',status:p.case.expected_status,citations:p.case.expected_status==='answered'?[1]:[]};
 const request=generationRequest(p.case,context,limits,{...options,constraint:p.arm.constraint});
 return {id:p.id,case:p.case.id,repeat:p.repeat,arm:p.arm.id,constraint:p.arm.constraint,split:'development',repository:'fixture',request,messages:request.messages,context,contextSha256:'same',renderedPrompt:'same',answer,raw:JSON.stringify(answer),provenanceValid:true,finish:'stop',usage:{prompt_tokens:10},totalMs:5};
});
const grading={grader:'fixture',limitations:[],grades:plan.map(p=>({id:p.id,facts:p.case.required_facts.map(()=> 'supported'),unsupportedClaims:[],citationsSupport:true,roleConfusion:false,...(p.case.expected_status==='insufficient_evidence'?{rawAbstention:true}:{}),notes:'fixture'}))};
const result=compareDecoding(rows,library,spec,grading);assert.equal(result.pairs.length,24);assert.ok(result.pairs.every(p=>p.schemaTask && p.unconstrainedTask && p.schemaContent && p.unconstrainedContent));
assert.ok(result.decisions.every(d=>d.taskSuccess));
assert.deepEqual(result.formats,spec.arms.map(a=>({arm:a.id,responses:24,jsonParsed:24,schemaValid:24,orderCompliant:24,nativeReferencesValid:24,contentCorrect:24,taskSuccess:24})));
assert.ok(result.pairs.every(p=>p.matchingPromptTokens));
const plain=structuredClone(rows),plainGrades=structuredClone(grading),target=plain.find(r=>r.arm==='unconstrained' && r.case==='evidence.routing.original' && r.repeat===1);
target.raw='dispatch handles Apply and calls validate.';delete target.answer;target.formatError='Unexpected token';target.provenanceValid=false;
const content=compareDecoding(plain,library,spec,plainGrades).pairs.find(p=>p.case===target.case && p.repeat===1);
assert.equal(content.unconstrainedTask,false);assert.equal(content.unconstrainedContent,true,'raw factual ability is separate from JSON task success');
assert.deepEqual(compareDecoding(plain,library,spec,plainGrades).formats[1],{arm:'unconstrained',responses:24,jsonParsed:23,schemaValid:23,orderCompliant:23,nativeReferencesValid:23,contentCorrect:24,taskSuccess:23});
assert.deepEqual(compareDecoding(plain,library,spec,plainGrades).formats[0],result.formats[0]);
const oversized=structuredClone(rows),long=oversized.find(r=>r.arm==='unconstrained');long.answer.answer='a'.repeat(2001);long.raw=JSON.stringify(long.answer);
assert.equal(compareDecoding(oversized,library,spec,grading).pairs[0].unconstrainedTask,false,'native answer bounds alone do not enforce the schema character limit');
assert.equal(compareDecoding(oversized,library,spec,grading).decisions.find(d=>d.id===long.id).taskSuccess,false,'per-response and paired task judgments must agree on schema validity');
for(const field of ['facts','unsupportedClaims']){
 const grades=structuredClone(grading),g=grades.grades.find(g=>g.id===target.id);
 if(field==='facts')g.facts=g.facts.map(()=> 'missing');else g.unsupportedClaims=['Wrong code behaviour'];
 const scored=compareDecoding(rows,library,spec,grades);assert.equal(scored.pairs.find(p=>p.case===target.case && p.repeat===1).unconstrainedContent,false);
 assert.equal(scored.formats[1].contentCorrect,23);assert.equal(scored.formats[1].taskSuccess,23);assert.deepEqual(scored.formats[0],result.formats[0]);
}
for(const field of ['messages','contextSha256','renderedPrompt','request']){
 const bad=structuredClone(rows);const row=bad.find(r=>r.arm==='unconstrained');if(field==='request')row.request.seed=7;else row[field]=field==='messages'?[]:'changed';
 assert.throws(()=>compareDecoding(bad,library,spec,grading));
}
assert.throws(()=>compareDecoding(rows.slice(1),library,spec,{...grading,grades:grading.grades.slice(1)}));
const noRender=structuredClone(rows);for(const r of noRender)delete r.renderedPrompt;assert.throws(()=>compareDecoding(noRender,library,spec,grading));
const emptyRender=structuredClone(rows);for(const r of emptyRender)r.renderedPrompt='';assert.throws(()=>compareDecoding(emptyRender,library,spec,grading));
const nonStringRender=structuredClone(rows),render=['same'];for(const r of nonStringRender)r.renderedPrompt=render;assert.throws(()=>compareDecoding(nonStringRender,library,spec,grading));
for(const mutate of [r=>{r.repeat=3;},r=>{r.arm='other';},r=>{r.constraint='other';},r=>{r.case='unknown';}]){const bad=structuredClone(rows);mutate(bad[0]);assert.throws(()=>compareDecoding(bad,library,spec,grading));}
const tokens=structuredClone(rows);tokens.find(r=>r.arm==='unconstrained').usage.prompt_tokens=11;assert.equal(compareDecoding(tokens,library,spec,grading).pairs[0].matchingPromptTokens,false);
const noTokens=structuredClone(rows);delete noTokens.find(r=>r.arm==='unconstrained').usage;assert.equal(compareDecoding(noTokens,library,spec,grading).pairs[0].matchingPromptTokens,null);
const noControlTokens=structuredClone(rows);delete noControlTokens[0].usage;assert.equal(compareDecoding(noControlTokens,library,spec,grading).pairs[0].matchingPromptTokens,null);
const contextChange=structuredClone(rows);contextChange[0].context={...contextChange[0].context,snapshot:'different'};assert.throws(()=>compareDecoding(contextChange,library,spec,grading));
const noSchema=structuredClone(rows);delete noSchema[0].request.response_format;assert.throws(()=>compareDecoding(noSchema,library,spec,grading));
const staleMessages=structuredClone(rows);for(const r of staleMessages.filter(r=>r.case===target.case && r.repeat===1))r.messages=[];assert.throws(()=>compareDecoding(staleMessages,library,spec,grading));
const wrongCase=structuredClone(rows);wrongCase[0].case='evidence.routing.changed';assert.throws(()=>compareDecoding(wrongCase,library,spec,grading));
const emptyOutput=structuredClone(rows),empty=emptyOutput.find(r=>r.arm==='unconstrained' && r.case==='evidence.routing.missing');empty.raw='';delete empty.answer;empty.provenanceValid=false;
assert.equal(compareDecoding(emptyOutput,library,spec,grading).pairs.find(p=>p.case===empty.case && p.repeat===empty.repeat).unconstrainedContent,false,'no generated text is not a successful abstention');
const bare=structuredClone(rows),bareGrades=structuredClone(grading),no=bare.find(r=>r.arm==='unconstrained' && r.case==='evidence.routing.missing');no.raw='No';delete no.answer;no.provenanceValid=false;bareGrades.grades.find(g=>g.id===no.id).rawAbstention=false;
assert.equal(compareDecoding(bare,library,spec,bareGrades).pairs.find(p=>p.case===no.case && p.repeat===no.repeat).unconstrainedContent,false);
for(const raw of [' \n\t',undefined,null]){
 const absent=structuredClone(emptyOutput),row=absent.find(r=>r.id===empty.id);row.raw=raw;
 assert.equal(compareDecoding(absent,library,spec,grading).pairs.find(p=>p.case===row.case && p.repeat===row.repeat).unconstrainedContent,false);
}
const missingJudgment=structuredClone(grading);delete missingJudgment.grades.find(g=>g.id===empty.id).rawAbstention;assert.throws(()=>compareDecoding(rows,library,spec,missingJudgment));
console.log('decoding: exact request-field ablation, balanced development-only plan, format/content separation and strict pairing');
