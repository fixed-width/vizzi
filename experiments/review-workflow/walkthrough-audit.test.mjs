import test from "node:test";
import assert from "node:assert/strict";
import {auditResponse,auditCheckpoint} from "./walkthrough-audit.js";

const metadata={token:"pair",comparison:"comparison"};
const file={...metadata,file:"file",path:"a.rs",base_version:"base-version",working_version:"working-version"};
const row={number:1,column:0,end_column:4,line_bytes:4,text:"text",left:false,right:false};
const window=(side="base")=>({schema:1,...metadata,side,window:{schema:1,file:"file",version:file[`${side}_version`],lines:[row],outline:[]}});
const state=()=>({...metadata,path:"a.rs",side:"working",selectedSides:["working"],fileButtons:1,hunkButtons:1,panes:{base:{version:"base-version",renderedRows:1,outlineButtons:1,rows:[{number:1,text:"text"}]},working:{version:"working-version",renderedRows:1,outlineButtons:1,rows:[{number:1,text:"text"}]}}});
const sources={base:Buffer.from("text\r\n"),working:Buffer.from("text\n")};

test("response audits require explicit pair and side pins and bounded projection payloads",()=>{
  const result=auditResponse("/api/comparisons/pair/window?side=base&file=file&version=base-version",window());
  assert.equal(result.kind,"window");assert.equal(result.sourceBytes,4);assert.equal(result.rows,1);
});

test("visible rows are independently checked against base/working bytes and their own response anchors",()=>{
  const result=auditCheckpoint(state(),file,[window(),window("working")],sources);
  assert.equal(result.verifiedRows,2);assert.equal(result.maxRenderedRows,1);
});

test("response auditing refuses mixed identities, invalid fragments and exceeded bounds",()=>{
  const uri="/api/comparisons/pair/window?side=base&file=file&version=base-version";
  for(const patch of [{schema:2},{token:"other"},{comparison:""},{side:"working"},{side:"unknown"},{window:{...window().window,file:"other"}},{window:{...window().window,version:"other"}},{window:{...window().window,schema:2}}])assert.throws(()=>auditResponse(uri,{...window(),...patch}));
  for(const patch of [{number:0},{column:1.5},{column:5},{end_column:5},{line_bytes:3},{text:"wrong"},{left:true},{right:true}])assert.throws(()=>auditResponse(uri,{...window(),window:{...window().window,lines:[{...row,...patch}]}}));
  for(const patch of [{lines:Array(97).fill(row)},{outline:Array(201).fill({})},{lines:[{...row,text:"x".repeat(32769),end_column:32769,line_bytes:32769}]}])assert.throws(()=>auditResponse(uri,{...window(),window:{...window().window,...patch}}));
  assert.equal(auditResponse("/api/comparisons/events",{}),null);
});

test("paging bounds accept the exact allowance on each projection",()=>{
  const files={schema:1,...metadata,files:Array(200).fill(file)};assert.equal(auditResponse("/api/comparisons/pair/files",files).rows,200);assert.throws(()=>auditResponse("/api/comparisons/pair/files",{...files,files:Array(201).fill(file)}));
  const diff={schema:1,...metadata,file:"file",base_version:null,working_version:"working-version",hunks:Array(200).fill({}),edits:Array(200).fill({})},uri="/api/comparisons/pair/diff?file=file&working_version=working-version";
  assert.equal(auditResponse(uri,diff).edits,200);
  for(const patch of [{base_version:"invented"},{working_version:"wrong"},{hunks:Array(201).fill({})},{edits:Array(201).fill({})}])assert.throws(()=>auditResponse(uri,{...diff,...patch}));
  const body={...window(),window:{...window().window,lines:Array(96).fill({...row,text:"",end_column:0,line_bytes:0}),outline:Array(200).fill({})}};
  assert.equal(auditResponse("/api/comparisons/pair/window?side=base&file=file&version=base-version",body).rows,96);
});

test("an off-page selected row has its own identity and metadata bounds",()=>{
  const uri="/api/comparisons/pair/files?file=file",page={schema:1,...metadata,files:[],selected:file};
  assert.equal(auditResponse(uri,page).selectedRows,1);
  assert.equal(auditResponse(uri,{...page,selected:null}).selectedRows,0);
  for(const selected of [{...file,path:"x".repeat(512),base_version:null,working_version:"x".repeat(256)},{...file,path:"🙂".repeat(128),working_version:null}])assert.equal(auditResponse(uri,{...page,selected}).selectedRows,1);
  for(const selected of [undefined,[],{...file,file:"wrong"},{...file,path:"x".repeat(513)},{...file,path:"🙂".repeat(129)},{...file,working_version:1},{...file,working_version:{length:3}},{...file,base_version:""},{...file,base_version:"x".repeat(257)}])assert.throws(()=>auditResponse(uri,{...page,selected}));
});

test("DOM auditing rejects wrong paths, side versions, rows, double selection and excessive elements",()=>{
  for(const patch of [{token:"wrong"},{comparison:"wrong"},{path:"wrong"},{side:"wrong"},{selectedSides:["base","working"]},{fileButtons:201},{hunkButtons:201}])assert.throws(()=>auditCheckpoint({...state(),...patch},file,[window(),window("working")],sources));
  for(const patch of [{version:"wrong"},{renderedRows:97},{outlineButtons:201},{renderedRows:0},{rows:[{number:2,text:"text"}]},{rows:[{number:1,text:"wrong"}]}]){const s=state();Object.assign(s.panes.working,patch);assert.throws(()=>auditCheckpoint(s,file,[window(),window("working")],sources));}
  const s=state();s.panes.working.version="base-version";assert.throws(()=>auditCheckpoint(s,file,[window(),window("working")],sources));
});

test("absent base is distinct from empty text and Unicode fragments verify exact bytes",()=>{
  const s=state();s.panes.base={version:"",renderedRows:0,outlineButtons:0,rows:[]};s.panes.working.rows=[{number:1,text:"…🙂…"}];
  const body=window("working");body.window.lines=[{number:1,column:1,end_column:5,line_bytes:6,text:"🙂",left:true,right:true}];
  const result=auditCheckpoint(s,{...file,base_version:null},[body],{working:Buffer.from("a🙂z\n")});assert.equal(result.verifiedRows,1);
  const empty=state();for(const side of ["base","working"])empty.panes[side].rows=[{number:1,text:""}];
  const blank=side=>({...window(side),window:{...window(side).window,lines:[{...row,text:"",line_bytes:0,end_column:0}]}});
  assert.equal(auditCheckpoint(empty,file,[blank("base"),blank("working")],{base:Buffer.from(""),working:Buffer.from("")}).verifiedRows,2);
});

test("projection scope is exact and percent-encoded tokens keep their native identity",()=>{
  const files={schema:1,...metadata,files:[]};
  for(const uri of ["/other/api/comparisons/pair/files","/api/comparisons/pair/files-extra"])assert.equal(auditResponse(uri,files),null);
  const body={...window(),token:"pair:1"};assert.equal(auditResponse("/api/comparisons/pair%3A1/window?side=base&file=file&version=base-version",body).token,"pair:1");
  for(const comparison of [[],["comparison"],0,null])assert.throws(()=>auditResponse("/api/comparisons/pair/files",{...files,comparison}));
});

test("fractional and unsafe coordinates cannot pass otherwise coherent fragment checks",()=>{
  const uri="/api/comparisons/pair/window?side=base&file=file&version=base-version";
  for(const patch of [{number:0.5},{number:Number.MAX_SAFE_INTEGER+1},{column:0.5,end_column:4.5,line_bytes:5,left:true,right:true}])assert.throws(()=>auditResponse(uri,{...window(),window:{...window().window,lines:[{...row,...patch}]}}));
  const unicode={...window(),window:{...window().window,lines:[{...row,column:1,end_column:5,line_bytes:6,text:"🙂",left:true,right:true}]}};
  assert.equal(auditResponse(uri,unicode).sourceBytes,4);
  const exact={...window(),window:{...window().window,lines:[{...row,text:"x".repeat(32768),line_bytes:32768,end_column:32768}]}};
  assert.equal(auditResponse(uri,exact).sourceBytes,32768);
});

test("DOM limits accept their exact bound and report the largest outline rather than the smallest",()=>{
  const s=state();s.fileButtons=200;s.hunkButtons=200;s.panes.base.renderedRows=96;s.panes.base.outlineButtons=10;s.panes.working.renderedRows=96;s.panes.working.outlineButtons=200;
  assert.deepEqual(auditCheckpoint(s,file,[window(),window("working")],sources),{verifiedRows:2,maxRenderedRows:96,maxOutlineButtons:200});
  for(const key of ["fileButtons","hunkButtons"])for(const value of [-1,0.5,"1"])assert.throws(()=>auditCheckpoint({...state(),[key]:value},file,[window(),window("working")],sources));
  for(const key of ["renderedRows","outlineButtons"])for(const value of [-1,0.5,"1"]){const t=state();t.panes.working[key]=value;assert.throws(()=>auditCheckpoint(t,file,[window(),window("working")],sources));}
});

test("response matching checks each anchor independently, even when both document versions are equal",()=>{
  for(const patch of [{token:"other"},{comparison:"other"},{side:"unknown"},{window:{...window("working").window,file:"other"}},{window:{...window("working").window,version:"other"}}])assert.throws(()=>auditCheckpoint(state(),file,[window(),{...window("working"),...patch}],sources));
  const s=state(),same={...file,working_version:file.base_version};s.panes.working.version=file.base_version;
  assert.throws(()=>auditCheckpoint(s,same,[window(),window()],sources));
  for(const key of ["token","comparison"])assert.throws(()=>auditCheckpoint({...state(),[key]:""},file,[window(),window("working")],sources));
  const literal=state();for(const side of ["base","working"])literal.panes[side]={version:"",renderedRows:0,outlineButtons:0,rows:[]};
  const noDocuments={...file,base_version:null,working_version:null};
  assert.equal(auditCheckpoint(literal,noDocuments,[],{}).verifiedRows,0);
  for(const key of ["token","comparison"])assert.throws(()=>auditCheckpoint({...literal,[key]:""},noDocuments,[],{}));
  for(const key of ["token","comparison"])assert.throws(()=>auditCheckpoint({...literal,[key]:"other"},noDocuments,[],{}));
});

test("source audits reject missing windows and forged fragment lengths even with matching visible text",()=>{
  const s=state();s.panes.working.rows=[];assert.throws(()=>auditCheckpoint(s,file,[window()],sources));
  const forged=window("working");forged.window.lines=[{...row,line_bytes:5,right:true}];const t=state();t.panes.working.rows[0].text="text…";
  assert.throws(()=>auditCheckpoint(t,file,[window(),forged],sources));
  const repeat=state();repeat.panes.working.rows=[{number:2,text:"text"}];assert.throws(()=>auditCheckpoint(repeat,file,[window(),window("working")],{...sources,working:Buffer.from("text\ntext")}));
});

test("invalid UTF-8 never becomes replacement text, and embedded CR remains source content",()=>{
  const s=state();s.panes.working.rows=[{number:1,text:"�"}];const body=window("working");body.window.lines=[{...row,text:"�",line_bytes:3,end_column:3}];
  assert.throws(()=>auditCheckpoint(s,file,[window(),body],{...sources,working:Buffer.from([255])}));
  const t=state();t.panes.working.rows=[{number:1,text:"te\rxt"}];const internal=window("working");internal.window.lines=[{...row,text:"te\rxt",line_bytes:5,end_column:5}];
  assert.equal(auditCheckpoint(t,file,[window(),internal],{...sources,working:Buffer.from("te\rxt\n")}).verifiedRows,2);
});

test('style audits enforce UTF-8 ranges, lexical vocabulary and the whole-window span allowance',()=>{
 const uri='/api/comparisons/pair/window?side=base&file=file&version=base-version';
 const styled=()=>{const w=structuredClone(window());w.window.coloring='ready';w.window.lines=[{...row,text:'🙂ab',end_column:6,line_bytes:6,styles:[{start:0,end:4,kind:'string'},{start:4,end:6,kind:'keyword'}]}];return w;};
 assert.equal(auditResponse(uri,styled()).styleSpans,2);
 for(const change of [w=>w.window.lines[0].styles[0].end=3,w=>w.window.lines[0].styles[0].kind='unknown',w=>w.window.lines[0].styles[1].start=0,w=>w.window.lines[0].styles=Array(1025).fill({start:0,end:4,kind:'string'})]){const w=styled();change(w);assert.throws(()=>auditResponse(uri,w));}
});
