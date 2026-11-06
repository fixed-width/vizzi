import assert from 'node:assert/strict';
import {prompt} from './model.mjs';

export function probePlan(spec){
 assert.equal(spec.schema,1);assert.equal(spec.repeats,2);assert.equal(spec.topCount,512);assert.equal(spec.reservedGenerations,0);
 assert.deepEqual(spec.cases,[{id:'evidence.pins.original',expected:'Yes'},{id:'evidence.pins.changed',expected:'No'},{id:'evidence.pins.relabel',expected:'Yes'},{id:'evidence.pins.missing',expected:'Unknown'}]);
 assert.deepEqual(spec.systems.map(s=>s.id),['production','diagnostic']);assert.equal(spec.systems[0].text,null);assert.ok(typeof spec.systems[1].text==='string' && spec.systems[1].text.length);
 assert.deepEqual(spec.prefixes.map(p=>p.id),['json','prose']);assert.equal(spec.prefixes[0].text,'{"answer":"');assert.equal(spec.prefixes[1].text,'Answer:');
 for(const p of spec.prefixes)assert.deepEqual(p.choices,Object.fromEntries(['Yes','No','Unknown'].map(label=>[label,(p.id==='prose'?' ':'')+label])));
 const unique=spec.cases.flatMap(item=>spec.systems.flatMap(system=>spec.prefixes.map(prefix=>({case:item,system,prefix}))));
 return [1,2].flatMap(repeat=>(repeat===1?unique:[...unique].reverse()).map(p=>({...p,repeat,id:`${p.case.id}/${p.system.id}/${p.prefix.id}/repeat-${repeat}`})));
}
export function probeMessages(question,context,system){
 const messages=prompt(question,context,{protocol:'ids',metadata:true,wordLimit:300});
 if(system.text!==null)messages[0].content=system.text;
 return messages;
}
export function probeRequest(tokens,topCount){
 assert.ok(tokens.length>0 && tokens.length<=8192 && tokens.every(n=>Number.isInteger(n) && n>=0));
 assert.ok(Number.isInteger(topCount) && topCount>=1 && topCount<=2048);
 return {prompt:tokens,n_predict:1,n_probs:topCount,post_sampling_probs:false,temperature:0,seed:42,stream:false,cache_prompt:false,return_tokens:true,repeat_penalty:1,presence_penalty:0,frequency_penalty:0};
}
export function readProbe(response,candidates,promptTokens,topCount){
 assert.deepEqual(Object.keys(candidates).sort(),['No','Unknown','Yes']);assert.equal(new Set(Object.values(candidates)).size,3);assert.ok(Object.values(candidates).every(n=>Number.isInteger(n) && n>=0));
 assert.equal(response.tokens_predicted,1);assert.equal(response.tokens_evaluated,promptTokens);assert.equal(response.tokens.length,1);
 assert.equal(response.generation_settings.post_sampling_probs,false);assert.equal(response.generation_settings.n_probs,topCount);assert.equal(response.completion_probabilities.length,1);
 const position=response.completion_probabilities[0],top=position.top_logprobs;assert.equal(position.id,response.tokens[0]);assert.equal(top.length,topCount);
 assert.equal(new Set(top.map(t=>t.id)).size,top.length);
 for(const [i,t]of top.entries()){assert.ok(Number.isInteger(t.id) && t.id>=0 && Number.isFinite(t.logprob) && t.logprob<=0);if(i)assert.ok(top[i-1].logprob>=t.logprob);}
 assert.ok(top.reduce((n,t)=>n+Math.exp(t.logprob),0)<=1.00001);
 const cutoff=top.at(-1).logprob,choices=Object.fromEntries(Object.entries(candidates).map(([label,id])=>{const entry=top.find(t=>t.id===id);return [label,{id,logprob:entry?.logprob??null,censored:!entry,upperBoundLogprob:entry?.logprob??cutoff}];}));
 const yes=choices.Yes.logprob,no=choices.No.logprob;
 const marginYesNo=yes!==null && no!==null?{value:yes-no,lower:yes-no,upper:yes-no}:yes===null && no!==null?{value:null,lower:null,upper:cutoff-no}:yes!==null?{value:null,lower:yes-cutoff,upper:null}:{value:null,lower:null,upper:null};
 const ranked=Object.entries(choices).filter(([,v])=>!v.censored).sort((a,b)=>b[1].logprob-a[1].logprob);
 const winner=ranked.length===3 && ranked[0][1].logprob!==ranked[1][1].logprob?ranked[0][0]:null;
 return {choices,marginYesNo,winner,top:top[0],sampled:{id:position.id,token:position.token},observedChoiceMass:ranked.reduce((n,[,v])=>n+Math.exp(v.logprob),0),cutoff};
}
function difference(a,b){
 return {value:a.value!==null && b.value!==null?a.value-b.value:null,lower:a.lower!==null && b.upper!==null?a.lower-b.upper:null,upper:a.upper!==null && b.lower!==null?a.upper-b.lower:null};
}
export function probeEffects(rows,spec){
 const plan=probePlan(spec);assert.equal(rows.length,plan.length);assert.equal(new Set(rows.map(r=>r.id)).size,rows.length);
 const lookup=new Map(rows.map(r=>[r.id,r]));
 for(const p of plan){const r=lookup.get(p.id);assert.ok(r);assert.equal(r.case,p.case.id);assert.equal(r.system,p.system.id);assert.equal(r.prefix,p.prefix.id);assert.equal(r.repeat,p.repeat);}
 return spec.systems.flatMap(system=>spec.prefixes.flatMap(prefix=>[1,2].map(repeat=>{
  const row=condition=>lookup.get(`evidence.pins.${condition}/${system.id}/${prefix.id}/repeat-${repeat}`),margin=condition=>row(condition).measurement.marginYesNo;
  return {system:system.id,prefix:prefix.id,repeat,originalMargin:margin('original'),changedMargin:margin('changed'),sourceShift:difference(margin('original'),margin('changed')),permutationShift:difference(margin('original'),margin('relabel')),missingWinner:row('missing').measurement.winner};
 })));
}
