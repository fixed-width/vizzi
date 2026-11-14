const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const crypto=require('node:crypto');
const {MessageChannel}=require('node:worker_threads');
const {createWorkspaceView}=require('./workspace-view.cjs');
const turn=()=>new Promise(setImmediate);
const id=n=>String(n).repeat(32),native=n=>String(n).repeat(64);
const deferred=()=>{let resolve;const promise=new Promise(r=>{resolve=r;});return {promise,resolve};};
async function fixture(t,{automatic=true,ready=()=>true,prepare}={}) {
 const {createDeliveryPage}=await import('./workspace-delivery.mjs');
 const origin='http://127.0.0.1:7474',nativePackets=[],hostPackets=[],applied=[],panels=[],ports=[];
 let receiver,listener,html,documentNumber=0,page,nativePort,child;
 const created=deferred();
 function hello(){listener({source:child,origin,data:{protocol:'vizzi-delivery-connect',version:1,type:'hello',document:id(documentNumber),mode:'call',transition:null}});}
 child={postMessage(message,target,transfers){
  assert.equal(target,origin);nativePackets.push(message);
  if(message.type==='probe'){if(automatic)queueMicrotask(hello);return;}
  if(message.type==='attach'){
   nativePort=transfers[0];ports.push(nativePort);
   nativePort.onmessage=event=>{nativePackets.push(event.data);page.receive(event.data);};nativePort.start();
   nativePort.postMessage({protocol:'vizzi-delivery-connect',version:1,type:'attached',document:message.document,challenge:message.challenge});page.connect();
  }
 }};
 const iframe={contentWindow:child,addEventListener(){}};
 const vscode={ViewColumn:{Active:-1},window:{createWebviewPanel(){
  const callbacks=[];
  const panel={webview:{onDidReceiveMessage:callback=>{receiver=callback;return {dispose(){}};},
    postMessage:async message=>{hostPackets.push(message);listener({data:message});return true;}},
    onDidDispose:callback=>callbacks.push(callback),reveal(){},dispose(){for(const cb of callbacks)cb();page?.dispose();for(const port of ports)port.close();}};
  Object.defineProperty(panel.webview,'html',{get:()=>html,set:value=>{
   html=value;documentNumber++;
   page=createDeliveryPage({mode:'call',document:id(documentNumber),ready,prepare:prepare|| (async action=>({commit(){applied.push(action);}})),send:message=>nativePort?.postMessage(message)});
   const script=value.match(/<script nonce="[^"]+">([\s\S]*?)<\/script>/)[1];
   vm.runInNewContext(script,{MessageChannel,crypto:crypto.webcrypto,document:{getElementById:()=>iframe},acquireVsCodeApi:()=>({postMessage:message=>receiver(message)}),window:{addEventListener:(_type,callback)=>{listener=callback;}}});
   created.resolve();
  }});panels.push(panel);return panel;
 }}};
 const view=createWorkspaceView(vscode);t.after(()=>view.dispose());
 return {view,created:created.promise,origin,child,nativePackets,hostPackets,applied,panels,hello,dispatch:event=>listener(event),get html(){return html;}};
}
const session={root:'/project',url:'http://127.0.0.1:7474/'};
const action={kind:'call',snapshot:native(1),call:native(2)};
test('the presentation adapter confirms through the real shared protocol and keeps its nonce private',async t=>{
 const order=[];const f=await fixture(t,{prepare:async value=>{order.push('prepare');return {commit(){order.push('commit');}};}});
 assert.equal((await f.view.present(session,action,{validate:async()=>order.push('verify')})).state,'applied');assert.deepEqual(order,['prepare','verify','commit']);
 const nonce=f.hostPackets[0].nonce;assert.ok(nonce);assert.ok(!JSON.stringify(f.nativePackets).includes(nonce));assert.match(f.html,/\/present/);assert.ok(!f.html.includes('editor-call='));
});
test('hello from a sibling or an incorrect origin cannot bind a delivery channel',async t=>{
 const f=await fixture(t,{automatic:false});const request=f.view.present(session,action);await f.created;
 const hello={protocol:'vizzi-delivery-connect',version:1,type:'hello',document:id(1),mode:'call',transition:null};
 f.dispatch({source:{},origin:f.origin,data:hello});f.dispatch({source:f.child,origin:'http://127.0.0.1:9999',data:hello});await turn();assert.equal(f.nativePackets.filter(m=>m.type==='attach').length,0);assert.deepEqual(f.applied,[]);
 f.hello();assert.equal((await request).state,'applied');
});
test('tab closure cancels pending preparation without applying it or stopping another resource',async t=>{
 const gate=deferred(),started=deferred();let committed=0;
 const f=await fixture(t,{prepare:async()=>{started.resolve();await gate.promise;return {commit(){committed++;}};}});const request=f.view.present(session,action);await started.promise;
 f.view.dispose();assert.equal((await request).state,'cancelled');gate.resolve();await turn();assert.equal(committed,0);
});
