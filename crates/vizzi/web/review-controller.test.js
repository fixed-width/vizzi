import test from "node:test";
import assert from "node:assert/strict";
import { createReviewController } from "./review-controller.js";

const metadata = (token = "pair") => ({token,origin:token, comparison:`comparison-${token}`,provider:"git",revision:"HEAD",resolved:"a".repeat(40),scope:".",working_started_ms:1,working_finished_ms:2});
const row = (name="a.rs",base="v-base",working="v-working") => ({file:name,path:name,path_truncated:false,change:"modified",base_version:base,working_version:working,base:base?{state:"present",kind:"regular",bytes:12}:null,working:working?{state:"present",kind:"regular",bytes:12}:null});
const range = (start=0,end=1) => ({lines:{start,end},bytes:{start:start*12,end:end*12}});
const hunk = (id="h0") => ({id,base:range(),working:range(),edits:{start:0,end:1}});
const window = (file,version) => ({schema:1,file,version,line_total:1,line_start:1,lines:[{number:1,text:"fn one() {}",column:0,end_column:11,line_bytes:11,left:false,right:false}],outline_total:0,outline_start:0,outline:[],cursor:null});
const change = (token="pair",revision="1",patch={}) => ({schema:1,stream:"stream",revision,status:{schema:1,ticket:token,pending:false,active:token?metadata(token):null,error:null,...patch}});
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{resolve,reject,promise};};
const flush=()=>new Promise(setImmediate);
function fixture(override) {
  const views=[],notices=[],requests=[],markers=[],moves=[],statuses=[],errors=[],pushes=[],replacements=[];
  let state,positions={base:{sourceTop:44,outlineTop:0},working:{sourceTop:88,outlineTop:0}};
  const response=async uri=>{
    const url=new URL(uri,"http://local"), token=url.pathname.split("/")[3],meta=metadata(token);
    if(url.pathname.endsWith("/files"))return {schema:1,...meta,files:[row()],selected:url.searchParams.has("file")?row(url.searchParams.get("file")):null,total:1,offset:0,next:null,remaining:0,totals:{paths:1,added:0,removed:0,modified:1,unchanged:0,unassessed:0,unavailable:0}};
    if(url.pathname.endsWith("/diff"))return {schema:1,...meta,file:url.searchParams.get("file"),base_version:url.searchParams.get("base_version"),working_version:url.searchParams.get("working_version"),hunks:[hunk()],edits:[{base:range(),working:range()}],hunk_total:1,edit_total:1,next_hunk:null,next_edit:null,remaining_hunks:0,remaining_edits:0};
    if(url.pathname.endsWith("/window")){const side=url.searchParams.get("side");return {schema:1,token,comparison:meta.comparison,side,window:window(url.searchParams.get("file"),url.searchParams.get("version")),classification:{kind:"assigned",language:"rust",provenance:{kind:"extension",extension:"rs"}},classification_truncated:false,contexts:[],hidden_contexts:0};}
    return {schema:1,ticket:"accepted"};
  };
  const controller=createReviewController({transport:async(uri,options)=>{requests.push({uri,options});return override?override(uri,options,response):response(uri);},onView:v=>views.push(v),onStatus:s=>statuses.push(s),onNotice:n=>notices.push(n),onError:e=>errors.push(e),onHistory:s=>{state=s;},capture:()=>structuredClone(positions),history:{push:m=>{markers.push(m);pushes.push(m);},replace:m=>{markers.push(m);replacements.push(m);},go:d=>moves.push(d)}});
  return {controller,views,notices,requests,markers,moves,statuses,errors,pushes,replacements,setPositions:value=>{positions=value;},state:()=>state,response};
}

test("review delivery assembles bounded views using exact tokens and side versions",async()=>{
  const f=fixture();f.controller.updates().receive(change());await flush();
  assert.equal(f.views.length,1);const v=f.views[0];
  assert.equal(v.metadata.token,"pair");assert.equal(v.file.path,"a.rs");
  assert.equal(v.windows.base.window.version,"v-base");assert.equal(v.windows.working.window.version,"v-working");
  assert.equal(v.selection.hunk,"h0");assert.equal(v.selection.side,"working");
  assert.equal(f.requests.filter(r=>r.uri.includes("/window?")).length,2);
  assert.ok(f.requests.every(r=>r.uri.startsWith("/api/comparisons/pair/")));
  const query=new URL(f.requests[0].uri,"http://local").searchParams;assert.equal(query.get("filter"),"");assert.equal(query.get("unchanged"),"false");assert.equal(query.get("offset"),"0");assert.equal(query.get("count"),"200");
  assert.equal(f.state().count,1);
  assert.deepEqual(f.state(),{count:1,canBack:false,canForward:false,pending:false});assert.equal(f.replacements.length,1);assert.equal(f.pushes.length,0);
});

test("a newer capture rejects every late window and resets positions without moving the baseline",async()=>{
  const held=deferred();let pause=true;
  const f=fixture((uri,options,response)=>uri.includes("/pair/window?") && pause?held.promise:response(uri));
  const s=f.controller.updates();s.receive(change());await flush();
  assert.equal(f.views.length,0);pause=false;s.receive(change("next","2"));await flush();
  assert.equal(f.views.at(-1).metadata.token,"next");
  held.resolve(await f.response("/api/comparisons/pair/window?file=a.rs&side=base&version=v-base"));await flush();
  assert.equal(f.views.length,1);assert.equal(f.errors.length,0);
  assert.ok(f.requests[0].options.signal.aborted);
  const count=f.views.length;s.receive(change("next","3",{pending:true}));s.receive(change("next","4",{error:{code:"capacity",message:"busy"}}));await flush();
  assert.equal(f.views.length,count);assert.equal(f.statuses.at(-1).error.message,"busy");
});

test("failed navigation and mismatched windows cannot replace the coherent view",async()=>{
  let mismatch=false,fail=false;
  const f=fixture(async(uri,options,response)=>{
    if(fail)throw Object.assign(new Error("occupied"),{status:429});
    const body=await response(uri);if(mismatch && body.side)body.side="other";return body;
  });
  f.controller.updates().receive(change());await flush();const original=f.views[0];
  mismatch=true;assert.equal(await f.controller.navigate({side:"base"}),false);assert.equal(f.views.length,1);
  assert.match(f.notices.at(-1),/window|Keeping/);
  mismatch=false;fail=true;assert.equal(await f.controller.navigate({filter:"new"}),false);
  assert.equal(f.views[0],original);assert.equal(f.state().count,1);
});

test("read-only mutations use explicit JSON and errors preserve the displayed pair",async()=>{
  let reject=false;
  const f=fixture((uri,options,response)=>{if(reject)throw new Error("refused");return response(uri);});
  assert.equal(await f.controller.refresh(),false);assert.equal(await f.controller.close(),false);
  assert.equal(await f.controller.create({revision:"ref",scope:"src"}),true);
  let request=f.requests.at(-1);assert.equal(request.uri,"/api/comparisons");assert.equal(request.options.method,"POST");assert.deepEqual(JSON.parse(request.options.body),{provider:"git",revision:"ref",scope:"src"});
  f.controller.updates().receive(change());await flush();
  assert.equal(await f.controller.refresh(),true);request=f.requests.at(-1);assert.equal(request.uri,"/api/comparisons/pair/refresh");assert.equal(request.options.headers["Content-Type"],"application/json");
  reject=true;assert.equal(await f.controller.close(),false);assert.equal(f.views.length,1);
  reject=false;assert.equal(await f.controller.close(),true);assert.equal(f.requests.at(-1).options.method,"DELETE");assert.equal(f.requests.at(-1).uri,"/api/comparisons/pair");assert.equal(f.views.at(-1),null);
});

test("history restores only the exact pair and preserves independent pane positions",async()=>{
  const f=fixture();const s=f.controller.updates();s.receive(change());await flush();
  const original=f.markers[0];await f.controller.navigate({side:"base"});
  assert.equal(f.state().count,2);f.controller.move(-1);assert.deepEqual(f.moves,[-1]);
  assert.equal(await f.controller.traverse(original),true);
  assert.equal(f.views.at(-1).selection.side,"working");assert.equal(f.views.at(-1).windows.base.position.sourceTop,44);assert.equal(f.views.at(-1).windows.working.position.sourceTop,88);
  const count=f.state().count;await f.controller.navigate({filter:"path"});assert.equal(f.state().count,count);
  s.receive(change("replacement","2"));await flush();const shown=f.views.at(-1);
  assert.equal(await f.controller.traverse(original),false);assert.equal(f.views.at(-1),shown);assert.match(f.notices.at(-1),/expired/);
  assert.equal(f.state().canBack,false);
  assert.equal(await f.controller.openCurrent(),true);assert.equal(f.views.at(-1).metadata.token,"replacement");
  const marker=JSON.stringify(f.markers[0]);assert.ok(!marker.includes("v-base"));assert.ok(!marker.includes("window"));
});

test("bounded review history branches after a failed return and ignores foreign browser entries",async()=>{
  const f=fixture();f.controller.updates().receive(change());await flush();const old=f.markers[0];
  for(let i=0;i<55;i++)await f.controller.navigate({side:i%2?"working":"base"});
  assert.equal(f.state().count,50);
  const requests=f.requests.length;assert.equal(await f.controller.traverse({foreign:true}),false);assert.equal(f.requests.length,requests);
  const previous=f.markers.at(-2);assert.equal(await f.controller.traverse(previous),true);
  await f.controller.navigate({side:"base",filter:"branch"});assert.equal(f.state().canForward,false);
  f.controller.move(0);f.controller.move(9);assert.ok(!f.moves.length);
  assert.equal(await f.controller.traverse(old),false);assert.match(f.notices.at(-1),/expired/);
});

test("a discarded marker starts a fresh metadata trail on explicit recovery",async()=>{
  const f=fixture();f.controller.updates().receive(change());await flush();const previous=f.markers[0];
  assert.equal(await f.controller.traverse({vizziReview:{schema:1,session:"expired",key:"discarded"}}),false);
  assert.equal(f.state().count,0);
  assert.equal(await f.controller.openCurrent(),true);assert.notDeepEqual(f.markers.at(-1),previous);assert.equal(f.state().count,1);assert.equal(f.notices.at(-1),null);
});

test("added and removed source stays on its available side; unsupported diff is explicit",async()=>{
  const f=fixture(async(uri,options,response)=>{
    if(uri.includes("/files?")){const body=await response(uri);body.files=[row("a.rs","v-base",null)];return body;}
    if(uri.includes("/diff?"))throw Object.assign(new Error("binary or unavailable"),{status:422});
    return response(uri);
  });
  f.controller.updates().receive(change());await flush();const v=f.views[0];
  assert.equal(v.selection.side,"base");assert.equal(v.windows.working,null);assert.match(v.diffError,/unavailable/);assert.equal(v.diff,null);
  assert.equal(f.requests.filter(r=>r.uri.includes("/window?")).length,1);
  const selected=fixture();selected.controller.updates().receive(change());await flush();
  await selected.controller.navigate({file:row("removed.txt","v-base",null)});assert.equal(selected.views.at(-1).selection.side,"base");
});

test("review streams validate revisions and do not accept stale or replaced subscribers",async()=>{
  const f=fixture();let s=f.controller.updates();
  for(const invalid of [null,{},change("pair","01"),change("pair","18446744073709551616"),{...change(),stream:""},change("pair","1",{pending:"yes"})])assert.throws(()=>s.receive(invalid),/review/);
  assert.equal(s.receive(change()),true);assert.equal(s.receive(change()),false);await flush();
  assert.throws(()=>s.receive({...change("pair","2"),stream:"other"}),/stream/);
  const previous=s;s=f.controller.updates();assert.equal(previous.receive(change("old","9")),false);
  s.receive({...change(),stream:"new"});await flush();assert.equal(f.views.length,1);
  s.close(new Error("disconnected"));assert.match(f.notices.at(-1),/disconnected/);
  assert.equal(f.errors.at(-1).message,"disconnected");
  previous.close(new Error("late"));assert.ok(!f.errors.some(e=>e.message==="late"));
});

test("independent notification fields reject malformed values while exact bounds and null states work",async()=>{
  const f=fixture();
  for(const patch of [{schema:2},{status:null},{stream:["x"]},{stream:"x".repeat(257)},{stream:0},{revision:"-1"},{revision:"1.1"},{revision:0}])assert.throws(()=>f.controller.updates().receive({...change(),...patch}),/review/);
  for(const patch of [{schema:2},{ticket:""},{ticket:["x"]},{ticket:42},{pending:null},{error:undefined},{error:{code:4,message:"x"}},{error:{code:"x",message:null}},{error:5},{active:undefined}])assert.throws(()=>f.controller.updates().receive(change("pair","0",patch)),/review/);
  for(const key of ["token","comparison","provider","revision","resolved","scope"])assert.throws(()=>f.controller.updates().receive(change("pair","0",{active:{...metadata(),[key]:null}})),/review/);
  for(const key of ["working_started_ms","working_finished_ms"])for(const value of [-1,0.5,Number.MAX_SAFE_INTEGER+1,"1"])assert.throws(()=>f.controller.updates().receive(change("pair","0",{active:{...metadata(),[key]:value}})),/review/);
  assert.equal(f.controller.updates().receive({...change(null,"0",{ticket:null}),stream:"x".repeat(256)}),true);await flush();assert.equal(f.views.length,0);
  for(const key of ["token","comparison"])for(const value of ["","x".repeat(257)])assert.throws(()=>f.controller.updates().receive(change("pair","0",{active:{...metadata(),[key]:value}})),/review/);
  assert.equal(f.controller.updates().receive(change("pair","18446744073709551615",{active:{...metadata(),working_started_ms:0,working_finished_ms:0}})),true);await flush();assert.equal(f.views.length,1);
  const current=f.views[0];f.controller.updates().receive(change(null,"0",{ticket:null}));assert.equal(f.views[0],current);assert.match(f.notices.at(-1),/expired|closed/);
});

test("malformed pair, file, diff and source responses cannot publish a partial comparison",async()=>{
  const cases=[
    ["files",()=>null],["files",b=>({...b,schema:2})],["files",b=>({...b,token:"wrong"})],["files",b=>({...b,comparison:"wrong"})],
    ["files",b=>({...b,files:[] ,total:-1})],["files",b=>({...b,files:Array(201).fill(row())})],["files",b=>({...b,files:{}})],
    ["files",b=>({...b,files:[{...row(),file:""}]})],["files",b=>({...b,files:[{...row(),base_version:1}]})],["files",b=>({...b,files:[{...row(),working_version:1}]})],
    ["files",b=>({...b,files:[{...row(),base_version:""}]})],["files",b=>({...b,files:[{...row(),working_version:""}]})],
    ["diff",b=>({...b,file:"wrong"})],["diff",b=>({...b,base_version:"wrong"})],["diff",b=>({...b,working_version:"wrong"})],
    ["diff",b=>({...b,hunks:null})],["diff",b=>({...b,edits:null})],["diff",b=>({...b,hunks:Array(201).fill(hunk())})],["diff",b=>({...b,edits:Array(201).fill({})})],
    ["window",b=>({...b,side:"wrong"})],["window",b=>({...b,window:null})],["window",b=>({...b,window:{...b.window,line_total:-1}})],["window",b=>({...b,window:{...b.window,outline_total:1.5}})],
    ["window",b=>({...b,window:{...b.window,line_total:0.5,lines:[]}})],["window",b=>({...b,window:{...b.window,outline_total:-0.5}})],
  ];
  for(const [endpoint,alter] of cases) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);return uri.includes(`/${endpoint}?`)?alter(b):b;});
    f.controller.updates().receive(change());await flush();assert.equal(f.views.length,0,endpoint);assert.equal(f.errors.length,1,endpoint);assert.ok(f.errors[0].message.length>0);assert.match(f.notices.at(-1),/comparison|review|window/);
  }
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/files?")){b.files=Array(200).fill(row());b.total=200;}return b;});
  f.controller.updates().receive(change());await flush();assert.equal(f.views[0].files.files.length,200);
  const full=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/diff?")){b.hunks=Array.from({length:200},(_,i)=>hunk(`h${i}`));b.edits=Array(200).fill({base:range(),working:range()});b.hunk_total=200;b.edit_total=200;}return b;});full.controller.updates().receive(change());await flush();assert.equal(full.views[0].diff.hunks.length,200);assert.equal(full.views[0].diff.edits.length,200);
});

test("empty projections, absent sides and per-side failures have explicit bounded results",async()=>{
  const empty=fixture(async(uri,options,response)=>({...await response(uri),files:[],total:0}));empty.controller.updates().receive(change());await flush();assert.equal(empty.views[0].file,null);assert.equal(empty.requests.length,1);
  for(const status of [404,413,422,503]) {
    const f=fixture(async(uri,options,response)=>{if(uri.includes("/diff?"))throw Object.assign(new Error("unsupported diff"),{status});if(uri.includes("/files?")){const b=await response(uri);b.files=[row("a.rs",null,"v-working")];return b;}return response(uri);});
    f.controller.updates().receive(change());await flush();assert.equal(f.views[0].windows.base,null);assert.equal(f.views[0].diffError,"unsupported diff");const url=new URL(f.requests.find(r=>r.uri.includes("/diff?")).uri,"http://local");assert.ok(!url.searchParams.has("base_version"));
  }
  for(const status of [404,422,503]) {
    const f=fixture(async(uri,options,response)=>{if(uri.includes("/window?") && uri.includes("side=base"))throw Object.assign(new Error("unreadable"),{status});return response(uri);});f.controller.updates().receive(change());await flush();assert.equal(f.views[0].windows.base.error,"unreadable");assert.ok(f.views[0].windows.working.window);
  }
  const refused=fixture(async(uri,options,response)=>{if(uri.includes("/window?"))throw Object.assign(new Error("occupied"),{status:429});return response(uri);});refused.controller.updates().receive(change());await flush();assert.equal(refused.views.length,0);
});

test("hunk and edit pages stay independent and reveal the selected ranges with fresh source",async()=>{
  const f=fixture(async(uri,options,response)=>{
    const b=await response(uri),u=new URL(uri,"http://local");
    if(uri.includes("/diff?")){const start=Number(u.searchParams.get("hunk_start")),edit=Number(u.searchParams.get("edit_start"));const n=start===200?200:0;b.hunks=[{...hunk(`h${n}`),base:range(9,12),working:range(19,22),edits:{start:n,end:n+1}}];b.edits=[{base:range(10,11),working:range(20,21)}];b.hunk_total=201;b.edit_total=201;b.next_hunk=start?null:200;b.next_edit=edit?null:200;}
    return b;
  });
  f.controller.updates().receive(change());await flush();assert.equal(await f.controller.navigate({hunk_start:200}),true);let v=f.views.at(-1);assert.equal(v.selection.hunk,"h200");assert.equal(v.query.edit_start,200);assert.equal(v.windows.base.position.sourceTop,9*22);assert.equal(v.windows.working.position.sourceTop,19*22);
  assert.ok(f.requests.some(r=>r.uri.includes("edit_start=200")));
  assert.equal(await f.controller.navigate({edit_start:0}),true);assert.equal(f.views.at(-1).query.edit_start,0);
  const count=f.views.length;assert.equal(await f.controller.navigate({hunk:"missing"}),false);assert.equal(f.views.length,count);assert.match(f.notices.at(-1),/hunk/);
});

test("declaration selection is side-specific and carries no hunk into its new reveal",async()=>{
  const entry={id:"declaration",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.window){Object.assign(b.window,{outline_total:1,outline:[entry],cursor:entry});}return b;});
  f.controller.updates().receive(change());await flush();
  assert.equal(await f.controller.navigate({side:"base",entity:{id:entry.id,index:0,start_line:1}}),true);const v=f.views.at(-1);assert.equal(v.selection.entity,entry.id);assert.equal(v.selection.side,"base");assert.equal(v.selection.hunk,null);assert.equal(v.query.mode,"declaration");
  const uri=f.requests.filter(r=>r.uri.includes("/window?") && r.uri.includes("side=base")).at(-1).uri;assert.equal(new URL(uri,"http://local").searchParams.get("cursor"),entry.id);
  const count=f.views.length;assert.equal(await f.controller.navigate({entity:{id:"missing",index:0,start_line:1}}),false);assert.equal(f.views.length,count);
});

test("history records user scrolling at departure and duplicate selections update its metadata",async()=>{
  const f=fixture();f.controller.updates().receive(change());await flush();const original=f.markers[0];
  const positions={base:{sourceTop:2200,sourceLeft:12,outlineTop:480,cursorIndex:8,columnLine:101,column:32768},working:{sourceTop:4400,sourceLeft:24,outlineTop:960,cursorIndex:16,columnLine:201,column:65536}};
  f.setPositions(positions);await f.controller.navigate({side:"base"});assert.equal(f.pushes.length,1);assert.deepEqual(f.state(),{count:2,canBack:true,canForward:false,pending:false});
  f.setPositions({base:{sourceTop:8800,outlineTop:0},working:{sourceTop:9900,outlineTop:0}});
  await f.controller.traverse(original);assert.deepEqual(f.views.at(-1).windows.base.position,positions.base);assert.equal(f.state().canForward,true);
  const request=f.requests.filter(r=>r.uri.includes("/window?") && r.uri.includes("side=working")).at(-1);const query=new URL(request.uri,"http://local").searchParams;
  assert.equal(query.get("line_start"),"185");assert.equal(query.get("outline_start"),"12");assert.equal(query.get("cursor_index"),"16");assert.equal(query.get("column_line"),"201");assert.equal(query.get("column"),"65536");
  await f.controller.navigate({filter:"new-filter"});assert.equal(f.pushes.length,1);await f.controller.navigate({side:"base"});assert.equal(f.state().canForward,false);
  const before=f.moves.length;f.controller.move(1);assert.equal(f.moves.length,before);f.controller.move(-1);assert.equal(f.moves.at(-1),-1);
});

test("known history ends and foreign session/schema markers cannot issue source queries",async()=>{
  for(const patch of [{session:"foreign"},{schema:2}]) {
    const f=fixture();f.controller.updates().receive(change());await flush();const marker=f.markers[0];const count=f.requests.length;
    assert.equal(await f.controller.traverse({vizziReview:{...marker.vizziReview,...patch}}),false);assert.equal(f.requests.length,count);
  }
  const f=fixture();const s=f.controller.updates();s.receive(change());await flush();const marker=f.markers[0];await f.controller.navigate({side:"base"});await f.controller.traverse(marker);f.controller.move(1);assert.equal(f.moves.at(-1),1);f.controller.move(-1);assert.deepEqual(f.moves,[1]);
  assert.equal(await f.controller.traverse(null),false);
  const count=f.requests.length;s.receive(change("pair","2",{active:{...metadata(),comparison:"different"}}));assert.equal(await f.controller.traverse(marker),false);assert.equal(f.requests.length,count);
});

test("no textual hunks remains a normal inspectable file and failed lifecycle requests report errors",async()=>{
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/diff?"))Object.assign(b,{hunks:[],edits:[],hunk_total:0,edit_total:0});return b;});f.controller.updates().receive(change());await flush();assert.equal(f.views[0].selection.hunk,null);assert.equal(f.views[0].diffError,null);
  const failed=fixture(()=>{throw new Error("request refused");});assert.equal(await failed.controller.create({revision:"HEAD"}),false);assert.equal(failed.errors[0].message,"request refused");assert.equal(failed.notices.at(-1),"request refused");
});

test("same-pair obsolete selections and shutdown cannot revive a superseded view",async()=>{
  let hold=false;const pending=[];
  const f=fixture((uri,options,response)=>{if(hold && uri.includes("/files?")){const p=deferred();pending.push({p,uri,options});return p.promise;}return response(uri);});
  assert.equal(await f.controller.navigate(),false);assert.equal(await f.controller.openCurrent(),false);
  f.controller.updates().receive(change());await flush();hold=true;
  const first=f.controller.navigate({filter:"first"});assert.equal(f.state().pending,true);
  const second=f.controller.navigate({filter:"second"});assert.equal(pending[0].options.signal.aborted,true);
  pending[1].p.resolve(await f.response(pending[1].uri));assert.equal(await second,true);const count=f.views.length;
  pending[0].p.resolve(await f.response(pending[0].uri));assert.equal(await first,false);assert.equal(f.views.length,count);assert.equal(f.views.at(-1).query.filter,"second");
  assert.equal(f.requests.filter(r=>r.options.signal===pending[0].options.signal).length,1);
  const last=f.controller.navigate({filter:"held"});assert.equal(await f.controller.close(),true);assert.equal(f.state().pending,false);
  pending[2].p.resolve(await f.response(pending[2].uri));assert.equal(await last,false);assert.equal(f.views.at(-1),null);
});

test("declaration reveals use nonzero ordinals and no hunk coordinates are reused after changing files",async()=>{
  const entry={id:"declaration",index:8,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:90,end_line:95};
  const f=fixture(async(uri,options,response)=>{
    const b=await response(uri);if(b.window)Object.assign(b.window,{line_total:200,line_start:1,lines:[],outline_total:20,outline_start:0,outline:[],cursor:entry});return b;
  });
  f.controller.updates().receive(change());await flush();
  await f.controller.navigate({side:"base",entity:{id:entry.id,index:8,start_line:90}});
  let v=f.views.at(-1);assert.equal(v.windows.base.position.sourceTop,86*22);assert.equal(v.windows.base.position.outlineTop,5*48);assert.equal(v.windows.base.position.cursorIndex,8);
  let request=f.requests.filter(r=>r.uri.includes("/window?") && r.uri.includes("side=base")).at(-1);assert.equal(new URL(request.uri,"http://local").searchParams.get("outline_start"),"5");
  await f.controller.navigate({file:row("other.rs")});v=f.views.at(-1);assert.equal(v.selection.entity,null);assert.equal(v.selection.side,"working");assert.equal(v.query.hunk_start,0);assert.equal(v.query.edit_start,0);assert.equal(v.query.mode,"hunk");assert.equal(v.windows.base.position.sourceTop,0);
});

test("file, hunk and declaration visits are distinct while a filter update is retained on return",async()=>{
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/diff?")){b.hunks=[hunk("h0"),{...hunk("h1"),base:range(10,12),working:range(20,22),edits:{start:200,end:201}}];b.edits=Array(200).fill({base:range(),working:range()});b.edit_total=201;}return b;});
  f.controller.updates().receive(change());await flush();const original=f.markers[0];
  await f.controller.navigate({filter:"saved-filter"});assert.equal(f.state().count,1);
  await f.controller.navigate({file:row("other.rs")});assert.equal(f.state().count,2);
  await f.controller.navigate({hunk:"h1"});assert.equal(f.state().count,3);assert.equal(f.views.at(-1).windows.base.position.sourceTop,220);assert.equal(f.views.at(-1).windows.working.position.sourceTop,440);assert.equal(f.views.at(-1).query.edit_start,200);
  await f.controller.navigate({edit_start:0});const selected=f.markers.at(-1);
  await f.controller.navigate({hunk:"h0"});assert.equal(f.views.at(-1).query.edit_start,0);
  await f.controller.traverse(selected);assert.equal(f.views.at(-1).query.edit_start,0);
  await f.controller.traverse(original);assert.equal(f.views.at(-1).query.filter,"saved-filter");
  await f.controller.navigate({file:row("branched.rs")});assert.equal(f.state().count,2);
});

test("declaration identities from one side cannot be sent to the other document",async()=>{
  const entry=side=>({id:`${side}-entity`,index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1});
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.window){const side=new URL(uri,"http://local").searchParams.get("side");Object.assign(b.window,{outline_total:1,outline:[entry(side)],cursor:entry(side)});}return b;});
  f.controller.updates().receive(change());await flush();assert.equal(await f.controller.navigate({side:"working",entity:{id:"working-entity",index:0,start_line:1}}),true);
  const base=f.requests.filter(r=>r.uri.includes("/window?") && r.uri.includes("side=base")).at(-1);assert.equal(new URL(base.uri,"http://local").searchParams.has("cursor"),false);
});

test("replacement and closed subscriptions cancel pending work, and repeated active status does not restart it",async()=>{
  let hold=true;const requests=[];
  const f=fixture((uri,options,response)=>{if(hold && uri.includes("/files?")){const p=deferred();requests.push({p,uri,options});return p.promise;}return response(uri);});
  let s=f.controller.updates();s.receive(change());s.receive(change("pair","2",{pending:true}));assert.equal(requests.length,1);
  s.close();assert.equal(requests[0].options.signal.aborted,true);assert.equal(f.state().pending,false);assert.equal(f.errors.length,0);
  requests[0].p.resolve(await f.response(requests[0].uri));await flush();assert.equal(f.views.length,0);
  s=f.controller.updates();s.receive(change());assert.equal(requests.length,2);
  f.controller.updates();assert.equal(requests[1].options.signal.aborted,true);assert.equal(f.state().pending,false);
  hold=false;s=f.controller.updates();s.receive(change());await flush();assert.equal(f.views.length,1);
  s.receive(change("replacement","2"));await flush();assert.match(f.notices.at(-1),/positions reset/);assert.equal(f.state().count,2);
});

test("default callback/position/history adapters support standalone native clients",async()=>{
  const f=fixture();let shown;
  const c=createReviewController({transport:f.response,onView:v=>{shown=v;}});c.updates().receive(change());await flush();assert.ok(shown);assert.equal(await c.navigate({filter:"source"}),true);
  assert.equal(await c.navigate({file:row("other.rs")}),true);
});

test("returns never rewrite another visit and duplicate-file selection keeps reading position",async()=>{
  const f=fixture();f.controller.updates().receive(change());await flush();const a=f.markers[0];
  await f.controller.navigate({file:row("b.rs")});const b=f.markers.at(-1);await f.controller.navigate({file:row("c.rs")});const c=f.markers.at(-1);
  await f.controller.traverse(b);assert.equal(f.views.at(-1).file.path,"b.rs");await f.controller.traverse(c);assert.equal(f.views.at(-1).file.path,"c.rs");await f.controller.traverse(a);assert.equal(f.views.at(-1).file.path,"a.rs");
  const positions={base:{sourceTop:2200,outlineTop:0},working:{sourceTop:4400,outlineTop:0}};f.setPositions(positions);await f.controller.navigate({file:row()});assert.equal(f.views.at(-1).windows.working.position.sourceTop,4400);
});

test("an obsolete completion cannot clear the newer pending request's cancellation handle",async()=>{
  let hold=false;const requests=[];
  const f=fixture((uri,options,response)=>{if(hold && uri.includes("/files?")){const p=deferred();requests.push({p,uri,options});return p.promise;}return response(uri);});f.controller.updates().receive(change());await flush();hold=true;
  const a=f.controller.navigate({filter:"a"}),b=f.controller.navigate({filter:"b"});requests[0].p.resolve(await f.response(requests[0].uri));assert.equal(await a,false);assert.equal(f.state().pending,true);
  const c=f.controller.navigate({filter:"c"});assert.equal(requests[1].options.signal.aborted,true);requests[1].p.resolve(await f.response(requests[1].uri));assert.equal(await b,false);requests[2].p.resolve(await f.response(requests[2].uri));assert.equal(await c,true);
});

test("a known replacement blocks old-pair navigation while the fresh view is still loading",async()=>{
  const held=deferred();const f=fixture((uri,options,response)=>uri.includes("/replacement/files?")?held.promise:response(uri));const s=f.controller.updates();s.receive(change());await flush();
  s.receive(change("replacement","2"));assert.equal(await f.controller.navigate({filter:"old"}),false);assert.equal(f.errors.length,0);assert.match(f.notices.at(-1),/loading/);assert.equal(f.views.length,1);
  held.resolve(await f.response("/api/comparisons/replacement/files?"));await flush();assert.equal(f.views.length,2);assert.equal(f.views.at(-1).metadata.token,"replacement");
});

test("hunk selection clears a lexical selection and distinct declarations append visits",async()=>{
  const make=(id,index)=>({id,index,analysis:0,name:id,kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1}),entries=[make("one",0),make("two",1)];
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.window){const cursor=new URL(uri,"http://local").searchParams.get("cursor");Object.assign(b.window,{outline_total:2,outline:entries,cursor:entries.find(e=>e.id===cursor) ?? entries[0]});}return b;});f.controller.updates().receive(change());await flush();await f.controller.navigate({entity:{id:"one",index:0,start_line:1}});await f.controller.navigate({entity:{id:"two",index:1,start_line:1}});assert.equal(f.state().count,3);
  await f.controller.navigate({hunk:"h0"});assert.equal(f.views.at(-1).selection.entity,null);assert.equal(f.views.at(-1).query.mode,"hunk");assert.equal(f.views.at(-1).selection.hunk,"h0");
});

test("external closure cancels in-flight inspection and clears the navigation busy state",async()=>{
  let hold=false;const held=deferred();let signal;
  const f=fixture((uri,options,response)=>{if(hold && uri.includes("/files?")){signal=options.signal;return held.promise;}return response(uri);});const s=f.controller.updates();s.receive(change());await flush();hold=true;
  const pending=f.controller.navigate({filter:"pending"});assert.equal(f.state().pending,true);s.receive(change(null,"2",{ticket:null}));assert.equal(signal.aborted,true);assert.equal(f.state().pending,false);
  held.resolve(await f.response("/api/comparisons/pair/files?"));assert.equal(await pending,false);
  assert.equal(await f.controller.traverse(f.markers[0]),false);
});

test("return and open-current actions save the location being left, and foreign returns cancel pending work",async()=>{
  const f=fixture();f.controller.updates().receive(change());await flush();const a=f.markers[0];await f.controller.navigate({file:row("b.rs")});const b=f.markers.at(-1);
  f.setPositions({base:{sourceTop:3300,outlineTop:0},working:{sourceTop:5500,outlineTop:0}});await f.controller.traverse(a);await f.controller.traverse(b);assert.equal(f.views.at(-1).windows.working.position.sourceTop,5500);
  f.setPositions({base:{sourceTop:6600,outlineTop:0},working:{sourceTop:7700,outlineTop:0}});await f.controller.openCurrent();await f.controller.traverse(b);assert.equal(f.views.at(-1).windows.working.position.sourceTop,7700);
  let hold=false;const held=deferred();let signal;const g=fixture((uri,options,response)=>{if(hold && uri.includes("/files?")){signal=options.signal;return held.promise;}return response(uri);});g.controller.updates().receive(change());await flush();hold=true;const pending=g.controller.navigate({filter:"pending"});await g.controller.traverse({vizziReview:{schema:1,session:"foreign",key:"unknown"}});assert.equal(signal.aborted,true);held.resolve(await g.response("/api/comparisons/pair/files?"));assert.equal(await pending,false);
});

test("a clean comparison can stay empty, then open unchanged context without losing history",async()=>{
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/files?") && new URL(uri,"http://local").searchParams.get("unchanged")!=="true")Object.assign(b,{files:[],total:0});return b;});
  f.controller.updates().receive(change(null,"0",{ticket:null}));assert.equal(f.notices.length,0);f.controller.updates().receive(change());await flush();assert.equal(f.views[0].file,null);
  assert.equal(await f.controller.navigate({filter:"empty"}),true);assert.equal(f.state().count,1);
  assert.equal(await f.controller.navigate({unchanged:true}),true);assert.equal(f.views.at(-1).file.file,"a.rs");
  const g=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/files?"))Object.assign(b,{files:[],total:0});return b;});g.controller.updates().receive(change());await flush();assert.equal(await g.controller.navigate({file:row()}),true);
});

test("same-content token replacement still expires history and explicit recovery branches the browser trail",async()=>{
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="new")b.comparison=metadata().comparison;return b;});const s=f.controller.updates();s.receive(change());await flush();const old=f.markers[0];
  s.receive(change("new","2",{active:{...metadata("new"),comparison:metadata().comparison}}));await flush();const count=f.requests.length,pushes=f.pushes.length;
  assert.equal(await f.controller.traverse(old),false);assert.equal(f.requests.length,count);
  await f.controller.openCurrent();assert.equal(f.pushes.length,pushes+1);assert.equal(f.state().canForward,false);
});

test("malformed envelopes and absent declaration cursors have actionable public errors",async()=>{
  const f=fixture(async(uri,options,response)=>uri.includes("/files?")?null:response(uri));f.controller.updates().receive(change());await flush();assert.match(f.errors[0].message,/Mismatched comparison response/);
  const g=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.window)b.window=null;return b;});g.controller.updates().receive(change());await flush();assert.match(g.errors[0].message,/Invalid review window totals/);
  const h=fixture();h.controller.updates().receive(change());await flush();assert.equal(await h.controller.navigate({entity:{id:"missing",index:0,start_line:1}}),false);assert.match(h.errors.at(-1).message,/Saved declaration is unavailable/);
});

test("late supported IO failures do not initiate more obsolete source work",async()=>{
  for(const endpoint of ["diff","window"]) {
    let hold=false;const held=deferred();let signal;
    const f=fixture((uri,options,response)=>{if(hold && uri.includes(`/${endpoint}?`)){hold=false;signal=options.signal;return held.promise;}return response(uri);});f.controller.updates().receive(change());await flush();hold=true;
    const old=f.controller.navigate({filter:"held"});await flush();const count=f.requests.filter(r=>r.options.signal===signal).length;await f.controller.navigate({filter:"current"});held.reject(Object.assign(new Error("old IO"),{status:503}));assert.equal(await old,false);assert.equal(f.requests.filter(r=>r.options.signal===signal).length,count);
  }
});

test("a same-origin refresh keeps an off-page file, side, projection and unchanged pane positions",async()=>{
  const f=fixture(async(uri,options,response)=>{
    const b=await response(uri),p=new URL(uri,"http://local").searchParams;
    if(b.token==="next")b.origin="pair";
    if(uri.includes("/files?"))Object.assign(b,{total:500,offset:Number(p.get("offset")),next:400,remaining:299});
    return b;
  });
  const s=f.controller.updates();s.receive(change());await flush();const old=f.markers[0];
  await f.controller.navigate({file:row("z.rs"),side:"base"});
  await f.controller.navigate({side:"base",filter:"narrow",unchanged:true,offset:200});
  const positions={base:{sourceTop:2200,sourceLeft:12,outlineTop:480,cursorIndex:8,columnLine:101,column:32768},working:{sourceTop:4400,sourceLeft:24,outlineTop:960,cursorIndex:16,columnLine:201,column:65536}};
  f.setPositions(positions);
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();
  const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.file.file,"z.rs");assert.equal(v.selection.side,"base");
  assert.equal(v.query.filter,"narrow");assert.equal(v.query.unchanged,true);assert.equal(v.query.offset,200);assert.equal(v.selection.hunk,null);assert.equal(v.query.hunk_start,0);assert.equal(v.query.edit_start,0);
  assert.deepEqual(v.windows.base.position,positions.base);assert.deepEqual(v.windows.working.position,positions.working);
  const lookup=f.requests.find(r=>r.uri.startsWith("/api/comparisons/next/files?"));assert.equal(new URL(lookup.uri,"http://local").searchParams.get("file"),"z.rs");
  const before=f.views.length;assert.equal(await f.controller.traverse(old),false);assert.equal(f.views.length,before);assert.match(f.notices.at(-1),/expired/);
});

test("changed selected declarations use current identity locations while unchanged opposite panes keep their positions",async()=>{
  const entry=fresh=>({id:"kept",index:fresh?12:8,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:fresh?120:90,end_line:fresh?125:95});
  const f=fixture(async(uri,options,response)=>{
    const b=await response(uri),fresh=b.token==="next";
    if(fresh){b.origin="pair";if(uri.includes("/files?"))b.selected=row("a.rs","v-base","next-working");}
    if(b.window){const e=entry(fresh),selected=new URL(uri,"http://local").searchParams.get("cursor")==="kept";Object.assign(b.window,{line_total:200,outline_total:20,outline_start:e.index,outline:[e],cursor:selected?e:{...e,id:"first",index:0,start_line:1,end_line:1}});}
    return b;
  });
  const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({entity:{id:"kept",index:8,start_line:90}});
  const base={sourceTop:2200,sourceLeft:14,outlineTop:480,cursorIndex:8,columnLine:101,column:32768};
  f.setPositions({base,working:{sourceTop:8800,sourceLeft:33,outlineTop:432,cursorIndex:8,columnLine:90,column:65536}});
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();
  const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.file.working_version,"next-working");assert.equal(v.selection.entity,"kept");assert.equal(v.query.entity.start_line,120);assert.equal(v.query.entity.index,12);
  assert.deepEqual(v.windows.base.position,base);assert.equal(v.windows.working.position.sourceTop,116*22);assert.equal(v.windows.working.position.outlineTop,9*48);assert.equal(v.windows.working.position.sourceLeft,0);assert.equal(v.windows.working.position.column,0);
  const requests=f.requests.filter(r=>r.uri.startsWith("/api/comparisons/next/window?") && new URL(r.uri,"http://local").searchParams.get("side")==="working").map(r=>new URL(r.uri,"http://local").searchParams);
  assert.ok(requests.some(q=>q.get("line_start")==="101"));
  for(const q of requests){assert.equal(q.get("version"),"next-working");assert.notEqual(q.get("cursor_index"),"8");assert.equal(q.has("column_line"),false);}
  assert.match(f.notices.at(-1),/working source changed.*current declaration/);assert.doesNotMatch(f.notices.at(-1),/file start|Hunk/);
});

test("refresh page shrink is bounded and selections survive becoming unchanged or disappearing from the filter",async()=>{
  const calls=[];
  const f=fixture(async(uri,options,response)=>{
    const b=await response(uri);if(b.token==="next")b.origin="pair";
    if(uri.includes("/files?")) {
      const p=new URL(uri,"http://local").searchParams,offset=Number(p.get("offset"));
      if(b.token==="next") {calls.push(offset);Object.assign(b,{total:201,offset:Math.min(offset,201),files:offset<201?[row("visible.rs")]:[],selected:row("z.rs"),remaining:0,next:null});b.selected.change="unchanged";}
      else b.total=1000;
    }
    return b;
  });
  const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({file:row("z.rs")});await f.controller.navigate({offset:800,filter:"visible"});
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);
  assert.equal(v.file.file,"z.rs");assert.equal(v.file.change,"unchanged");assert.equal(v.query.unchanged,false);assert.equal(v.query.filter,"visible");assert.equal(v.query.offset,200);assert.deepEqual(calls,[800,200]);
});

test("missing files choose a fresh visible row or explicit empty projection without old anchors",async()=>{
  for(const files of [[row("replacement.rs",null,"new-only")],[]]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?"))Object.assign(b,{selected:null,files,total:files.length});}return b;});
    const s=f.controller.updates();s.receive(change());await flush();f.setPositions({working:{sourceTop:9000,outlineTop:480,columnLine:44,column:65536}});
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);
    assert.equal(v.metadata.token,"next");assert.equal(v.file?.file,files[0]?.file);assert.equal(v.selection.entity,null);assert.equal(v.selection.hunk,null);assert.equal(v.query.hunk_start,0);assert.equal(v.windows.working?.position.sourceTop,files.length?0:undefined);assert.match(f.notices.at(-1),/absent/);
    if(!files.length)assert.match(f.notices.at(-1),/No file/);
  }
});

test("a removed side switches explicitly while unretained source keeps its selected side and availability",async()=>{
  for(const kind of ["absent","unavailable"]) {
    const f=fixture(async(uri,options,response)=>{
      const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?")){b.selected=row("a.rs","v-base",null);if(kind==="unavailable")b.selected.working={state:"unobserved",unavailable:"unreadable"};}}return b;
    });
    const s=f.controller.updates();s.receive(change());await flush();s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);
    assert.equal(v.selection.side,kind==="absent"?"base":"working");assert.equal(v.windows.working,null);assert.ok(v.windows.base);if(kind==="absent")assert.match(f.notices.at(-1),/side is absent/);
  }
});

test("reconnects can coalesce refreshes without a local POST, but identical new captures and legacy labels reset context",async()=>{
  for(const preserve of [true,false,"legacy"]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.comparison=metadata().comparison;b.origin=preserve===true?"pair":"new-create";if(preserve==="legacy")delete b.origin;}return b;});
    f.controller.updates().receive(change());await flush();await f.controller.navigate({file:row("chosen.rs")});await f.controller.navigate({filter:"saved",unchanged:true});
    const meta={...metadata("next"),comparison:metadata().comparison,origin:preserve===true?"pair":"new-create"};if(preserve==="legacy")delete meta.origin;
    const s=f.controller.updates();s.receive({...change("next","0",{active:meta}),stream:"reconnected"});await flush();const v=f.views.at(-1);
    assert.equal(v.metadata.token,"next");assert.equal(v.file.file,preserve===true?"chosen.rs":"a.rs");assert.equal(v.query.filter,preserve===true?"saved":"");assert.equal(v.query.unchanged,preserve===true);
  }
});

test("failed fresh declaration reads keep the coherent pair and retry resolves against the new origin",async()=>{
  const e={id:"kept",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};let fail=true;
  const f=fixture(async(uri,options,response)=>{const b=await response(uri),fresh=b.token==="next";if(fresh){b.origin="pair";if(uri.includes("/files?"))b.selected=row("a.rs","v-base","fresh");if(fail && uri.includes("/window?"))throw Object.assign(new Error("backing temporarily unavailable"),{status:503});}if(b.window)Object.assign(b.window,{outline_total:1,outline:[e],cursor:e});return b;});
  const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({entity:{id:e.id,index:0,start_line:1}});const old=f.views.at(-1);
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();assert.equal(f.views.at(-1),old);assert.equal(f.state().pending,false);assert.match(f.notices.at(-1),/Keeping/);
  fail=false;assert.equal(await f.controller.navigate(),true);assert.equal(f.views.at(-1).metadata.token,"next");assert.equal(f.views.at(-1).selection.entity,e.id);assert.equal(f.views.at(-1).file.working_version,"fresh");assert.equal(f.views.at(-1).windows.working.window.version,"fresh");
});

test("old navigation and history cannot cancel a pending replacement, and late refreshes cannot overwrite a newer capture",async()=>{
  const held=deferred();let signal;
  const f=fixture((uri,options,response)=>{if(uri.includes("/next/files?")){signal=options.signal;return held.promise;}return response(uri);});const s=f.controller.updates();s.receive(change());await flush();const marker=f.markers[0];
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));assert.equal(await f.controller.navigate({filter:"old"}),false);assert.equal(await f.controller.traverse(marker),false);assert.equal(signal.aborted,false);assert.equal(f.state().pending,true);
  s.receive(change("latest","3"));await flush();assert.equal(signal.aborted,true);assert.equal(f.views.at(-1).metadata.token,"latest");const count=f.views.length;
  const old=await f.response("/api/comparisons/next/files?file=a.rs");old.origin="pair";held.resolve(old);await flush();assert.equal(f.views.length,count);
});

test("invalid origins and inconsistent refreshed file pins never replace accepted source",async()=>{
  const f=fixture();const s=f.controller.updates();s.receive(change());await flush();
  for(const origin of [null,"",1,"x".repeat(257)])assert.throws(()=>s.receive(change("next","2",{active:{...metadata("next"),origin}})),/origin/);
  for(const selected of [undefined,row("wrong.rs"),{...row(),working_version:1}]) {
    const g=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?"))b.selected=selected;}return b;});const events=g.controller.updates();events.receive(change());await flush();const old=g.views.at(-1);
    events.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();assert.equal(g.views.at(-1),old);assert.ok(g.errors.length);assert.match(g.notices.at(-1),/Invalid/);
  }
});

test("refresh notices distinguish preserved context, changed source and cleared hunk selection",async()=>{
  for(const changed of [false,true]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(changed && uri.includes("/files?"))b.selected=row("a.rs","v-base","changed");}return b;});
    const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({side:"base",entity:null});
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();
    assert.equal(f.views.at(-1).metadata.token,"next");
    assert.equal(f.notices.at(-1),changed?"working source changed; showing file start.":"Refreshed comparison; file and reading positions kept.");
  }
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next")b.origin="pair";return b;});const s=f.controller.updates();s.receive(change());await flush();
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();assert.equal(f.notices.at(-1),"Hunk selection cleared for the new comparison.");
});

test("refresh of an empty comparison can select a newly added or removed file without old reading coordinates",async()=>{
  for(const current of [row("new.rs",null,"new-version"),row("removed.rs","old-base",null),null]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/files?")){b.files=b.token==="pair" || !current?[]:[current];b.total=b.files.length;if(b.token==="next")b.selected=null;}if(b.token==="next")b.origin="pair";return b;});
    const s=f.controller.updates();s.receive(change());await flush();assert.equal(f.views.at(-1).file,null);
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.file?.file,current?.file);
    if(current){const side=current.working?"working":"base";assert.equal(v.selection.side,side);assert.equal(v.windows[side].position.sourceTop,0);}assert.equal(v.selection.entity,null);assert.equal(v.selection.hunk,null);assert.doesNotMatch(f.notices.at(-1),/Selected file is absent/);
  }
});

test("file loss explains the chosen fallback or absence of matching rows",async()=>{
  for(const current of [row("removed.rs","old-base",null),null]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?"))Object.assign(b,{selected:null,files:current?[current]:[],total:current?1:0});}return b;});const s=f.controller.updates();s.receive(change());await flush();
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();assert.equal(f.views.at(-1).metadata.token,"next");
    assert.equal(f.notices.at(-1),current?"Selected file is absent from the refreshed inventory. Showing the first available file.":"Selected file is absent from the refreshed inventory. No file matches the current view.");if(current)assert.equal(f.views.at(-1).selection.side,"base");
  }
});

test("a selected declaration is never transferred to another side or an unavailable document",async()=>{
  const e={id:"kept",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};
  for(const absent of [true,false]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?")){b.selected=row("a.rs","v-base",null);if(!absent)b.selected.working={state:"unobserved",unavailable:"unreadable"};}}if(b.window)Object.assign(b.window,{outline_total:1,outline:[e],cursor:e});return b;});
    const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({side:"working",entity:{id:e.id,index:0,start_line:1}});
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.selection.entity,null);assert.equal(v.selection.side,absent?"base":"working");
    assert.equal(f.notices.at(-1),absent?"working side is absent; showing base. working source changed; showing file start.":"Saved declaration source is unavailable in the refreshed capture. working source changed; showing file start.");
  }
});

test("changed base versions are handled conservatively and unchanged declaration positions need no fresh reveal",async()=>{
  const e={id:"base-kept",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};
  for(const changed of [true,false]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?"))b.selected=row("a.rs",changed?"new-base":"v-base","v-working");}if(b.window)Object.assign(b.window,{outline_total:1,outline:[e],cursor:e});return b;});
    const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({side:"base",entity:{id:e.id,index:0,start_line:1}});const positions={base:{sourceTop:1000,outlineTop:480},working:{sourceTop:2200,outlineTop:960}};f.setPositions(positions);
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.selection.entity,e.id);assert.deepEqual(v.windows.working.position,positions.working);
    assert.equal(v.windows.base.position.sourceTop,changed?0:1000);assert.equal(f.notices.at(-1),changed?"base source changed; showing the current declaration.":"Refreshed comparison; file and reading positions kept.");
  }
});

test("an offset at the matching end moves to the last page while empty results need no second inventory query",async()=>{
  for(const total of [200,0]) {
    const calls=[];
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next")b.origin="pair";if(uri.includes("/files?")){const offset=Number(new URL(uri,"http://local").searchParams.get("offset"));if(b.token==="next"){calls.push(offset);Object.assign(b,{total,files:offset<total?[row()]:[],selected:row(),offset:Math.min(offset,total)});}else b.total=600;}return b;});const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({offset:200});
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.query.offset,0);assert.equal(v.file.file,"a.rs");assert.deepEqual(calls,total?[200,0]:[200]);
  }
});

test("response origins must match when known, and unknown legacy origins never imply context preservation",async()=>{
  for(const legacy of [false,true]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.origin!==undefined)b.origin="other";return b;});const meta=metadata();if(legacy)delete meta.origin;
    f.controller.updates().receive(change("pair","1",{active:meta}));await flush();
    if(legacy)assert.equal(f.views.length,1);else{assert.equal(f.views.length,0);assert.match(f.errors.at(-1).message,/origin/);}
  }
});

test("navigation after external closure or replacement failure cannot query a retired pair",async()=>{
  for(const closed of [true,false]) {
    let fail=false;const f=fixture((uri,options,response)=>fail?Promise.reject(new Error("unavailable")):response(uri));const s=f.controller.updates();s.receive(change());await flush();fail=true;
    s.receive(closed?change(null,"2",{ticket:null}):change("new-create","2"));await flush();const count=f.requests.length;
    assert.equal(await f.controller.navigate(),false);assert.equal(f.requests.length,count);assert.match(f.notices.at(-1),/expired.*current comparison/);
  }
});

test("hunk navigation fetches new edit metadata at a page boundary and avoids repeating an already loaded page",async()=>{
  const f=fixture(async(uri,options,response)=>{
    const b=await response(uri),p=new URL(uri,"http://local").searchParams;
    if(uri.includes("/diff?")){const start=Number(p.get("edit_start"));b.hunks=[hunk(),{...hunk("later"),base:range(300,302),working:range(400,402),edits:{start:200,end:202}}];b.hunk_total=2;b.edit_total=202;b.edits=Array.from({length:start===200?2:200},()=>({base:range(),working:range()}));}
    if(b.window){const start=Number(p.get("line_start"));b.window.line_total=1000;b.window.line_start=start;b.window.lines=Array.from({length:Math.min(96,1001-start)},(_,i)=>{const text=`line ${start+i}`;return{number:start+i,text,column:0,end_column:text.length,line_bytes:text.length,left:false,right:false};});}
    return b;
  });
  f.controller.updates().receive(change());await flush();const diffs=()=>f.requests.filter(r=>r.uri.includes("/diff?")).length;assert.equal(diffs(),1);
  let before=diffs();assert.equal(await f.controller.navigate({hunk:"later"}),true);assert.equal(f.views.at(-1).query.edit_start,200);assert.equal(f.views.at(-1).diff.edits.length,2);assert.equal(diffs()-before,2);
  before=diffs();assert.equal(await f.controller.navigate({filter:"still-current"}),true);assert.equal(diffs()-before,1);assert.equal(f.views.at(-1).query.edit_start,200);
  await f.controller.navigate({hunk:"h0"});await f.controller.navigate({edit_start:200});assert.equal(f.views.at(-1).query.edit_start,200);
  before=diffs();await f.controller.navigate({filter:"selected-hunk"});assert.equal(f.views.at(-1).query.edit_start,0);assert.equal(diffs()-before,2);
});

test("an explicitly selected absent base falls back to working without transferring declaration state",async()=>{
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(uri.includes("/files?")){b.files=[row("a.rs",null,"v-working")];if(b.token==="next")b.selected=b.files[0];}if(b.token==="next")b.origin="pair";return b;});const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({side:"base",entity:null});
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.metadata.token,"next");assert.equal(v.selection.side,"working");assert.equal(v.selection.entity,null);assert.equal(f.notices.at(-1),"base side is absent; showing working.");
});

test("a selected declaration survives successive working refreshes rather than only the first replacement",async()=>{
  const e={id:"kept",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token!=="pair"){b.origin="pair";if(uri.includes("/files?"))b.selected=row("a.rs","v-base",`version-${b.token}`);}if(b.window)Object.assign(b.window,{outline_total:1,outline:[e],cursor:e});return b;});const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({entity:{id:e.id,index:0,start_line:1}});
  for(const [token,revision] of [["one","2"],["two","3"],["three","4"]]) {s.receive(change(token,revision,{active:{...metadata(token),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.metadata.token,token);assert.equal(v.selection.entity,e.id);assert.equal(v.windows.working.window.version,`version-${token}`);}
});

test("changed nonselected source reports its own fresh position while a selected declaration remains on its side",async()=>{
  const e={id:"kept",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};
  const f=fixture(async(uri,options,response)=>{const b=await response(uri);if(b.token==="next"){b.origin="pair";if(uri.includes("/files?"))b.selected=row("a.rs","changed-base","v-working");}if(b.window)Object.assign(b.window,{outline_total:1,outline:[e],cursor:e});return b;});const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({side:"working",entity:{id:e.id,index:0,start_line:1}});f.setPositions({base:{sourceTop:4400,outlineTop:960},working:{sourceTop:2200,outlineTop:480}});
  s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);assert.equal(v.selection.entity,e.id);assert.equal(v.windows.base.position.sourceTop,0);assert.equal(v.windows.working.position.sourceTop,2200);assert.equal(f.notices.at(-1),"base source changed; showing file start.");
});

test("a return marker before initial inspection completes safely cancels the pending first view",async()=>{
  const held=deferred();let signal;const f=fixture((uri,options)=>{signal=options.signal;return held.promise;});f.controller.updates().receive(change());
  assert.equal(await f.controller.traverse({vizziReview:{schema:1,session:"prior-browser-session",key:"old"}}),false);assert.equal(signal.aborted,true);assert.equal(f.state().pending,false);
  held.resolve(await f.response("/api/comparisons/pair/files?"));await flush();assert.equal(f.views.length,0);
});

test("missing or ambiguous refreshed declarations clear selection and reset only changed source",async()=>{
  const e={id:"kept",index:0,analysis:0,name:"one",kind:"fn",name_truncated:false,kind_truncated:false,start_line:1,end_line:1};
  for(const replacement of [null,{...e,id:"different"}]) {
    const f=fixture(async(uri,options,response)=>{const b=await response(uri),fresh=b.token==="next";if(fresh){b.origin="pair";if(uri.includes("/files?"))b.selected=row("a.rs","v-base","changed");}if(b.window){const current=fresh?replacement:e;Object.assign(b.window,{outline_total:current?1:0,outline:current?[current]:[],cursor:current});}return b;});
    const s=f.controller.updates();s.receive(change());await flush();await f.controller.navigate({entity:{id:"kept",index:0,start_line:1}});const base={sourceTop:44,outlineTop:0};f.setPositions({base,working:{sourceTop:8800,sourceLeft:99,outlineTop:480,columnLine:1,column:65536}});
    s.receive(change("next","2",{active:{...metadata("next"),origin:"pair"}}));await flush();const v=f.views.at(-1);
    assert.equal(v.metadata.token,"next");assert.equal(v.selection.entity,null);assert.equal(v.selection.hunk,null);assert.equal(v.windows.working.position.sourceTop,0);assert.equal(v.windows.working.position.sourceLeft,0);assert.deepEqual(v.windows.base.position,base);assert.match(f.notices.at(-1),/declaration.*(?:unavailable|available)/);
  }
});

test('creation sends an optional explicit target revision without changing working-tree requests',async()=>{
 const sent=[];const controller=createReviewController({transport:async(uri,options)=>{sent.push(JSON.parse(options.body));}});
 await controller.create({revision:'base',scope:'.',target_revision:'target'});
 assert.deepEqual(sent[0],{provider:'git',revision:'base',scope:'.',target_revision:'target'});
 await controller.create({revision:'HEAD',scope:'.'});assert.deepEqual(sent[1],{provider:'git',revision:'HEAD',scope:'.'});
});
