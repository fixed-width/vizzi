import assert from 'node:assert/strict';
import {probePlan,probeMessages,probeRequest,readProbe,probeEffects} from './token-probe-support.mjs';
import {prompt} from './model.mjs';
import {json} from './support.mjs';
const spec=await json('experiments/qa/library/token-probe/cohort.json');
const plan=probePlan(spec);assert.equal(plan.length,32);assert.equal(new Set(plan.map(p=>p.id)).size,32);
assert.deepEqual(plan.filter(p=>p.repeat===2).map(p=>p.case.id+'/'+p.system.id+'/'+p.prefix.id),plan.filter(p=>p.repeat===1).map(p=>p.case.id+'/'+p.system.id+'/'+p.prefix.id).reverse());
for(const mutate of [s=>{s.schema=2;},s=>{s.repeats=1;},s=>{s.topCount=0;},s=>{s.reservedGenerations=1;},s=>{s.cases[0].id='reserved.card.original';},s=>{s.cases[1].id=s.cases[0].id;},s=>{s.systems[0].text='replacement';},s=>{s.systems[1].text='';},s=>{s.systems.reverse();},s=>{s.prefixes.reverse();},s=>{s.prefixes[0].text='other';},s=>{s.prefixes[0].choices.No='Yes';}]){const s=structuredClone(spec);mutate(s);assert.throws(()=>probePlan(s));}
const nonStringSystem=structuredClone(spec);nonStringSystem.systems[1].text=['wrong'];assert.throws(()=>probePlan(nonStringSystem));
const q={repository:'fixture',question:'Does finish release?',expected:'HIDDEN_GOLD'},context={snapshot:'s',evidence:[{label:1,path:'a.rs',start_line:1,end_line:1,text:'fn finish() {}'}]};
const production=probeMessages(q,context,spec.systems[0]),diagnostic=probeMessages(q,context,spec.systems[1]);assert.deepEqual(production,prompt(q,context,{protocol:'ids',metadata:true,wordLimit:300}));assert.deepEqual(diagnostic[1],production[1]);assert.equal(diagnostic[0].content,spec.systems[1].text);assert.ok(!JSON.stringify(diagnostic).includes('HIDDEN_GOLD'));
const request=probeRequest([1,2,3],512);assert.deepEqual(request.prompt,[1,2,3]);assert.equal(request.n_predict,1);assert.equal(request.n_probs,512);assert.equal(request.post_sampling_probs,false);assert.equal(request.cache_prompt,false);assert.equal(request.stream,false);assert.equal(request.temperature,0);assert.equal(request.seed,42);assert.equal(request.return_tokens,true);assert.ok(!('grammar' in request || 'response_format' in request || 'logit_bias' in request));
for(const tokens of [[],[1.5],[-1],['1']])assert.throws(()=>probeRequest(tokens,512));
assert.throws(()=>probeRequest([1,'bad'],512));assert.equal(probeRequest([0],512).prompt[0],0);
assert.equal(probeRequest([1],1).n_probs,1);assert.equal(probeRequest([1],2048).n_probs,2048);
assert.equal(probeRequest(Array(8192).fill(1),512).prompt.length,8192);assert.throws(()=>probeRequest(Array(8193).fill(1),512));for(const n of [0,2049,1.5])assert.throws(()=>probeRequest([1],n));
const candidates={Yes:1,No:2,Unknown:3};
const response=(values)=>({tokens_predicted:1,tokens_evaluated:3,tokens:[values[0].id],generation_settings:{post_sampling_probs:false,n_probs:4},completion_probabilities:[{id:values[0].id,token:'top',top_logprobs:values}]});
const values=[{id:2,token:'No',logprob:-0.5},{id:1,token:'Yes',logprob:-2},{id:9,token:'other',logprob:-3},{id:3,token:'Unknown',logprob:-4}];
const observed=readProbe(response(values),candidates,3,4);assert.equal(observed.marginYesNo.value,-1.5);assert.equal(observed.choices.Yes.logprob,-2);assert.equal(observed.winner,'No');assert.equal(observed.top.id,2);assert.ok(observed.observedChoiceMass<1);
assert.deepEqual(observed.marginYesNo,{value:-1.5,lower:-1.5,upper:-1.5});assert.deepEqual(observed.sampled,{id:2,token:'top'});
assert.ok(Math.abs(observed.observedChoiceMass-(Math.exp(-.5)+Math.exp(-2)+Math.exp(-4)))<1e-12);
const zeroId=values.map(v=>({...v,id:v.id===1?0:v.id}));assert.equal(readProbe(response(zeroId),{Yes:0,No:2,Unknown:3},3,4).choices.Yes.id,0);
const noUnknown=readProbe(response([...values.slice(0,3),{id:10,token:'other2',logprob:-4}]),candidates,3,4);assert.equal(noUnknown.choices.Unknown.censored,true);assert.equal(noUnknown.choices.Unknown.logprob,null);assert.equal(noUnknown.choices.Unknown.upperBoundLogprob,-4);assert.equal(noUnknown.winner,null);assert.equal(noUnknown.marginYesNo.value,-1.5);
const noYes=readProbe(response([{id:2,logprob:-0.5},{id:3,logprob:-2},{id:9,logprob:-3},{id:10,logprob:-4}]),candidates,3,4);assert.deepEqual(noYes.marginYesNo,{value:null,lower:null,upper:-3.5});
const noNo=readProbe(response([{id:1,logprob:-0.5},{id:3,logprob:-2},{id:9,logprob:-3},{id:10,logprob:-4}]),candidates,3,4);assert.deepEqual(noNo.marginYesNo,{value:null,lower:3.5,upper:null});
const neither=readProbe(response([{id:9,logprob:-0.5},{id:3,logprob:-2},{id:10,logprob:-3},{id:11,logprob:-4}]),candidates,3,4);assert.deepEqual(neither.marginYesNo,{value:null,lower:null,upper:null});
const outside=readProbe(response([{id:9,logprob:-0.5},{id:2,logprob:-2},{id:1,logprob:-3},{id:3,logprob:-4}]),candidates,3,4);assert.equal(outside.top.id,9);assert.equal(outside.winner,'No');assert.ok(outside.observedChoiceMass<.3);
const tied=readProbe(response([{id:2,logprob:-1.5},{id:1,logprob:-1.5},{id:9,logprob:-3},{id:3,logprob:-4}]),candidates,3,4);assert.equal(tied.winner,null);assert.equal(tied.marginYesNo.value,0);
for(const mutate of [r=>{r.tokens_predicted=2;},r=>{r.tokens_evaluated=4;},r=>{r.generation_settings.post_sampling_probs=true;},r=>{r.completion_probabilities=[];},r=>{r.completion_probabilities[0].top_logprobs[1].id=2;},r=>{r.completion_probabilities[0].top_logprobs[0].logprob=1;},r=>{r.completion_probabilities[0].top_logprobs[2].logprob=-0.1;},r=>{r.completion_probabilities[0].top_logprobs[3].logprob=null;}]){const r=structuredClone(response(values));mutate(r);assert.throws(()=>readProbe(r,candidates,3,4));}
for(const mutate of [r=>{r.tokens=[];},r=>{r.tokens=[99];},r=>{r.generation_settings.n_probs=3;},r=>{r.completion_probabilities[0].top_logprobs.pop();},r=>{r.completion_probabilities[0].top_logprobs[0].id=-1;},r=>{r.completion_probabilities[0].top_logprobs[0].logprob='-1';},r=>{r.completion_probabilities[0].top_logprobs.forEach(t=>t.logprob=-0.1);}]){const r=structuredClone(response(values));mutate(r);assert.throws(()=>readProbe(r,candidates,3,4));}
for(const mutate of [r=>{r.tokens.push(9);},r=>{r.completion_probabilities.push(r.completion_probabilities[0]);},r=>{const v=r.completion_probabilities[0].top_logprobs;[v[1],v[2]]=[v[2],v[1]];}]){const r=structuredClone(response(values));mutate(r);assert.throws(()=>readProbe(r,candidates,3,4));}
const zeroLog=[{id:2,logprob:0},{id:1,logprob:-30},{id:9,logprob:-31},{id:3,logprob:-32}];assert.equal(readProbe(response(zeroLog),candidates,3,4).choices.No.logprob,0);
const positiveLog=structuredClone(zeroLog);positiveLog[0].logprob=1e-6;assert.throws(()=>readProbe(response(positiveLog),candidates,3,4));
const tolerance=values.map(v=>({...v,logprob:Math.log(1.00001/4)}));readProbe(response(tolerance),candidates,3,4);
assert.throws(()=>readProbe(response(values),{Yes:1,No:1,Unknown:3},3,4));assert.throws(()=>readProbe(response(values),{Yes:1,No:2,Other:3},3,4));assert.throws(()=>readProbe(response(values),{Yes:1,No:2,Unknown:-1},3,4));
const rows=plan.map(p=>({...p,case:p.case.id,system:p.system.id,prefix:p.prefix.id,measurement:{...observed,marginYesNo:{value:p.case.id.endsWith('changed')?-2:1,lower:p.case.id.endsWith('changed')?-2:1,upper:p.case.id.endsWith('changed')?-2:1}}}));
const effects=probeEffects(rows,spec);assert.equal(effects.length,8);assert.ok(effects.every(e=>e.sourceShift.value===3 && e.permutationShift.value===0));
assert.throws(()=>probeEffects(rows.slice(1),spec));
assert.throws(()=>probeEffects([...rows,{...rows[0],id:'unplanned'}],spec),'extra unique observations must not be silently ignored');
const censored=structuredClone(rows);censored[0].measurement.marginYesNo={value:null,lower:2,upper:null};assert.equal(probeEffects(censored,spec).find(e=>e.repeat===1 && e.system==='production' && e.prefix==='json').sourceShift.value,null);
assert.deepEqual(probeEffects(censored,spec).find(e=>e.repeat===1 && e.system==='production' && e.prefix==='json').sourceShift,{value:null,lower:4,upper:null});
for(const [a,b,wanted]of [
 [{value:null,lower:null,upper:2},{value:-2,lower:-2,upper:-2},{value:null,lower:null,upper:4}],
 [{value:1,lower:1,upper:1},{value:null,lower:-2,upper:null},{value:null,lower:null,upper:3}],
 [{value:1,lower:1,upper:1},{value:null,lower:null,upper:-2},{value:null,lower:3,upper:null}],
 [{value:null,lower:null,upper:null},{value:-2,lower:-2,upper:-2},{value:null,lower:null,upper:null}]
]){const r=structuredClone(rows);r.find(x=>x.case==='evidence.pins.original' && x.system==='production' && x.prefix==='json' && x.repeat===1).measurement.marginYesNo=a;r.find(x=>x.case==='evidence.pins.changed' && x.system==='production' && x.prefix==='json' && x.repeat===1).measurement.marginYesNo=b;assert.deepEqual(probeEffects(r,spec).find(e=>e.repeat===1 && e.system==='production' && e.prefix==='json').sourceShift,wanted);}
const distinct=structuredClone(rows);distinct.find(r=>r.case==='evidence.pins.original' && r.system==='diagnostic' && r.prefix==='prose' && r.repeat===2).measurement.marginYesNo={value:5,lower:5,upper:5};
const changedEffects=probeEffects(distinct,spec);assert.equal(changedEffects.find(e=>e.repeat===2 && e.system==='diagnostic' && e.prefix==='prose').sourceShift.value,7);assert.equal(changedEffects.find(e=>e.repeat===1 && e.system==='diagnostic' && e.prefix==='prose').sourceShift.value,3);
for(const mutate of [r=>{r.id='unknown';},r=>{r.case='other';},r=>{r.system='other';},r=>{r.prefix='other';},r=>{r.repeat=3;}]){const bad=structuredClone(rows);mutate(bad[0]);assert.throws(()=>probeEffects(bad,spec));}
console.log('token probe: fixed readouts, uncensored probabilities, bounded missing choices and source-sensitive margins');
