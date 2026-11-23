import {test} from 'node:test';import assert from 'node:assert/strict';
import {getEventListeners} from 'node:events';
import {fetchRead} from './read-retry.js';
test('historical window reads recover from busy admission without retrying history navigation',async()=>{
 for(const status of [429,503]){
  let attempts=0;const delays=[];
  const response=await fetchRead('/api/function-history/window?token=captured&line_start=97',{},async()=>new Response(++attempts===1?'busy':'historical source',{status:attempts===1?status:200}),async delay=>delays.push(delay));
  assert.equal(await response.text(),'historical source');assert.equal(attempts,2);assert.deepEqual(delays,[50]);
 }
 for(const path of ['/api/function-history','/api/function-history/select','/api/function-history/step','/api/function-comparisons'])assert.equal((await fetchRead(path,{method:'POST'},async()=>new Response('busy',{status:503}),()=>{throw Error('History navigation must not retry');})).status,503);
});
test('historical window retries remain bounded, respect cancellation and retain expiry conflicts',async()=>{
 const path='/api/function-history/window?token=captured';let attempts=0;const delays=[];
 assert.equal((await fetchRead(path,{},async()=>{attempts++;return new Response('busy',{status:503});},async delay=>delays.push(delay))).status,503);
 assert.equal(attempts,6);assert.deepEqual(delays,[50,100,200,400,800]);
 const controller=new AbortController();attempts=0;
 await assert.rejects(fetchRead(path,{signal:controller.signal},async()=>{attempts++;return new Response('busy',{status:503});},async()=>controller.abort()),error=>error.name==='AbortError');assert.equal(attempts,1);
 assert.equal((await fetchRead(path,{},async()=>new Response('expired',{status:409}),()=>{throw Error('Expired history must not retry');})).status,409);
});
test('successful historical backoff releases its abort listener',async()=>{
 const controller=new AbortController();let busy=true;
 await fetchRead('/api/function-history/window',{signal:controller.signal},async()=>{if(busy){busy=false;return new Response('busy',{status:503});}return new Response('source');});
 assert.equal(getEventListeners(controller.signal,'abort').length,0);
});
test('cancelling historical backoff releases its timer and listener, including a null reason',async()=>{
 const timers=()=>process.getActiveResourcesInfo().filter(resource=>resource==='Timeout').length;
 for(const reason of [new Error('View closed'),null]){
  const controller=new AbortController(),before=timers();let busy=true;
  const read=fetchRead('/api/function-history/window',{signal:controller.signal},async()=>{if(busy){busy=false;return new Response('busy',{status:503});}return new Response('source');});read.catch(()=>{});
  try{
   for(let turn=0;turn<20&&getEventListeners(controller.signal,'abort').length===0;turn++)await Promise.resolve();
   assert.equal(getEventListeners(controller.signal,'abort').length,1);
   controller.abort(reason);await assert.rejects(read,error=>reason===null?error?.name==='AbortError'&&error.message==='Aborted':error===reason);
   assert.equal(getEventListeners(controller.signal,'abort').length,0);assert.equal(timers(),before);
  }finally{controller.abort();await read.catch(()=>{});}
 }
});
test('transient source-read admission recovers, while writes retain their first response',async()=>{
 let busy=true;const fetcher=async()=>{if(busy){busy=false;return new Response('busy',{status:503});}return new Response('captured source');};assert.equal(await (await fetchRead('/api/dependency/window?file=x',{},fetcher,async()=>{})).text(),'captured source');
 assert.equal((await fetchRead('/api/editor/open',{method:'POST'},async()=>new Response('busy',{status:503}),()=>{throw Error('write was retried');})).status,503);
});
test('permanent admission failure terminates and cancellation prevents another read',async()=>{
 let count=0;assert.equal((await fetchRead('/api/structure',{method:'POST'},async()=>{if(++count>20)throw Error('Read retries did not terminate');return new Response('busy',{status:429});},async()=>{})).status,429);
 const abort=new AbortController();await assert.rejects(fetchRead('/api/window',{signal:abort.signal},async()=>new Response('busy',{status:503}),async()=>abort.abort()),e=>e.name==='AbortError');
});
test('all source read interfaces recover while successful and nonbusy responses return directly',async()=>{
 for(const path of ['/api/window','/api/dependency/window','/api/function-history/window','/api/function-history/confirm','/api/function-comparisons/window','/api/captured-functions','/api/structure','/api/visual/refresh','/api/references']){let busy=true;const response=await fetchRead(path,{},async()=>{if(busy){busy=false;return new Response('busy',{status:503});}return new Response('source');},async()=>{});assert.equal(await response.text(),'source');}
 for(const path of ['/api/window','/api/captured-functions']){
  const response=await fetchRead(path,{},async()=>new Response('conflict',{status:409}),()=>{throw Error('Conflict must not retry');});
  assert.equal(response.status,409);
 }
});
test('real backoff can be cancelled before or during its timer',async()=>{
 const controller=new AbortController();const request=fetchRead('/api/window',{signal:controller.signal},async()=>new Response('busy',{status:503}));controller.abort();await assert.rejects(request,e=>e.name==='AbortError');
 const second=new AbortController();const waiting=fetchRead('/api/window',{signal:second.signal},async()=>new Response('busy',{status:503}));setTimeout(()=>second.abort(),5);await assert.rejects(waiting,e=>e.name==='AbortError');
});
test('default backoff completes and releases transient response streams',async()=>{
 let cancelled=false,busy=true;const response=await fetchRead('/api/window',{},async()=>{if(busy){busy=false;return new Response(new ReadableStream({start(controller){controller.enqueue(new TextEncoder().encode('busy'));controller.close();},cancel(){cancelled=true;}}),{status:503});}return new Response('source');});assert.equal(await response.text(),'source');assert.equal(cancelled,true);
});
test('bodyless admission failures recover and cancellation preserves a custom reason promptly',async()=>{
 let busy=true;const response=await fetchRead('/api/window',{},async()=>{if(busy){busy=false;return new Response(null,{status:503});}return new Response('source');},async()=>{});assert.equal(await response.text(),'source');
 const controller=new AbortController(),reason=new Error('View closed');const read=fetchRead('/api/window',{signal:controller.signal},async()=>new Response('busy',{status:503}));controller.abort(reason);let timer;try{await assert.rejects(Promise.race([read,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Cancellation was delayed')),20);})]),e=>e===reason);}finally{clearTimeout(timer);}
});
