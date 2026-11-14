import {test} from 'node:test';import assert from 'node:assert/strict';import {createNavigation} from './navigation.js';
function fixture(){let current={file:'a',entity:null,filter:'',snapshot:'s',version:'v'},intent;const push=[],replace=[];
 const nav=createNavigation({capture:()=>current,load:async()=>false,history:{push:x=>push.push(x),replace:x=>replace.push(x),go(){}},
 prepare:async(query,next)=>{intent=next;return {view:{snapshot:'s',selection:{file:query.file},source:{id:query.file,version:'v'}},commit(){current={...current,file:query.file};nav.observe(next);return true;},cancel(){}};}});
 nav.observe();return {nav,push,replace,current:()=>current,intent:()=>intent};}
const anchor={snapshot:'s',file:'b',version:'v',line:3};
test('prepared editor navigation preserves the old visit and pushes a same-page message visit on commit',async()=>{const f=fixture();const planned=await f.nav.prepare({file:'b',entity:null,filter:''},anchor,{replaceHistory:false});assert.equal(f.current().file,'a');assert.equal(f.push.length,0);assert.equal(planned.commit(),true);assert.equal(f.current().file,'b');assert.equal(f.push.length,1);});
test('legacy editor fragment preparation replaces the entry already created by the URL',async()=>{const f=fixture();const planned=await f.nav.prepare({file:'b',entity:null,filter:''},anchor,{replaceHistory:true});planned.commit();assert.equal(f.push.length,0);assert.equal(f.replace.length,2);});
test('manual navigation invalidates a prepared editor visit before history or view changes',async()=>{const f=fixture();const planned=await f.nav.prepare({file:'b'},anchor,{});await f.nav.navigate({file:'c'});assert.equal(planned.commit(),false);assert.equal(f.current().file,'a');assert.equal(f.push.length,0);});
