import { test } from "node:test";
import assert from "node:assert/strict";
import { createWindowReader } from "./windows.js";

const view = (snapshot = "one", version = "v1") => ({ snapshot, source: { id: "file", version, summary: { lines: 2, declarations: 2 } } });
function windowFor(v) {
  const row = { index:0, analysis:0, id:"e0", name:"first", name_truncated:false, kind_truncated:false, kind:"function", start_line:1, end_line:1 };
  return { schema:1, snapshot:v.snapshot,file:v.source.id,version:v.source.version,line_total:2,line_start:1,lines:[{number:1,column:0,end_column:3,line_bytes:3,text:"abc",left:false,right:false}],outline_total:2,outline_start:0,outline:[row],cursor:{...row} };
}
test("window requests bind all identities and late replies cannot replace newer pages or documents", async () => {
  const pending=[],applied=[],errors=[];
  const reader=createWindowReader({transport:(uri,options)=>new Promise(resolve=>pending.push({uri,options,resolve})),onWindow:w=>applied.push(w),onError:e=>errors.push(e)});
  const first=view();reader.activate(first);const a=reader.request({line_start:1});const b=reader.request({line_start:2});
  assert.ok(pending[0].options.signal.aborted);
  const url=new URL(pending[1].uri,"http://local");assert.equal(url.searchParams.get("snapshot"),"one");assert.equal(url.searchParams.get("version"),"v1");assert.equal(url.searchParams.get("file"),"file");
  pending[1].resolve(windowFor(first));assert.equal(await b,true);pending[0].resolve(windowFor(first));assert.equal(await a,false);assert.equal(applied.length,1);
  const c=reader.request({});reader.activate(view("two","v2"));pending[2].resolve(windowFor(first));assert.equal(await c,false);assert.equal(applied.length,1);assert.deepEqual(errors,[]);
});
test("malformed bindings, text budgets, row bounds, and ordinals fail before application", async () => {
  const v=view();const errors=[],applied=[];let response;
  const reader=createWindowReader({transport:async()=>response,onError:e=>errors.push(e),onWindow:w=>applied.push(w)});reader.activate(v);
  const changes=[w=>w.schema=2,w=>w.snapshot="other",w=>w.file="other",w=>w.version="other",w=>w.line_total=3,w=>w.outline_total=3,w=>w.lines=Array(97).fill(w.lines[0]),w=>w.outline=Array(201).fill(w.outline[0]),w=>w.lines[0].text="x".repeat(32769),w=>w.lines[0].end_column=-1,w=>w.outline[0].index=2,w=>w.outline[0].name="x".repeat(129),w=>w.outline[0].kind="x".repeat(33),w=>w.cursor={...w.cursor,id:""}];
  for(const change of changes){response=structuredClone(windowFor(v));change(response);assert.equal(await reader.request({}),false);}
  assert.equal(errors.length,changes.length);assert.deepEqual(applied,[]);assert.ok(errors.every(error=>error.message.length>5));
});
test("preparation is side-effect free; current errors report once and absent source does not fetch", async () => {
  const applied=[],errors=[],calls=[];const v=view();
  const reader=createWindowReader({transport:async(uri)=>{calls.push(uri);return windowFor(v);},onWindow:w=>applied.push(w),onError:e=>errors.push(e)});
  assert.equal((await reader.prepare(v,{})).version,"v1");assert.equal(applied.length,0);assert.equal(await reader.prepare({snapshot:"one",source:null},{}),null);assert.equal(calls.length,1);
  reader.activate(v);assert.equal(await reader.request({}),true);assert.equal(applied.length,1);
  const errorReader=createWindowReader({transport:async()=>{throw new Error("offline");},onError:e=>errors.push(e)});errorReader.activate(v);assert.equal(await errorReader.request({}),false);assert.equal(errors.length,1);
});

test("exact byte, entry, opaque-identity limits and empty text are accepted",async()=>{
  const v=view("s".repeat(256),"v".repeat(256));v.source.id="f".repeat(256);v.source.summary={lines:96,declarations:200};let data=windowFor(v);
  data.line_total=96;data.outline_total=200;data.lines=[{number:1,column:0,end_column:32768,line_bytes:32768,text:"x".repeat(32768),left:false,right:false}];
  data.outline=Array.from({length:200},(_,index)=>({...data.outline[0],index,id:index===0?"e".repeat(256):`e${index}`,name:"n".repeat(128),kind:"k".repeat(32),end_line:96}));data.cursor=data.outline[199];
  const reader=createWindowReader({transport:async()=>data});assert.equal((await reader.prepare(v,{})).outline.length,200);
  data.lines=Array.from({length:96},(_,i)=>({number:i+1,column:0,end_column:0,line_bytes:0,text:"",left:false,right:false}));assert.equal((await reader.prepare(v,{})).lines.length,96);
  const empty=view();empty.source.summary={lines:0,declarations:0};data={schema:1,snapshot:empty.snapshot,file:empty.source.id,version:empty.source.version,line_total:0,line_start:1,lines:[],outline_total:0,outline_start:0,outline:[],cursor:null};assert.equal((await reader.prepare(empty,{})).cursor,null);
});
test("UTF-8 columns and both continuation flags are checked independently",async()=>{
  const v=view();let data=windowFor(v);data.lines[0]={number:1,column:2,end_column:4,line_bytes:8,text:"β",left:true,right:true};
  const reader=createWindowReader({transport:async()=>data});assert.equal((await reader.prepare(v,{})).lines[0].text,"β");
  for(const change of [w=>w.lines[0].left=false,w=>w.lines[0].right=false,w=>w.lines[0].column=3,w=>w.lines[0].end_column=9,w=>w.lines[0].number=2,w=>w.outline[0].start_line=0,w=>w.outline[0].end_line=3,w=>w.outline[0].analysis=-1,w=>delete w.outline[0].kind_truncated,w=>w.outline[0].index=1]){
    data=windowFor(v);data.lines[0]={number:1,column:2,end_column:4,line_bytes:8,text:"β",left:true,right:true};change(data);await assert.rejects(reader.prepare(v,{}));
  }
});

test("independently invalid metadata, row types and aggregate budgets cannot be masked by callback errors",async()=>{
  let v=view(),data=windowFor(v),lastUri;
  const reader=createWindowReader({transport:async uri=>{lastUri=uri;return data;}});
  for(const change of [v=>v.snapshot="",v=>v.snapshot=7,v=>v.source.id="x".repeat(257),v=>v.source.id={length:1},v=>v.source.version="",v=>v.source.summary.lines=-1,v=>v.source.summary.declarations=1.5]){
    v=view();change(v);data=windowFor(v);await assert.rejects(reader.prepare(v,{}),/summary/);
  }
  v=view();data=windowFor(v);await reader.prepare(v,{line_start:undefined,column_line:null,column:0});
  const query=new URL(lastUri,"http://local").searchParams;assert.equal(query.has("line_start"),false);assert.equal(query.has("column_line"),false);assert.equal(query.get("column"),"0");
  for(const change of [w=>w.line_start=0,w=>w.line_start=3,w=>w.outline_start=-1,w=>w.outline_start=2,w=>w.lines=null,w=>w.outline=null,w=>w.lines[0].text=123,w=>w.lines[0].column=-1,w=>w.lines[0].end_column=2,w=>w.lines[0].line_bytes=2,w=>w.outline[0].name=null,w=>w.outline[0].kind=123,w=>delete w.outline[0].name_truncated,w=>w.outline[0].end_line=0,w=>w.cursor={...w.cursor,index:2},w=>w.cursor={...w.cursor,name:"x".repeat(129)}]){
    data=windowFor(v);change(data);await assert.rejects(reader.prepare(v,{}));
  }
  data=windowFor(v);data.lines=Array.from({length:2},(_,i)=>({number:i+1,column:0,end_column:20000,line_bytes:20000,text:"x".repeat(20000),left:false,right:false}));await assert.rejects(reader.prepare(v,{}),/budget/);
});
test("late failures are silent and completed signals are released rather than aborted later",async()=>{
  const pending=[],errors=[];const reader=createWindowReader({transport:(uri,options)=>new Promise((resolve,reject)=>pending.push({resolve,reject,options})),onError:e=>errors.push(e)});const v=view();reader.activate(v);
  const a=reader.request({}),b=reader.request({});pending[1].resolve(windowFor(v));assert.equal(await b,true);pending[0].reject(new Error("late"));assert.equal(await a,false);assert.deepEqual(errors,[]);
  reader.activate(view("two","v2"));assert.equal(pending[1].options.signal.aborted,false);
});

test("range limits hold even for empty returned pages and individually valid oversized lists",async()=>{
  const v=view();let data;const reader=createWindowReader({transport:async()=>data});
  for(const change of [w=>w.line_start=0,w=>w.line_start=3,w=>w.line_start=1.5,w=>w.outline_start=-1,w=>w.outline_start=2,w=>w.outline_start=0.5]){data={...windowFor(v),lines:[],outline:[]};change(data);await assert.rejects(reader.prepare(v,{}),/bounds/);}
  const big=view();big.source.summary={lines:400,declarations:400};data=windowFor(big);data.line_total=400;data.outline_total=400;
  data.lines=Array.from({length:97},(_,i)=>({number:i+1,column:0,end_column:1,line_bytes:1,text:"x",left:false,right:false}));await assert.rejects(reader.prepare(big,{}),/bounds/);
  data.lines=[];data.outline=Array.from({length:201},(_,index)=>({...data.cursor,index,id:`e${index}`}));await assert.rejects(reader.prepare(big,{}),/bounds/);
  for(const field of ["source","summary"]){const bad=view();if(field==="source")delete bad.source;else delete bad.source.summary;data=windowFor(v);await assert.rejects(reader.prepare(bad,{}),/summary/);}
  data=null;await assert.rejects(reader.prepare(v,{}),/window/);
});
test("finishing an obsolete page cannot clear the abort handle of an outstanding page",async()=>{
  const pending=[];const reader=createWindowReader({transport:(uri,options)=>new Promise(resolve=>pending.push({resolve,options}))});const v=view();reader.activate(v);
  const first=reader.request({}),second=reader.request({});pending[0].resolve(windowFor(v));assert.equal(await first,false);
  reader.activate(view("two","v2"));assert.equal(pending[1].options.signal.aborted,true);pending[1].resolve(windowFor(v));assert.equal(await second,false);
});

test("source coordinates and cursor ordinals are integral, and windows cannot pass document ends",async()=>{
  const v=view();let data;const reader=createWindowReader({transport:async()=>data});
  for(const line of [{number:1,column:-1,end_column:2,line_bytes:3,text:"abc",left:false,right:true},{number:1,column:1.5,end_column:4.5,line_bytes:7,text:"abc",left:true,right:true}]){data=windowFor(v);data.lines=[line];await assert.rejects(reader.prepare(v,{}),/fragment/);}
  data=windowFor(v);data.lines=Array.from({length:3},(_,index)=>({...data.lines[0],number:index+1}));await assert.rejects(reader.prepare(v,{}),/fragment/);
  data=windowFor(v);data.cursor={...data.cursor,index:0.5};await assert.rejects(reader.prepare(v,{}),/entry/);
  data=windowFor(v);data.cursor=null;await assert.rejects(reader.prepare(v,{}),/cursor/);
  data=windowFor(v);data.outline[0].index=1;await assert.rejects(reader.prepare(v,{}),/outline window/);
});
