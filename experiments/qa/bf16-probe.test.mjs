import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {gunzipSync} from 'node:zlib';
import {json} from './support.mjs';
import {probePlan} from './token-probe-support.mjs';
import {assertProbeReference,comparePrecisionProbes} from './precision-probe-support.mjs';
const spec=await json('experiments/qa/library/token-probe/cohort.json');
const preflight=JSON.parse(gunzipSync(await readFile('experiments/qa/results/token-probe/preflight.json.gz')));
assert.deepEqual(assertProbeReference(preflight,structuredClone(preflight),spec),{conditions:16,candidateSpellings:6});
for(const mutate of [p=>{p.candidates.json.No++;},p=>{delete p.conditions[Object.keys(p.conditions)[0]];},p=>{p.conditions[Object.keys(p.conditions)[0]].tokenIds[0]++;},p=>{p.conditions[Object.keys(p.conditions)[0]].messages=[];},p=>{p.conditions[Object.keys(p.conditions)[0]].renderedPrompt='changed';},p=>{p.conditions[Object.keys(p.conditions)[0]].request.cache_prompt=true;}]){const p=structuredClone(preflight);mutate(p);assert.throws(()=>assertProbeReference(preflight,p,spec));}
const partial=structuredClone(preflight);delete partial.conditions[Object.keys(partial.conditions)[0]];assert.throws(()=>assertProbeReference(partial,partial,spec),'two matching incomplete preflights are not sufficient');
const margin=value=>({value,lower:value,upper:value});
const rows=probePlan(spec).map(p=>{const input=preflight.conditions[p.case.id+'/'+p.system.id+'/'+p.prefix.id];return {id:p.id,case:p.case.id,system:p.system.id,prefix:p.prefix.id,repeat:p.repeat,promptSha256:input.promptSha256,promptTokens:input.tokenIds.length,request:input.request,measurement:{marginYesNo:margin(p.case.id.endsWith('changed')?-1:0),winner:'No'}};});
const control={model:{sha256:'a'.repeat(64)},preflight,rows},candidate=structuredClone(control);candidate.model.sha256='b'.repeat(64);
candidate.rows[0].measurement.marginYesNo=margin(1);
const result=comparePrecisionProbes(control,candidate,spec);assert.equal(result.pairs.length,32);assert.equal(result.pairs[0].delta.value,1);assert.equal(result.pairs[1].delta.value,0);assert.equal(result.effects.control.length,8);assert.equal(result.effects.candidate.length,8);
for(const mutate of [c=>{c.model.sha256=control.model.sha256;},c=>{delete c.model.sha256;},c=>{c.model.sha256='x'+'b'.repeat(64);},c=>{c.model.sha256='b'.repeat(64)+'x';},c=>{c.rows.pop();},c=>{c.rows.push({...c.rows[0],id:'extra'});},c=>{c.rows[0].request={...c.rows[0].request,seed:7};},c=>{c.rows[0].promptSha256='changed';},c=>{c.rows[0].promptTokens++;}]){const c=structuredClone(candidate);mutate(c);assert.throws(()=>comparePrecisionProbes(control,c,spec));}
const badControl=structuredClone(control);badControl.rows[0].request={...badControl.rows[0].request,cache_prompt:true};assert.throws(()=>comparePrecisionProbes(badControl,candidate,spec));
const censored=structuredClone(candidate);censored.rows[0].measurement.marginYesNo={value:null,lower:2,upper:null};const bounded=comparePrecisionProbes(control,censored,spec).pairs[0];assert.deepEqual(bounded.delta,{value:null,lower:2,upper:null});
console.log('precision probe: complete identical preflights, exact pairing, extra-row rejection and bounded model deltas');
