import {test} from 'node:test';
import assert from 'node:assert/strict';
import {installEditorFileLinks} from './editor-file-links.js';
const id=n=>String(n).repeat(64);
const hash=`#editor-file=${id(2)}&snapshot=${id(1)}&version=${id(3)}&line=131`;
function fixture(initial=hash) {
  const events=new EventTarget(),opened=[],errors=[],marker={reading:'kept'};
  const window={location:new URL('http://127.0.0.1:7474/explore'+initial),addEventListener:events.addEventListener.bind(events),
    history:{state:marker,replaceState(state,_title,url){this.state=state;window.location=new URL(url,window.location);}}};
  let ready=false;
  const links=installEditorFileLinks({window,ready:()=>ready,open:async request=>{opened.push(request);},failed:error=>errors.push(error.message)});
  const navigate=fragment=>{window.location.hash=fragment;const event=new Event('hashchange');event.newURL=window.location.href;events.dispatchEvent(event);};
  return {window,marker,links,opened,errors,navigate,available(){ready=true;}};
}
test('file links wait for a visible source browser, preserve history markers and support repeats',async()=>{
  const f=fixture();assert.equal(f.opened.length,0);assert.equal(f.window.location.hash,'');assert.equal(f.window.history.state,f.marker);
  assert.equal(f.links.pending(),true);
  f.available();await f.links.resume();
  assert.deepEqual(f.opened,[{snapshot:id(1),file:id(2),version:id(3),line:131}]);
  assert.equal(f.links.pending(),false);
  f.navigate(hash);await f.links.resume();assert.equal(f.opened.length,2);
});
test('malformed identifiers/lines and oversized queues never open substituted source',()=>{
  const f=fixture('#saved');assert.equal(f.window.location.hash,'#saved');
  for(const fragment of [hash.replace(id(2),'../a.py'),hash.replace('line=131','line=0'),hash.replace('line=131','line=1.5'),hash.replace('line=131','line=9007199254740992')])f.navigate(fragment);
  assert.equal(f.opened.length,0);assert.equal(f.errors.length,4);
  for(let i=0;i<17;i++)f.navigate(hash);
  assert.match(f.errors.at(-1),/Too many/);
});
