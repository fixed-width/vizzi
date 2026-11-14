import {test} from 'node:test';import assert from 'node:assert/strict';
import {createReviewPreparation} from './review-delivery.js';
const turn=()=>new Promise(setImmediate);
function fixture(serverTicket='requested') {
 let status={ticket:'previous'},queries=0;const watchers=new Set();
 const prepare=createReviewPreparation({current:()=>status,subscribe:watch=>{watchers.add(watch);return()=>watchers.delete(watch);},status:async()=>{queries++;return {ticket:serverTicket};}});
 return {prepare,queries:()=>queries,watchers,setServer(ticket){serverTicket=ticket;},set(ticket){status={ticket};for(const watcher of [...watchers])watcher();}};
}
test('an accepted review waits for its ticket to reach the independent browser status stream',async()=>{
 const f=fixture();let settled=false;const result=f.prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal}).then(value=>{settled=true;return value;},error=>{settled=true;return error;});
 await turn();const early=settled;f.set('requested');const prepared=await result;
 assert.equal(early,false);assert.equal(f.queries(),1);assert.equal(typeof prepared.commit,'function');assert.notEqual(prepared.commit(),false);assert.equal(f.watchers.size,0);
});
test('a fresh server status can establish that the requested review was actually replaced',async()=>{
 const f=fixture('different');await assert.rejects(f.prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal}),/replaced/);assert.equal(f.queries(),1);assert.equal(f.watchers.size,0);
});
test('manual cancellation while review status is catching up releases its subscription',async()=>{
 const f=fixture(),controller=new AbortController();const result=f.prepare({kind:'review',ticket:'requested'},{signal:controller.signal});const rejected=assert.rejects(result,error=>error.name==='AbortError');await turn();controller.abort();await rejected;f.set('requested');assert.equal(f.watchers.size,0);
});
test('a replacement observed after review preparation cannot commit the old ticket',async()=>{
 const f=fixture();f.set('requested');const prepared=await f.prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal});f.set('different');assert.equal(prepared.commit(),false);
});

test('stale status notifications cannot turn an accepted review into a false replacement',async()=>{
 const f=fixture();let settled=false;const result=f.prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal}).then(value=>{settled=true;return value;});await turn();f.set('previous');await turn();const early=settled;f.set('requested');const prepared=await result;assert.equal(early,false);assert.notEqual(prepared.commit(),false);assert.equal(f.watchers.size,0);
});
test('aborted review delivery does not query status or leave a watcher',async()=>{
 const f=fixture(),controller=new AbortController();controller.abort();await assert.rejects(f.prepare({kind:'review',ticket:'requested'},{signal:controller.signal}),e=>e.name==='AbortError');assert.equal(f.queries(),0);assert.equal(f.watchers.size,0);
});
test('reveal and an already represented review do not need an additional status request',async()=>{
 const f=fixture();const reveal=await f.prepare({kind:'reveal'},{signal:new AbortController().signal});assert.notEqual(reveal.commit(),false);f.set('requested');const review=await f.prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal});assert.notEqual(review.commit(),false);assert.equal(f.queries(),0);assert.equal(f.watchers.size,0);
});

test('a real replacement while the status stream catches up refuses instead of waiting forever',async()=>{
 const f=fixture();const result=f.prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal});const rejected=assert.rejects(result,/replaced/);await turn();f.setServer('replacement');f.set('replacement');await rejected;assert.equal(f.watchers.size,0);
});

test('a comparison replaced while a status request is in flight is checked again after that request',async()=>{
 let resolveStatus,local={ticket:'previous'},server='requested',queries=0;const watchers=new Set();
 const prepare=createReviewPreparation({current:()=>local,subscribe:watch=>{watchers.add(watch);return()=>watchers.delete(watch);},status:()=>{queries++;return queries===1?new Promise(resolve=>{resolveStatus=resolve;}):Promise.resolve({ticket:server});}});
 const result=prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal});const rejected=assert.rejects(result,/replaced/);
 await turn();server='replacement';local={ticket:server};for(const watch of [...watchers])watch();assert.equal(queries,1);resolveStatus({ticket:'requested'});await rejected;assert.equal(watchers.size,0);
});
test('review status transport failure is returned and releases the status subscription',async()=>{
 const watchers=new Set();const prepare=createReviewPreparation({current:()=>({ticket:'previous'}),subscribe:watch=>{watchers.add(watch);return()=>watchers.delete(watch);},status:async()=>{throw new Error('status unavailable');}});
 await assert.rejects(prepare({kind:'review',ticket:'requested'},{signal:new AbortController().signal}),/status unavailable/);assert.equal(watchers.size,0);
});
test('a represented ticket can prepare while an older server status request is still pending',async()=>{
 let resolveStatus,local={ticket:'previous'};const watchers=new Set();const controller=new AbortController();
 const prepare=createReviewPreparation({current:()=>local,subscribe:watch=>{watchers.add(watch);return()=>watchers.delete(watch);},status:()=>new Promise(resolve=>{resolveStatus=resolve;})});
 const result=prepare({kind:'review',ticket:'requested'},{signal:controller.signal});await turn();local={ticket:'requested'};for(const watch of [...watchers])watch();const prepared=await result;assert.notEqual(prepared.commit(),false);assert.equal(watchers.size,0);resolveStatus({ticket:'obsolete'});await turn();controller.abort();assert.throws(()=>prepared.commit(),error=>error.name==='AbortError');
});

test('a review without a capture ticket preserves the current comparison without querying status',async()=>{
 let queries=0;const prepare=createReviewPreparation({current:()=>undefined,subscribe:()=>{throw new Error('unexpected subscription');},status:async()=>{queries++;}});
 const prepared=await prepare({kind:'review'});assert.notEqual(prepared.commit(),false);assert.equal(queries,0);
});
test('an already represented review supports an omitted cancellation signal',async()=>{
 const f=fixture();f.set('requested');const prepared=await f.prepare({kind:'review',ticket:'requested'});assert.notEqual(prepared.commit(),false);assert.equal(f.watchers.size,0);
});
