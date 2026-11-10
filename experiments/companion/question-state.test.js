import assert from 'node:assert/strict';
import {SearchSession,groupResults}from'./question-state.js';
const token='a'.repeat(64),other='b'.repeat(64),hit={id:other,title:'new',path:'src/a.rs',file:token,version:other,line:1,end_line:2,excerpt:'fn new() {}',suggested_role:'definition',scope:null};
const payload={snapshot:token,question:'where?',selector:'focused',available_files:2,searched_files:1,omitted_files:1,results:[hit]};
const session=new SearchSession();assert.throws(()=>session.begin('where?','focused'));session.setSnapshot(token);
for(const q of ['', ' \n','é'.repeat(1025)])assert.throws(()=>session.begin(q,'focused'));assert.throws(()=>session.begin('q','unknown'));
const first=session.begin('where?','focused'),second=session.begin('where?','focused');assert.equal(session.complete(first.ticket,payload),false);assert.equal(session.complete(second.ticket,payload),true);assert.equal(session.canKeep,true);assert.equal(session.pending,false);
assert.deepEqual(groupResults(payload.results).map(g=>g.label),['Definition candidates']);
session.setSnapshot(other);assert.equal(session.stale,true);assert.equal(session.canKeep,false);const changed=session.begin('where?','original');session.setSnapshot(token);assert.equal(session.complete(changed.ticket,{...payload,snapshot:other,selector:'original'}),false);
const base=new SearchSession();base.setSnapshot(token);
for(const mutate of [p=>{p.snapshot=other;},p=>{p.question='other';},p=>{p.selector='original';},p=>{p.results=Array(9).fill(hit);},p=>{p.results[0].line=0;},p=>{p.results[0].end_line=0;},p=>{p.results[0].file='bad';},p=>{p.results[0].path='../x';},p=>{p.results[0].excerpt='x'.repeat(4001);},p=>{p.results[0].suggested_role='guaranteed';},p=>{p.omitted_files=0;}]){const start=base.begin('where?','focused'),bad=structuredClone(payload);mutate(bad);assert.throws(()=>base.complete(start.ticket,bad));base.fail(start.ticket);}
const empty=base.begin('where?','focused');assert.equal(base.complete(empty.ticket,{...payload,results:[]}),true);assert.equal(base.data.results.length,0);assert.equal(base.stale,false);
const pending=base.begin('where?','focused');base.fail(pending.ticket-1);assert.equal(base.pending,true);base.fail(pending.ticket);assert.equal(base.pending,false);
assert.deepEqual(groupResults([{...hit,suggested_role:'validation'},{...hit,suggested_role:null},{...hit,suggested_role:'entry'}]).map(g=>g.label),['Entry and use candidates','Check candidates','Matching source']);
console.log('question state: bounded native response validation, supersession, stale capture, empty results and suggested role groups');
const failedNewQuery=new SearchSession();failedNewQuery.setSnapshot(token);
const originalQuery=failedNewQuery.begin('where?','focused');failedNewQuery.complete(originalQuery.ticket,payload);
const differentQuery=failedNewQuery.begin('another question','focused');failedNewQuery.fail(differentQuery.ticket);
assert.equal(failedNewQuery.canKeep,false,'old cards must not be saved as results of a failed new query');

// Invalid transport shapes must never enable a source save.
const malformed = [
 p=>{p.available_files=-1;},p=>{p.available_files=1.5;},p=>{p.searched_files=-1;},p=>{p.searched_files=NaN;},p=>{p.omitted_files=-1;},
 p=>{p.results=null;},p=>{p.results[0]=null;},p=>{p.results.push({...p.results[0]});},p=>{p.results[0].id='invalid';},p=>{p.results[0].version='invalid';},
 p=>{p.results[0].title=null;},p=>{p.results[0].title='é'.repeat(65);},p=>{p.results[0].path=null;},p=>{p.results[0].path='x'.repeat(4097);},p=>{p.results[0].path='a\\b';},p=>{p.results[0].path='/absolute';},p=>{p.results[0].path='a//b';},p=>{p.results[0].path='./b';},
 p=>{p.results[0].line=0.5;},p=>{p.results[0].line=Number.MAX_SAFE_INTEGER+1;},p=>{p.results[0].end_line=-1;},p=>{p.results[0].end_line=0.5;},p=>{p.results[0].excerpt=null;},
];
for (const mutate of malformed) {const s=new SearchSession();s.setSnapshot(token);const r=s.begin('where?','focused'),bad=structuredClone(payload);mutate(bad);assert.throws(()=>s.complete(r.ticket,bad));assert.equal(s.canKeep,false);s.fail(r.ticket);assert.equal(s.pending,false);}
const limits=new SearchSession();limits.setSnapshot(token);limits.setSnapshot(token);
for(const invalid of [undefined,null,1,{},'\t'])assert.throws(()=>limits.begin(invalid,'focused'));
const max=limits.begin('é'.repeat(1024),'original');const maxHit={...hit,title:'é'.repeat(64),path:'x'.repeat(4096),end_line:1,excerpt:'é'.repeat(2000),suggested_role:null};
assert.equal(limits.complete(max.ticket,{snapshot:token,question:max.body.question,selector:'original',available_files:1,searched_files:1,omitted_files:0,results:[maxHit]}),true);
assert.equal(limits.canKeep,true);limits.setSnapshot(null);assert.equal(limits.canKeep,false);assert.equal(limits.stale,true);assert.throws(()=>limits.begin('q','focused'));
assert.deepEqual(groupResults(['definition','registration','entry','validation','example',null].reverse().map((role,i)=>({...hit,id:String(i),suggested_role:role}))).map(g=>g.role),['definition','registration','entry','validation','example',null]);
const eight=new SearchSession();eight.setSnapshot(token);const r8=eight.begin('where?','focused');assert.equal(eight.complete(r8.ticket,{...payload,results:Array.from({length:8},(_,i)=>({...hit,id:i.toString(16).repeat(64),excerpt:''}))}),true);
const changedSelector=failedNewQuery.begin('where?','original');failedNewQuery.fail(changedSelector.ticket);assert.equal(failedNewQuery.canKeep,false);
for(const invalid of ['A'.repeat(64),'g'.repeat(64),'a'.repeat(63),'a'.repeat(65),'!'+token,token+'!',42,null]){
 const s=new SearchSession();s.setSnapshot(invalid);assert.throws(()=>s.begin('where?','focused'));
 for(const field of ['id','file','version']){const b=new SearchSession();b.setSnapshot(token);const r=b.begin('where?','focused'),bad=structuredClone(payload);bad.results[0][field]=invalid;assert.throws(()=>b.complete(r.ticket,bad));}
}
assert.deepEqual(groupResults(['definition','registration','entry','validation','example',null].map(role=>({...hit,suggested_role:role}))).map(g=>g.label),['Definition candidates','Registration candidates','Entry and use candidates','Check candidates','Example candidates','Matching source']);
for(const invalid of [null,undefined,{},[]]){const s=new SearchSession();s.setSnapshot(token);const r=s.begin('where?','focused');assert.throws(()=>s.complete(r.ticket,invalid),/Invalid source-search response/);}

const sameCapture=new SearchSession();sameCapture.setSnapshot(token);const active=sameCapture.begin('where?','focused');sameCapture.setSnapshot(token);assert.equal(sameCapture.complete(active.ticket,payload),true);

const initial=new SearchSession();assert.equal(initial.pending,false);assert.equal(initial.stale,false);assert.equal(initial.canKeep,false);initial.setSnapshot(token);assert.equal(initial.stale,false);
for(const counts of [{available_files:-1,searched_files:-1,omitted_files:0},{available_files:1.5,searched_files:0.5,omitted_files:1},{available_files:'00',searched_files:'0',omitted_files:0}]){const s=new SearchSession();s.setSnapshot(token);const r=s.begin('where?','focused');assert.throws(()=>s.complete(r.ticket,{...payload,...counts,results:[]}),/response/);}
for(const field of ['id','file','version']){const s=new SearchSession();s.setSnapshot(token);const r=s.begin('where?','focused'),bad=structuredClone(payload);bad.results[0][field]=[token];assert.throws(()=>s.complete(r.ticket,bad),/location/);}
const arraySnapshot=new SearchSession();arraySnapshot.setSnapshot([token]);assert.throws(()=>arraySnapshot.begin('q','focused'),/indexing/);
const nine=new SearchSession();nine.setSnapshot(token);const nr=nine.begin('where?','focused');assert.throws(()=>nine.complete(nr.ticket,{...payload,results:Array.from({length:9},(_,i)=>({...hit,id:i.toString(16).repeat(64),excerpt:''}))}),/response/);
for(const path of [null,[],{}]){const s=new SearchSession();s.setSnapshot(token);const r=s.begin('where?','focused');assert.throws(()=>s.complete(r.ticket,{...payload,results:[{...hit,path}]}),/location/);}
const messages=new SearchSession();messages.setSnapshot(token);
for(const q of [null,1,{},''])assert.throws(()=>messages.begin(q,'focused'),/question/);
assert.throws(()=>messages.begin('q','wrong'),/comparison/);
messages.setSnapshot(null);assert.throws(()=>messages.begin('q','focused'),/indexing/);
messages.setSnapshot(token);const mr=messages.begin('where?','focused');assert.throws(()=>messages.complete(mr.ticket,{...payload,results:[{...hit,excerpt:'x'.repeat(4001)}]}),/allowance/);
const generations=new SearchSession();generations.setSnapshot(token);const ga=generations.begin('where?','focused');generations.setSnapshot(other);assert.equal(generations.pending,false);const gb=generations.begin('new question','focused');assert.equal(generations.complete(ga.ticket,payload),false);assert.equal(generations.pending,true);assert.equal(generations.complete(gb.ticket,{...payload,snapshot:other,question:'new question'}),true);
const repeat=new SearchSession();repeat.setSnapshot(token);const ra=repeat.begin('where?','focused');repeat.complete(ra.ticket,payload);const rb=repeat.begin('where?','focused');assert.equal(repeat.canKeep,false);repeat.complete(rb.ticket,payload);repeat.setSnapshot(other);assert.equal(repeat.stale,true);repeat.setSnapshot(token);assert.equal(repeat.stale,false);assert.equal(repeat.canKeep,true);
