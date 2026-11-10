import {test} from 'node:test';import assert from 'node:assert/strict';
import {moduleContours} from './module-contours.js';
test('bounded native module groups yield contours without changing view geometry',()=>{
 const views=[{module:'a',x:0,y:0,w:440,h:400},{module:'a',x:600,y:0,w:440,h:400},{module:'b',x:0,y:500,w:440,h:400},{module:'b',x:600,y:500,w:440,h:400}],saved=structuredClone(views),paths=moduleContours(views);assert.equal(paths.length,2);for(const p of paths){assert.match(p.path,/^M/);assert.ok(p.path.length<=32768);assert.ok(!/NaN|Infinity/.test(p.path));}assert.deepEqual(views,saved);
});
test('unknown, singleton, overlapping or excessively spread sets do not create misleading regions',()=>{
 assert.deepEqual(moduleContours([]),[]);assert.deepEqual(moduleContours([{module:null,x:0,y:0,w:100,h:100}]),[]);assert.deepEqual(moduleContours([{module:'a',x:0,y:0,w:100,h:100},{module:'b',x:200,y:0,w:100,h:100}]),[]);
 assert.deepEqual(moduleContours([{module:'a',x:0,y:0,w:100,h:100},{module:'a',x:200,y:0,w:100,h:100},{module:'b',x:10,y:10,w:100,h:100}]),[]);
 assert.deepEqual(moduleContours([{module:'a',x:-100000,y:0,w:320,h:200},{module:'a',x:100000,y:0,w:320,h:200}]),[]);
 assert.deepEqual(moduleContours(Array(25).fill({module:'a',x:0,y:0,w:100,h:100})),[]);assert.deepEqual(moduleContours([{module:'a',x:NaN,y:0,w:100,h:100}]),[]);
});
test('two-member groups and the full 24-bubble allowance remain usable with unknown obstacles',()=>{
 const group=[{module:'a',x:0,y:0,w:200,h:100},{module:'a',x:600,y:0,w:200,h:100}],obstacles=Array.from({length:22},(_,i)=>({module:null,x:1000+(i%6)*220,y:500+Math.floor(i/6)*120,w:100,h:80}));
 assert.deepEqual(moduleContours(group).map(p=>p.id),['a']);assert.deepEqual(moduleContours([...group,...obstacles]).map(p=>p.id),['a']);assert.deepEqual(moduleContours([...group,...obstacles,obstacles[0]]),[]);
 for(const [property,value] of [['w',0],['h',0],['w',-1],['h',-1],['x',Infinity],['y',-Infinity],['w',NaN],['h',NaN]])assert.deepEqual(moduleContours([{...group[0],[property]:value},group[1]]),[]);
});
test('corner overlap suppresses a contour while separation or edge contact permits one',()=>{
 const group=[{module:'a',x:0,y:0,w:100,h:100},{module:'a',x:300,y:0,w:100,h:100}];
 assert.deepEqual(moduleContours([...group,{module:null,x:90,y:90,w:100,h:100}]),[]);
 for(const obstacle of [{x:90,y:300},{x:-200,y:90},{x:100,y:0},{x:0,y:100},{x:-50,y:0},{x:0,y:-50}])assert.deepEqual(moduleContours([...group,{module:null,...obstacle,w:50,h:50}]).map(p=>p.id),['a']);
});
test('invalid obstacles invalidate the batch and one undersized member suppresses its set',()=>{
 const group=[{module:'a',x:0,y:0,w:100,h:100},{module:'a',x:300,y:0,w:100,h:100}];for(const field of ['w','h'])for(const n of [0,-1])assert.deepEqual(moduleContours([...group,{module:null,x:100,y:300,w:50,h:50,[field]:n}]),[]);
 for(const field of ['w','h'])for(const n of [5,6]){const a={module:'a',x:0,y:0,w:100,h:100,[field]:n};assert.equal(moduleContours([a,group[1]]).length,n===6?1:0);}
});
