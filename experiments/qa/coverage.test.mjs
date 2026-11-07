import assert from 'node:assert/strict';
import {coveragePlan,locateTarget,assessCoverage} from './coverage-support.mjs';
const target={id:'check',alternatives:[{path:'src/check.rs',needle:'fn check(',scope:{name:'check',kind:'function',parent:'Gate'}}]};
const source={label:2,path:'src/check.rs',text:'if rejected { return Err(); }',scope:{name:'check',kind:'function',parents:['Gate']}};
const context={snapshot:'current',evidence:[{...source,label:1,path:'src/other.rs',text:'fn check() {}'},source]};
assert.deepEqual(locateTarget(context,target),{id:'check',literalRank:null,scopeRank:2,foundRank:2});
assert.deepEqual(locateTarget({snapshot:'current',evidence:[{...source,text:'fn check() {}'}]},target),{id:'check',literalRank:1,scopeRank:1,foundRank:1});
assert.deepEqual(locateTarget({evidence:[{...source,text:'fn check() {}',scope:null},source]},target),{id:'check',literalRank:1,scopeRank:2,foundRank:1});
assert.equal(locateTarget(context,{id:'alternative',alternatives:[{path:'other.rs',needle:'no'},...target.alternatives]}).foundRank,2);
assert.equal(locateTarget(context,{id:'optional-parent',alternatives:[{path:'src/check.rs',needle:'absent',scope:{name:'check',kind:'function'}}]}).foundRank,2);
for(const mutate of [e=>{e.path='other.rs';},e=>{e.scope.name='other';},e=>{e.scope.kind='macro';},e=>{e.scope.parents=[];},e=>{e.scope=null;}]){const e=structuredClone(source);mutate(e);assert.equal(locateTarget({snapshot:'current',evidence:[e]},target).foundRank,null);}
const literal={id:'branch',alternatives:[{path:'src/check.rs',needle:'return Err()'}]};
assert.equal(locateTarget({evidence:[source]},literal).literalRank,1);assert.equal(locateTarget({evidence:[]},literal).foundRank,null);
assert.equal(locateTarget({evidence:[{...source,text:'no marker'}]},literal).foundRank,null);
for(const bad of [{...target,id:''},{id:'x',alternatives:[]},{id:'x',alternatives:[{path:'x',needle:''}]},{id:'x',alternatives:[{path:'../x',needle:'x'}]},{id:'x',alternatives:[{path:'x',needle:'x',scope:{name:'',kind:'function'}}]}])assert.throws(()=>locateTarget(context,bad));
for(const path of ['', '/x', 'x\\y', 'x//y', 'x/./y'])assert.throws(()=>locateTarget(context,{id:'bad-path',alternatives:[{path,needle:'needle'}]}));
for(const scope of [{name:'check',kind:''},{name:'check',kind:1},{name:'check',kind:'function',parent:''},{name:'check',kind:'function',parent:1}])assert.throws(()=>locateTarget(context,{id:'bad-scope',alternatives:[{path:'x',needle:'needle',scope}]}));
const cases=[{id:'a',question:'where?',targets:[target]},{id:'b',question:'trace?',targets:[literal]}],plan=coveragePlan(cases);
assert.equal(plan.length,12);assert.equal(new Set(plan.map(p=>p.id)).size,12);assert.deepEqual([...new Set(plan.map(p=>p.selector))].sort(),['compact','search']);assert.deepEqual([...new Set(plan.map(p=>p.repeat))].sort(),[1,2,3]);
assert.notEqual(plan[0].selector,plan[4].selector);
assert.deepEqual(plan.map(p=>p.id),['a/search/repeat-1','a/compact/repeat-1','b/compact/repeat-1','b/search/repeat-1','b/compact/repeat-2','b/search/repeat-2','a/search/repeat-2','a/compact/repeat-2','b/search/repeat-3','b/compact/repeat-3','a/compact/repeat-3','a/search/repeat-3']);
const rows=plan.map(p=>({...p,context,elapsedMs:0,roundTripMs:1})),result=assessCoverage(rows,cases,'current');assert.equal(result.length,12);assert.equal(result[0].targets[0].foundRank,2);
for(const r of result)assert.equal(r.targets[0].id,r.case==='a'?'check':'branch');
for(const mutate of [r=>{r.pop();},r=>{r.push({...r[0],id:'extra'});},r=>{r[1].id=r[0].id;},r=>{r[0].case='wrong';},r=>{r[0].selector='wrong';},r=>{r[0].repeat=4;},r=>{r[0].question='wrong';},r=>{r[0].context.snapshot='stale';},r=>{r[0].elapsedMs=-1;},r=>{r[0].roundTripMs=NaN;}]){const bad=structuredClone(rows);mutate(bad);assert.throws(()=>assessCoverage(bad,cases,'current'));}
assert.throws(()=>coveragePlan([]));assert.throws(()=>coveragePlan([...cases,cases[0]]));
for(const bad of [{id:'',question:'q',targets:[target]},{id:'x',question:'',targets:[target]},{id:'x',question:null,targets:[target]},{id:'x',question:'q',targets:[]}])assert.throws(()=>coveragePlan([bad]));
for(const id of [1,['a']])assert.throws(()=>coveragePlan([{id,question:'q',targets:[target]}]));
assert.throws(()=>coveragePlan([{id:'x',question:['q'],targets:[target]}]));
for(const id of [1,['x']])assert.throws(()=>locateTarget(context,{...target,id}));
for(const needle of [null,' \n'])assert.throws(()=>locateTarget(context,{id:'x',alternatives:[{path:'x',needle}]}));
for(const scope of [{name:['check'],kind:'function'},{name:'check',kind:['function']},{name:'check',kind:'function',parent:['Gate']}])assert.throws(()=>locateTarget(context,{id:'x',alternatives:[{path:'x',needle:'needle',scope}]}));
for(const key of ['elapsedMs','roundTripMs'])for(const value of [null,Infinity,-1]){const bad=structuredClone(rows);bad[0][key]=value;assert.throws(()=>assessCoverage(bad,cases,'current'));}
assert.equal(assessCoverage(rows.map(r=>({...r,roundTripMs:0})),cases,'current').length,12);
console.log('coverage: literal/scope distinctions, exact path/parent binding, complete alternating ledgers and timing validity');
