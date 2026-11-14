import {test} from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {pathToFileURL} from 'node:url';import {resolve} from 'node:path';import {MessageChannel} from 'node:worker_threads';
const code=(await readFile(new URL('./editor-delivery.js',import.meta.url),'utf8')).replace(/['"]\/workspace-delivery\.mjs['"]/,JSON.stringify(pathToFileURL(resolve('crates/vizzi/web/workspace-delivery.mjs')).href));
const {installWorkspaceDelivery}=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
const id=n=>String(n).repeat(64),turn=()=>new Promise(setImmediate);
function fixture({mode='file',hash='',ready=()=>true,prepare,embedded=false}={}) {
 const events=new EventTarget(),docEvents=new EventTarget(),opened=[],errors=[],marker={reading:'saved'};
 const window={location:new URL('http://127.0.0.1:7474/'+(mode==='file'?'explore':'')+hash),addEventListener:events.addEventListener.bind(events),removeEventListener:events.removeEventListener.bind(events),stop(){}};const announcements=[];window.parent=embedded?{postMessage:message=>announcements.push(message)}:window;
 window.history={state:marker,replaceState(state,_title,path){this.state=state;window.location=new URL(path,window.location);}};
 const document={addEventListener:docEvents.addEventListener.bind(docEvents),removeEventListener:docEvents.removeEventListener.bind(docEvents)};
 const page=installWorkspaceDelivery({window,document,mode,ready,prepare:prepare|| (async(action,options)=>({commit(){opened.push({action,options});}})),failed:error=>errors.push(error.message)});
 return {page,window,opened,errors,marker,announcements,clickLink(path,options={}){const anchor={href:new URL(path,window.location).href,target:options.target||'',hasAttribute:name=>name==='download'&&options.download};const event=new Event('click');Object.defineProperty(event,'target',{value:{closest:()=>anchor}});const {target,download,...modifiers}=options;Object.assign(event,{button:0,ctrlKey:false,metaKey:false,shiftKey:false,altKey:false,...modifiers});docEvents.dispatchEvent(event);},message(data,source,ports=[]){const event=new Event('message');event.data=data;event.source=source;event.ports=ports;events.dispatchEvent(event);},navigate(hash){window.location.hash=hash;const event=new Event('hashchange');event.newURL=window.location.href;events.dispatchEvent(event);}};
}
const file=n=>`#editor-file=${id(n)}&snapshot=${id(1)}&version=${id(2)}&line=131`;
test('legacy file links wait, preserve their history marker and replace the entry on application',async t=>{
 let ready=false;const f=fixture({hash:file(3),ready:()=>ready});t.after(()=>f.page.dispose());assert.equal(f.window.location.hash,'');assert.equal(f.window.history.state,f.marker);assert.equal(f.page.pending(),true);
 ready=true;await f.page.resume();await turn();assert.equal(f.opened.length,1);assert.equal(f.opened[0].options.replaceHistory,true);assert.equal(f.opened[0].action.line,131);assert.equal(f.page.pending(),false);
 f.navigate(file(3));await turn();assert.equal(f.opened.length,2);
});
test('only the newest hidden legacy request applies when readiness returns',async t=>{
 let ready=false;const f=fixture({ready:()=>ready});t.after(()=>f.page.dispose());f.navigate(file(3));f.navigate(file(4));ready=true;await f.page.resume();await turn();assert.deepEqual(f.opened.map(x=>x.action.file),[id(4)]);
});
test('legacy call identity errors refuse without changing unrelated fragments or opening a view',async t=>{
 const f=fixture({mode:'call',hash:'#saved'});t.after(()=>f.page.dispose());assert.equal(f.window.location.hash,'#saved');
 f.navigate('#editor-call=../name&snapshot='+id(1));assert.equal(f.opened.length,0);assert.equal(f.errors.length,1);
});
test('manual navigation cancels queued legacy work and its later readiness cannot pull the reader back',async t=>{
 let ready=false;const f=fixture({hash:file(3),ready:()=>ready});t.after(()=>f.page.dispose());f.page.manual();ready=true;await f.page.resume();await turn();assert.deepEqual(f.opened,[]);assert.deepEqual(f.errors,[]);
});

test('a native document binds a transferred channel only for its parent and current identity',async t=>{
 const f=fixture({mode:'call',embedded:true});t.after(()=>f.page.dispose());const channel=new MessageChannel();t.after(()=>{channel.port1.close();channel.port2.close();});const packets=[];
 channel.port1.onmessage=event=>packets.push(event.data);channel.port1.start();const hello=f.announcements[0];assert.equal(hello.type,'hello');
 const attach={protocol:'vizzi-delivery-connect',version:1,type:'attach',document:hello.document,challenge:'fresh challenge'};
 f.message(attach,{},[channel.port2]);await turn();assert.deepEqual(packets,[]);
 f.message({...attach,document:'f'.repeat(32)},f.window.parent,[channel.port2]);await turn();assert.deepEqual(packets,[]);
 f.message(attach,f.window.parent,[channel.port2]);await turn();assert.equal(packets[0].type,'attached');assert.equal(packets[0].challenge,attach.challenge);assert.equal(packets[1].type,'connected');
 const {createDeliveryClient}=await import('./workspace-delivery.mjs');const client=createDeliveryClient({send:message=>channel.port1.postMessage(message)});t.after(()=>client.dispose());channel.port1.onmessage=event=>client.receive(event.data);client.receive(packets[1]);
 const outcome=await client.deliver({kind:'call',snapshot:id(1),call:id(2)});assert.equal(outcome.state,'applied');assert.equal(f.opened.length,1);
});
test('a standalone opaque mode-continuation URL cannot strand the ordinary source browser',async t=>{
 const f=fixture({hash:'#vizzi-delivery='+'a'.repeat(32)});t.after(()=>f.page.dispose());assert.equal(f.window.location.hash,'');assert.equal(f.page.pending(),false);assert.equal(f.opened.length,0);
});

test('all same-tab view-mode links cancel queued editor navigation',async t=>{
 for(const path of ['/','/workspace','/explore','/review','/investigate#saved']){let ready=false;const f=fixture({hash:file(3),ready:()=>ready});t.after(()=>f.page.dispose());f.clickLink(path);ready=true;await f.page.resume();await turn();assert.deepEqual(f.opened,[],path);}
});
test('links opened elsewhere and modifier clicks preserve the pending action in this view',async t=>{
 for(const options of [{button:1},{ctrlKey:true},{metaKey:true},{shiftKey:true},{altKey:true},{target:'_blank'},{download:true}]){let ready=false;const f=fixture({hash:file(3),ready:()=>ready});t.after(()=>f.page.dispose());f.clickLink('/review',options);ready=true;await f.page.resume();await turn();assert.equal(f.opened.length,1,JSON.stringify(options));}
 let ready=false;const f=fixture({hash:file(3),ready:()=>ready});t.after(()=>f.page.dispose());f.clickLink('https://external.invalid/review');ready=true;await f.page.resume();await turn();assert.equal(f.opened.length,1);
});
test('valid legacy calls use native identities and are consumed without losing the query or extra fragment data',async t=>{
 const f=fixture({mode:'call',hash:`?ignored`});t.after(()=>f.page.dispose());f.window.location=new URL(`http://127.0.0.1:7474/?demo=kept#editor-call=${id(3)}&snapshot=${id(1)}&reading=kept`);
 f.navigate(f.window.location.hash);await turn();assert.equal(f.opened.length,1);assert.deepEqual(f.opened[0].action,{kind:'call',call:id(3),snapshot:id(1)});assert.equal(f.window.location.search,'?demo=kept');assert.equal(f.window.location.hash,'#reading=kept');
});
test('disposed page adapters cannot consume another legacy action',async()=>{
 const f=fixture();f.page.dispose();f.navigate(file(3));await turn();assert.deepEqual(f.opened,[]);assert.deepEqual(f.errors,[]);
});
