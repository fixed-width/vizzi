import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createController} from './controller.js';
const view=file=>({schema:2,snapshot:'capture',selection:{file,entity:null}});
test('prepared views do not apply until committed and cannot apply twice',async()=>{
 const shown=[];const controller=createController({transport:async()=>view('b'),onView:next=>shown.push(next)});
 const prepared=await controller.prepare({file:'b'});assert.deepEqual(shown,[]);assert.equal(prepared.view.selection.file,'b');assert.equal(prepared.commit(),true);assert.equal(prepared.commit(),false);assert.deepEqual(shown,[view('b')]);
});
test('a newer view load invalidates a prepared editor view without changing the current display',async()=>{
 const shown=[];const controller=createController({transport:async uri=>view(new URL(uri,'http://localhost').searchParams.get('file')),onView:next=>shown.push(next.selection.file)});
 const prepared=await controller.prepare({file:'b'});assert.equal(await controller.load({file:'c'}),true);assert.equal(prepared.commit(),false);assert.deepEqual(shown,['c']);
});
test('a cancelled prepared view is discarded without a browser error',async()=>{
 const shown=[],errors=[];const controller=createController({transport:async()=>view('b'),onView:next=>shown.push(next),onError:error=>errors.push(error)});
 const prepared=await controller.prepare({file:'b'});prepared.cancel();assert.equal(prepared.commit(),false);assert.deepEqual(shown,[]);assert.deepEqual(errors,[]);
});

test('discarding a prepared editor view still follows a source publication that arrived while it waited',async()=>{
 const shown=[];const controller=createController({transport:async uri=>{const p=new URL(uri,'http://localhost').searchParams;return {schema:2,snapshot:p.get('snapshot'),selection:{file:p.get('file')||'a',entity:null}};},onView:next=>shown.push([next.snapshot,next.selection.file])});
 const stream=controller.updates();const change=(revision,snapshot)=>({schema:1,stream:'stream',revision:String(revision),status:{schema:1,name:'fixture',observation:'saved',generation:revision,pending:false,snapshot,error:null,measurements:null}});
 stream.receive(change(1,'old'));await new Promise(setImmediate);const planned=await controller.prepare({file:'b'},{rollbackOnFailure:true});
 stream.receive(change(2,'new'));assert.equal(planned.commit(),false);planned.cancel();await new Promise(setImmediate);assert.deepEqual(shown,[['old','a'],['new','a']]);
});
