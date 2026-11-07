import assert from 'node:assert/strict';

export function coveragePlan(cases){
 assert.ok(cases.length>0);assert.equal(new Set(cases.map(c=>c.id)).size,cases.length);
 for(const c of cases){assert.ok(typeof c.id==='string'&&c.id.length>0&&typeof c.question==='string'&&c.question.length>0&&c.targets.length>0);}
 return [1,2,3].flatMap(repeat=>{
  const order=repeat===2?[...cases].reverse():repeat===3?[...cases.slice(1),cases[0]]:cases;
  return order.flatMap((c,index)=>((index+repeat)%2?['search','compact']:['compact','search']).map(selector=>({id:`${c.id}/${selector}/repeat-${repeat}`,case:c.id,question:c.question,selector,repeat})));
 });
}
export function locateTarget(context,target){
 assert.ok(typeof target.id==='string'&&target.id.length>0&&target.alternatives.length>0);
 for(const a of target.alternatives){
  assert.ok(typeof a.path==='string'&&!a.path.startsWith('/')&&!a.path.includes('\\')&&a.path.split('/').every(p=>p&&p!=='.'&&p!=='..'));
  assert.ok(typeof a.needle==='string'&&a.needle.trim().length>0);
  if(a.scope){assert.ok(typeof a.scope.name==='string'&&a.scope.name.length>0&&typeof a.scope.kind==='string'&&a.scope.kind.length>0);if(a.scope.parent!==undefined)assert.ok(typeof a.scope.parent==='string'&&a.scope.parent.length>0);}
 }
 const rank=predicate=>{const index=context.evidence.findIndex(e=>target.alternatives.some(a=>e.path===a.path&&predicate(e,a)));return index<0?null:index+1;};
 const literalRank=rank((e,a)=>e.text.includes(a.needle)),scopeRank=rank((e,a)=>a.scope&&e.scope&&e.scope.name===a.scope.name&&e.scope.kind===a.scope.kind&&(a.scope.parent===undefined||e.scope.parents.includes(a.scope.parent)));
 const found=[literalRank,scopeRank].filter(n=>n!==null);
 return {id:target.id,literalRank,scopeRank,foundRank:found.length?Math.min(...found):null};
}
export function assessCoverage(rows,cases,snapshot){
 const plan=coveragePlan(cases);assert.equal(rows.length,plan.length);assert.equal(new Set(rows.map(r=>r.id)).size,rows.length);
 return plan.map(p=>{
  const row=rows.find(r=>r.id===p.id);assert.ok(row);
  for(const key of ['case','selector','repeat','question'])assert.equal(row[key],p[key]);
  assert.equal(row.context.snapshot,snapshot);assert.ok(Number.isFinite(row.elapsedMs)&&row.elapsedMs>=0&&Number.isFinite(row.roundTripMs)&&row.roundTripMs>=0);
  return {...p,targets:cases.find(c=>c.id===p.case).targets.map(t=>locateTarget(row.context,t)),elapsedMs:row.elapsedMs,roundTripMs:row.roundTripMs};
 });
}
