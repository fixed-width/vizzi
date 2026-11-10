import {test} from 'node:test';import assert from 'node:assert/strict';
import {fetchRead} from './read-retry.js';
test('transient source-read admission recovers, while writes retain their first response',async()=>{
 let busy=true;const fetcher=async()=>{if(busy){busy=false;return new Response('busy',{status:503});}return new Response('captured source');};assert.equal(await (await fetchRead('/api/dependency/window?file=x',{},fetcher,async()=>{})).text(),'captured source');
 assert.equal((await fetchRead('/api/editor/open',{method:'POST'},async()=>new Response('busy',{status:503}),()=>{throw Error('write was retried');})).status,503);
});
test('permanent admission failure terminates and cancellation prevents another read',async()=>{
 let count=0;assert.equal((await fetchRead('/api/structure',{method:'POST'},async()=>{if(++count>20)throw Error('Read retries did not terminate');return new Response('busy',{status:429});},async()=>{})).status,429);
 const abort=new AbortController();await assert.rejects(fetchRead('/api/window',{signal:abort.signal},async()=>new Response('busy',{status:503}),async()=>abort.abort()),e=>e.name==='AbortError');
});
test('all source read interfaces recover while successful and nonbusy responses return directly',async()=>{
 for(const path of ['/api/window','/api/dependency/window','/api/structure','/api/visual/refresh']){let busy=true;const response=await fetchRead(path,{},async()=>{if(busy){busy=false;return new Response('busy',{status:503});}return new Response('source');},async()=>{});assert.equal(await response.text(),'source');}
 const response=await fetchRead('/api/window',{},async()=>new Response('expired',{status:409}),()=>{throw Error('Conflict must not retry');});assert.equal(response.status,409);
});
test('real backoff can be cancelled before or during its timer',async()=>{
 const controller=new AbortController();const request=fetchRead('/api/window',{signal:controller.signal},async()=>new Response('busy',{status:503}));controller.abort();await assert.rejects(request,e=>e.name==='AbortError');
 const second=new AbortController();const waiting=fetchRead('/api/window',{signal:second.signal},async()=>new Response('busy',{status:503}));setTimeout(()=>second.abort(),5);await assert.rejects(waiting,e=>e.name==='AbortError');
});
test('default backoff completes and releases transient response streams',async()=>{
 let cancelled=false,busy=true;const response=await fetchRead('/api/window',{},async()=>{if(busy){busy=false;return new Response(new ReadableStream({cancel(){cancelled=true;}}),{status:503});}return new Response('source');});assert.equal(await response.text(),'source');assert.equal(cancelled,true);
});
test('bodyless admission failures recover and cancellation preserves a custom reason promptly',async()=>{
 let busy=true;const response=await fetchRead('/api/window',{},async()=>{if(busy){busy=false;return new Response(null,{status:503});}return new Response('source');},async()=>{});assert.equal(await response.text(),'source');
 const controller=new AbortController(),reason=new Error('View closed');const read=fetchRead('/api/window',{signal:controller.signal},async()=>new Response('busy',{status:503}));controller.abort(reason);let timer;try{await assert.rejects(Promise.race([read,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Cancellation was delayed')),20);})]),e=>e===reason);}finally{clearTimeout(timer);}
});
