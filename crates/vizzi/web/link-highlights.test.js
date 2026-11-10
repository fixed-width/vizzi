import {test} from 'node:test';
import assert from 'node:assert/strict';
import {linkHighlights} from './link-highlights.js';
const source={file:'file',version:'v1',context:'rust',path:'lib.rs',line:1,end_line:100};
const views=[{id:'caller',symbol:{source}},{id:'child',symbol:{source:{...source,line:110,end_line:120}}},{id:'other',symbol:{source:{...source,file:'other',path:'other.rs'}}}];
const a={id:'a',from:'caller',to:'child',source:{...source,line:10,end_line:13}},b={id:'b',from:'other',to:'child',source:{...views[2].symbol.source,line:20,end_line:20}};
test('bubble hover identifies every incoming call range and arrow',()=>{
 const h=linkHighlights(views,[a,b],[{view:'child'}]);assert.deepEqual([...h.nodes],['child']);assert.deepEqual([...h.links],['a','b']);assert.deepEqual(h.lines,[{view:'caller',start:10,end:13},{view:'other',start:20,end:20}]);
});
test('any physical line in a multi-line call highlights its opened destination and arrow',()=>{
 for(const line of [10,11,12,13]){const h=linkHighlights(views,[a,b],[{view:'caller',line}]);assert.deepEqual([...h.nodes],['child']);assert.deepEqual([...h.links],['a']);}
 for(const line of [9,14,NaN])assert.equal(linkHighlights(views,[a,b],[{view:'caller',line}]).links.size,0);
});
test('historical arrows retain association without painting different captured source',()=>{
 for(const change of [{version:'v2'},{file:'new'},{context:'python'},{path:'other.rs'},{line:11},{end_line:12}]){const changed=[{...views[0],symbol:{source:{...source,...change}}},...views.slice(1)];const h=linkHighlights(changed,[a],[{view:'child'}]);assert.equal(h.links.has('a'),true);assert.deepEqual(h.lines,[]);assert.equal(linkHighlights(changed,[a],[{view:'caller',line:11}]).links.size,0);}
});
test('arrow hover identifies both ends, while absent views and cleared intent leave no highlights',()=>{
 const h=linkHighlights(views,[a,b],[{link:'b'}]);assert.deepEqual(new Set(h.nodes),new Set(['other','child']));assert.deepEqual([...h.links],['b']);assert.deepEqual(h.lines,[{view:'other',start:20,end:20}]);assert.equal(linkHighlights(views,[a],[]).links.size,0);assert.equal(linkHighlights(views.slice(1),[a],[{view:'child'}]).links.size,0);assert.equal(linkHighlights(views,[a],[null]).links.size,0);
});
test('one line may lead to multiple opened destinations and recursion stays bounded',()=>{
 const branch={...a,id:'branch',to:'other'},recursive={...a,id:'recursive',to:'caller'};const h=linkHighlights(views,[a,branch,recursive],[{view:'caller',line:11}]);assert.deepEqual(new Set(h.nodes),new Set(['child','other','caller']));assert.equal(h.links.size,3);assert.equal(h.lines.length,3);
});
test('single-line function boundaries are included, but inverted call ranges cannot colour code',()=>{
 const single=[{...views[0],symbol:{source:{...source,line:1,end_line:1}}},...views.slice(1)],edge={...a,source:{...source,line:1,end_line:1}};
 assert.deepEqual(linkHighlights(single,[edge],[{view:'child'}]).lines,[{view:'caller',start:1,end:1}]);assert.equal(linkHighlights(single,[edge],[{view:'caller',line:1}]).nodes.has('child'),true);
 const invalid={...edge,source:{...edge.source,end_line:0}};assert.deepEqual(linkHighlights(single,[invalid],[{view:'child'}]).lines,[]);assert.equal(linkHighlights(single,[invalid],[{view:'caller',line:1}]).links.size,0);
});
