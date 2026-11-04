import test from "node:test";
import assert from "node:assert/strict";
import {createReviewWindowReader,validateReviewWindow} from "./review-windows.js";
const view=(side="base",token="pair")=>({review:{token,comparison:"comparison",side},source:{id:"file",version:"version",summary:{lines:1,declarations:0}}});
const response=(side="base",token="pair")=>({schema:1,token,comparison:"comparison",side,window:{schema:1,file:"file",version:"version",line_total:1,line_start:1,lines:[{number:1,text:"text",column:0,end_column:4,line_bytes:4,left:false,right:false}],outline_total:0,outline_start:0,outline:[],cursor:null}});
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{resolve,reject,promise};};
test("review windows require pair, side and document anchors before shared window validation",()=>{
  assert.equal(validateReviewWindow(view(),response()).lines[0].text,"text");
  for(const patch of [{schema:2},{token:"other"},{comparison:"other"},{side:"working"},{window:null},{window:{...response().window,version:"other"}},{window:{...response().window,lines:[{...response().window.lines[0],text:"wrong"}]}}])assert.throws(()=>validateReviewWindow(view(),{...response(),...patch}),/window|fragment/);
});
test("scrolling uses only review source and obsolete panes cannot commit",async()=>{
  const requests=[],windows=[],errors=[];
  const reader=createReviewWindowReader({transport:(uri,options)=>{const d=deferred();requests.push({...d,uri,options});return d.promise;},onWindow:w=>windows.push(w),onError:e=>errors.push(e)});
  reader.activate(view());const first=reader.request({line_start:1,previous_snapshot:"live",snapshot:"live",cursor:null,column_line:undefined});
  const url=new URL(requests[0].uri,"http://local");assert.equal(url.pathname,"/api/comparisons/pair/window");assert.equal(url.searchParams.get("side"),"base");assert.equal(url.searchParams.get("version"),"version");assert.ok(!url.searchParams.has("snapshot"));assert.ok(!url.searchParams.has("previous_snapshot"));
  assert.ok(!url.searchParams.has("cursor"));assert.ok(!url.searchParams.has("column_line"));
  reader.activate(view("working","next"));const second=reader.request({line_start:1});
  assert.ok(requests[0].options.signal.aborted);requests[1].resolve(response("working","next"));assert.equal(await second,true);
  requests[0].resolve(response());assert.equal(await first,false);assert.equal(windows.length,1);assert.equal(errors.length,0);
  const failed=reader.request({});requests[2].reject(new Error("current"));assert.equal(await failed,false);assert.equal(errors[0].message,"current");
  const old=reader.request({});const new_=reader.request({});requests[3].reject(new Error("old"));assert.equal(await old,false);reader.activate(view());assert.ok(requests[4].options.signal.aborted);requests[4].resolve(response("working","next"));assert.equal(await new_,false);assert.equal(errors.length,1);
  assert.equal(await reader.prepare({source:null},{}),null);
});

test("completed review requests release their abort handles, and null envelopes fail coherently",async()=>{
  const requests=[];const reader=createReviewWindowReader({transport:async(uri,options)=>{requests.push(options);return response();}});
  reader.activate(view());assert.equal(await reader.request({}),true);reader.activate(view());assert.equal(requests[0].signal.aborted,false);
  assert.throws(()=>validateReviewWindow(view(),null),/review window/);
});
