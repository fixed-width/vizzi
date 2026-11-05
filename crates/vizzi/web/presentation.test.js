import {test} from 'node:test';
import assert from 'node:assert/strict';
import {coverageLabel,fileCount,pageCount,kindGroup} from './presentation.js';
test('coverage is readable without duplicated syntax or assignment claims',()=>{
  assert.equal(coverageLabel('syntax'),'Syntax available');assert.equal(coverageLabel('partial'),'Syntax with recovery');assert.equal(coverageLabel('deferred'),'Analysis deferred');assert.equal(coverageLabel('unavailable'),'Analysis unavailable');assert.equal(coverageLabel('syntax',true),'Source only');
});
test('counts distinguish filters and caps only when necessary',()=>{
 assert.equal(fileCount(12,12,12),'12 files');assert.equal(fileCount(3,3,12),'3 of 12 files');assert.equal(fileCount(2000,3200,4000),'Showing 2,000 of 3,200 matching files');assert.equal(fileCount(2000,3200,3200),'Showing 2,000 of 3,200 files');assert.equal(fileCount(0,0,0),'0 files');
 assert.equal(fileCount(1,1,1),'1 file');assert.equal(pageCount(0,1,1),'1 file');
 assert.equal(pageCount(0,3,3),'3 files');assert.equal(pageCount(200,50,450),'201–250 of 450 files');assert.equal(pageCount(0,0,0),'0 files');
});
test('types, callables and values have distinct stable badge categories',()=>{
 for(const kind of ['struct','enum','class','trait','union','type','type_alias','typedef'])assert.equal(kindGroup(kind),'type');
 assert.equal(kindGroup('type'),'type');assert.equal(kindGroup('type_alias'),'type');assert.equal(kindGroup('typedef'),'type');
 assert.equal(kindGroup('function'),'function');assert.equal(kindGroup('method'),'function');for(const kind of ['constant','const','static','variable','field'])assert.equal(kindGroup(kind),'constant');assert.equal(kindGroup('module'),'module');assert.equal(kindGroup('namespace'),'module');assert.equal(kindGroup('macro'),'function');assert.equal(kindGroup('unknown'),'other');
});
