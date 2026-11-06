import assert from 'node:assert/strict';

export function batchArgs(limits){
 const batch=limits.batch_tokens??512,physical=limits.ubatch_tokens??128;
 assert.ok(Number.isInteger(batch) && batch>=1 && batch<=8192,'invalid logical batch');
 assert.ok(Number.isInteger(physical) && physical>=1 && physical<=batch,'invalid physical batch');
 return ['--batch-size',String(batch),'--ubatch-size',String(physical)];
}
export function profileLimits(base,config,id){
 if(config===undefined){assert.equal(id,undefined);return base;}
 assert.equal(config.schema,1);assert.ok(Number.isInteger(config.request_timeout_ms) && config.request_timeout_ms>=base.request_timeout_ms && config.request_timeout_ms<=600000);
 const seen=new Set();for(const profile of config.profiles){assert.ok(profile.id && !seen.has(profile.id));seen.add(profile.id);assert.deepEqual(Object.keys(profile).sort(),['batch_tokens','id','ubatch_tokens']);batchArgs(profile);}
 const profile=config.profiles.find(p=>p.id===id);assert.ok(profile,'unknown runtime profile');
 return {...base,batch_tokens:profile.batch_tokens,ubatch_tokens:profile.ubatch_tokens,request_timeout_ms:config.request_timeout_ms};
}
export function selectedCases(cases,ids){
 if(ids===undefined)return cases;
 const names=ids.split(',');assert.equal(new Set(names).size,names.length);assert.ok(names.length && names.every(id=>cases.some(c=>c.id===id)),'unknown selected case');
 return cases.filter(c=>names.includes(c.id));
}
export function selectResourceProfile(screen){
 const eligible=screen.filter(r=>r.success && Number.isFinite(r.peakPssKiB) && r.peakPssKiB<=4*1024**2 && Number.isFinite(r.totalMs));
 return eligible.sort((a,b)=>a.totalMs-b.totalMs || a.id.localeCompare(b.id))[0]??null;
}
