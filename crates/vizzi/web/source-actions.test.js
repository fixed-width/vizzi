import assert from 'node:assert/strict';
import {test} from 'node:test';
import {requestSourceActions} from './call-actions.js';

test('oversized action results retain a bounded prefix of valid pages',async()=>{
    const offsets=[];
    const actions=await requestSourceActions(async (_path,{snapshot,query:{offset}})=>{
        offsets.push(offset);
        return {snapshot,matching:4097,next:offset+256,items:Array.from({length:256},(_,i)=>({id:offset+i}))};
    },'snapshot',{kind:'source_actions'});
    assert.deepEqual(offsets,Array.from({length:16},(_,i)=>i*256));
    assert.equal(actions.length,4096);
    assert.equal(actions.at(-1).id,4095);
});
