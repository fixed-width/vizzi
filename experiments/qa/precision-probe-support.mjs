import assert from 'node:assert/strict';
import {probePlan,probeEffects,marginDifference} from './token-probe-support.mjs';

export function assertProbeReference(reference,current,spec){
 const expected=probePlan(spec).filter(p=>p.repeat===1).map(p=>p.case.id+'/'+p.system.id+'/'+p.prefix.id).sort();
 assert.deepEqual(Object.keys(reference.conditions).sort(),expected);assert.deepEqual(Object.keys(current.conditions).sort(),expected);
 assert.deepEqual(reference.candidates,current.candidates);
 for(const key of expected)assert.deepEqual(reference.conditions[key],current.conditions[key]);
 return {conditions:expected.length,candidateSpellings:Object.values(current.candidates).reduce((n,m)=>n+Object.keys(m).length,0)};
}
export function comparePrecisionProbes(control,candidate,spec){
 for(const model of [control.model,candidate.model])assert.match(model.sha256,/^[a-f0-9]{64}$/);
 assert.notEqual(control.model.sha256,candidate.model.sha256);
 const inputs=assertProbeReference(control.preflight,candidate.preflight,spec);
 const effects={control:probeEffects(control.rows,spec),candidate:probeEffects(candidate.rows,spec)},plan=probePlan(spec);
 const maps=[control,candidate].map(run=>new Map(run.rows.map(row=>[row.id,row])));
 const pairs=plan.map(p=>{
  const a=maps[0].get(p.id),b=maps[1].get(p.id),input=control.preflight.conditions[p.case.id+'/'+p.system.id+'/'+p.prefix.id];
  for(const row of [a,b]){assert.deepEqual(row.request,input.request);assert.equal(row.promptSha256,input.promptSha256);assert.equal(row.promptTokens,input.tokenIds.length);}
  return {id:p.id,case:p.case.id,system:p.system.id,prefix:p.prefix.id,repeat:p.repeat,expected:p.case.expected,control:a.measurement.marginYesNo,candidate:b.measurement.marginYesNo,delta:marginDifference(b.measurement.marginYesNo,a.measurement.marginYesNo),controlWinner:a.measurement.winner,candidateWinner:b.measurement.winner};
 });
 return {schema:1,inputs,pairs,effects};
}
