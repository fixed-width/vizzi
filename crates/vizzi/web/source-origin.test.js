import {test} from 'node:test';
import assert from 'node:assert/strict';
import {validOrigin,captureFor,originLabel} from './source-origin.js';
const origin={id:'a'.repeat(64),capture:'b'.repeat(64),label:'helper',version:'1.2.3',language:'rust'};
test('dependency origins bind separate captures and give readable provenance',()=>{
 assert.equal(validOrigin(null),true);assert.equal(validOrigin(undefined),true);assert.equal(validOrigin(origin),true);assert.equal(originLabel(origin),'Dependency · helper 1.2.3');assert.equal(originLabel({...origin,version:null}),'Dependency · helper');assert.equal(captureFor({origin},'c'.repeat(64)),origin.capture);assert.equal(captureFor({},'c'.repeat(64)),'c'.repeat(64));
});
test('saved origin metadata rejects malformed tokens and display data',()=>{
 for(const o of [{},false,{...origin,id:'../file'},{...origin,capture:[]},{...origin,label:''},{...origin,label:'x'.repeat(257)},{...origin,version:3},{...origin,version:'x'.repeat(129)},{...origin,language:''},{...origin,language:'x'.repeat(65)}])assert.equal(validOrigin(o),false);
});
test('origin byte bounds and token delimiters survive persistence without coercion',()=>{
 assert.equal(validOrigin({...origin,label:'é'.repeat(128),language:'x'.repeat(64),version:'v'.repeat(128)}),true);assert.equal(originLabel(null),'');
 for(const bad of [{...origin,id:[origin.id]},{...origin,id:'0'+origin.id},{...origin,capture:origin.capture+'0'},{...origin,label:['helper']},{...origin,label:'bad\nlabel'},{...origin,label:'é'.repeat(129)},{...origin,language:'bad\0language'}])assert.equal(validOrigin(bad),false);
});
