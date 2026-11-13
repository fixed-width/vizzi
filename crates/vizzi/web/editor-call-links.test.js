import {test} from 'node:test';
import assert from 'node:assert/strict';
import {installEditorCallLinks} from './editor-call-links.js';

function fixture(hash='') {
 const events=new EventTarget();
 const window={location:new URL('http://127.0.0.1:7474/'+hash),addEventListener:events.addEventListener.bind(events)};
 window.history={replaceState(_state,_title,url){window.location=new URL(url,window.location);}};
 const opened=[],errors=[];let available=false;
 const links=installEditorCallLinks({window,ready:()=>available,open:async request=>{opened.push(request);},failed:error=>errors.push(error.message)});
 function navigate(hash){window.location.hash=hash;const event=new Event('hashchange');event.newURL=window.location.href;events.dispatchEvent(event);}
 return {window,opened,errors,navigate,links,available:()=>{available=true;}};
}
const hash='#editor-call='+'b'.repeat(64)+'&snapshot='+'a'.repeat(64);
test('editor links wait for captured source and are consumed for repeated navigation',async()=>{
 const f=fixture(hash);
 assert.equal(f.opened.length,0);
 assert.equal(f.window.location.hash,'');
 f.available();await f.links.resume();
 assert.deepEqual(f.opened,[{snapshot:'a'.repeat(64),call:'b'.repeat(64)}]);
 f.navigate(hash);await f.links.resume();
 assert.equal(f.opened.length,2);
 assert.equal(f.window.location.hash,'');
});
test('malformed links fail without opening source and unrelated fragments are preserved',()=>{
 const f=fixture('#saved');
 assert.equal(f.window.location.hash,'#saved');
 f.navigate('#editor-call=some-name&snapshot=../file');
 assert.equal(f.opened.length,0);
 assert.match(f.errors[0],/Invalid editor call link/);
});
