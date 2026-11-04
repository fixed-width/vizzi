import test from "node:test";
import assert from "node:assert/strict";
import {createStartup,readiness} from "./startup.js";

const status=patch=>({schema:1,snapshot:null,pending:false,error:null,...patch});
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{promise,resolve,reject};};
const flush=async()=>{for(let i=0;i<8;i++)await Promise.resolve();};
test("readiness distinguishes unstarted, initializing, failed and complete empty observations",()=>{
  assert.equal(readiness(status()),"not_started");assert.equal(readiness(status({pending:true,error:"previous failure"})),"initializing");assert.equal(readiness(status({error:"failed"})),"failed");assert.equal(readiness(status({snapshot:"empty",pending:true,error:"refresh failure"})),"ready");
  assert.equal(readiness(status({readiness:"not_started"})),"not_started");
  for(const value of ["unknown",null,1,"ready"])assert.throws(()=>readiness(status({readiness:value})),/readiness/);
});
test("exploration activates once per host stream, with JSON and single-flight retry",async()=>{
  const requests=[],states=[],errors=[];
  const s=createStartup({transport:(uri,options)=>{const d=deferred();requests.push({...d,uri,options});return d.promise;},onState:v=>states.push(v),onError:e=>errors.push(e)});
  s.receive(status(),"one");s.receive(status(),"one");assert.equal(requests.length,1);assert.equal(requests[0].uri,"/api/workspace/start");assert.equal(requests[0].options.method,"POST");assert.equal(requests[0].options.headers["Content-Type"],"application/json");assert.equal(requests[0].options.body,"{}");assert.equal(states.at(-1).busy,true);
  requests[0].reject(new Error("not connected"));await flush();assert.equal(errors[0].message,"not connected");assert.equal(states.at(-1).action,"Start indexing");
  s.receive(status(),"one");assert.equal(requests.length,1);
  const retry=s.activate();s.activate();assert.equal(requests.length,2);requests[1].resolve({});assert.equal(await retry,true);assert.equal(states.at(-1).busy,false);
  s.receive(status({pending:true,readiness:"initializing"}),"one");assert.equal(states.at(-1).label,"Indexing workspace…");assert.equal(states.at(-1).action,null);
  s.receive(status({error:"failed",readiness:"failed"}),"one");assert.equal(states.at(-1).action,"Retry indexing");
});
test("ready applications need no activation, and restarted streams supersede old requests",async()=>{
  const requests=[],errors=[],states=[];
  const s=createStartup({transport:(uri,options)=>{const d=deferred();requests.push({...d,options});return d.promise;},onError:e=>errors.push(e),onState:v=>states.push(v)});
  s.receive(status({snapshot:"ready",readiness:"ready"}),"one");assert.equal(requests.length,0);assert.equal(states.at(-1).initial,false);assert.equal(states.at(-1).action,null);
  s.receive(status(),"two");assert.equal(requests.length,1);s.receive(status(),"three");assert.equal(requests.length,2);assert.equal(requests[0].options.signal.aborted,true);
  requests[0].reject(new Error("old"));requests[1].resolve({});await flush();assert.equal(errors.length,0);assert.equal(states.at(-1).busy,false);
});
test("initial states expose useful messages and every activation immediately publishes busy state",async()=>{
  const requests=[],states=[];
  const s=createStartup({transport:()=>{const d=deferred();requests.push(d);return d.promise;},onState:v=>states.push(v)});
  s.receive(status());
  assert.deepEqual(states.at(-1),{initial:true,busy:true,label:"Workspace index not started",action:"Start indexing"});
  requests[0].resolve({});await flush();
  s.receive(status({pending:true}));assert.deepEqual(states.at(-1),{initial:true,busy:false,label:"Indexing workspace…",action:null});
  s.receive(status({error:"failed"}));assert.deepEqual(states.at(-1),{initial:true,busy:false,label:"Initial indexing failed",action:"Retry indexing"});
  s.receive(status({snapshot:"ready"}));assert.deepEqual(states.at(-1),{initial:false,busy:false,label:null,action:null});
  s.receive(status());assert.equal(requests.length,1,"repeated legacy notifications do not reactivate");
});
test("manual activation before a notification is safe, and late successful requests cannot finish a new stream",async()=>{
  const requests=[],states=[],errors=[];
  const s=createStartup({transport:(uri,options)=>{const d=deferred();requests.push({...d,options});return d.promise;},onState:v=>states.push(v),onError:e=>errors.push(e)});
  const early=s.activate();assert.equal(states.length,0);
  s.receive(status(),"one");assert.equal(requests.length,2);
  requests[0].resolve({});assert.equal(await early,false);assert.equal(states.at(-1).busy,true);
  const current=s.activate();requests[1].reject(new Error("current failure"));assert.equal(await current,false);assert.equal(errors.at(-1).message,"current failure");assert.equal(states.at(-1).busy,false);
});
test("invalid status does not cancel an in-flight activation on another stream",async()=>{
  const requests=[];
  const s=createStartup({transport:(uri,options)=>{const d=deferred();requests.push({...d,options});return d.promise;}});
  s.receive(status(),"one");
  assert.throws(()=>s.receive(status({readiness:"ready"}),"two"),/readiness/);
  assert.equal(requests[0].options.signal.aborted,false);
  s.receive(status(),"one");assert.equal(requests.length,1);
  requests[0].resolve({});await flush();
});
