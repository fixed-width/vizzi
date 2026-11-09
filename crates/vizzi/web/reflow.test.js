import assert from 'node:assert/strict';
import {planReflow} from './reflow.js';
const line=(text,points,leading=0)=>({text,reflow:{leading,points},left:false,right:false});
const p=(offset,priority=0,anchor=null)=>({offset,priority,anchor});
const mono=text=>text.replace(/\t/g,'    ').length;
const text='    process(first_argument, second_argument, third_argument);';
const input=line(text,[p(12,1,12),p(28,0,12),p(45,0,12)],4);
const pieces=planReflow(input,35,mono);
assert.ok(pieces.length>1);assert.equal(pieces.map(s=>text.slice(s.start,s.end)).join(''),text);
assert.ok(pieces.slice(1).every(s=>s.indent>0&&s.indent<=35/2));
assert.deepEqual(planReflow(input,120,mono),[{start:0,end:text.length,indent:0}]);
const utf='🙂 call(first, second);';const bytes=new TextEncoder().encode(utf),offset=new TextEncoder().encode('🙂 call(first, ').length;
const unicode=planReflow(line(utf,[p(offset,0,10)]),17,mono);assert.equal(unicode.length,2);assert.equal(new TextDecoder().decode(bytes.slice(unicode[0].start,unicode[0].end)), '🙂 call(first, ');
for(const malformed of [null,{}, {leading:0,points:[p(2)]}, {leading:0,points:[p(-1)]},{leading:0,points:[p(bytes.length+1)]},{leading:0,points:[p(offset,99)]}])assert.equal(planReflow({...line(utf,[]),reflow:malformed},10,mono).length,1);
assert.equal(planReflow({...input,left:true},10,mono).length,1);
assert.equal(planReflow(input,NaN,mono).length,1);
const literal=line('call("a very long indivisible string", next)',[p(5,1,5),p(38,0,5)]);
assert.ok(planReflow(literal,16,mono).every(s=>s.start<=5||s.start>=38));
console.log('reflow planner: byte-exact partitions, preferred breaks, indentation, widening, Unicode, protected tokens and invalid metadata');
assert.equal(planReflow(line('aaa bbb ccc ddd',[p(4,0),p(8,8),p(12,8)]),10,mono)[0].end,4,'structural break precedes a whitespace fallback');
assert.equal(planReflow(line('aaa bbb ccc',[p(4),p(8)]),9,mono)[0].end,8,'equal priorities use the fuller line');
assert.equal(planReflow(line('f(first_arg, second_arg)',[p(13,0,2)]),17,mono)[1].indent,2);
for(const reflow of [{leading:-1,points:[p(offset)]},{leading:1,points:[p(offset)]},{leading:0,points:[p(offset,0,-1)]},{leading:0,points:[p(offset,0,offset+1)]},{leading:0,points:[p(offset),p(offset)]},{leading:0,points:[null]},{leading:0,points:[p(offset,0,2)]},{leading:0,points:'bad'},{leading:0,points:[p(offset,0,NaN)]}])assert.equal(planReflow({...line(utf,[]),reflow},10,mono).length,1);
for(const width of [0,-1,Infinity])assert.equal(planReflow(input,width,mono).length,1);
for(const measure of [()=>NaN,()=>-1,()=>{throw Error('unavailable');}])assert.equal(planReflow(input,20,measure).length,1);
assert.equal(planReflow(line('a'.repeat(4097),[p(2048)]),2049,mono).length,1);
assert.equal(planReflow(line('a'.repeat(4096),[p(2048)]),2049,mono).length,2);
for(const rows of [64,65]){const input=line('a '.repeat(rows-1)+'z',Array.from({length:rows-1},(_,i)=>p((i+1)*2,0,0)));assert.equal(planReflow(input,2,mono).length,rows===64?64:1);}
const original=input=>[{start:0,end:new TextEncoder().encode(input.text).length,indent:0}];
for(const reflow of [null,{}, {leading:-1,points:[p(offset)]},{leading:0,points:[p(offset),p(bytes.length)]},{leading:0,points:[p(offset,-1)]}]){const value={...line(utf,[]),reflow};assert.deepEqual(planReflow(value,10,mono),original(value));}
for(const leading of [1,2,3]){const value=line('x  call(first, second)',[p(15)],leading);assert.deepEqual(planReflow(value,10,mono),original(value));}
for(const count of [256,257]){const value=line('a '.repeat(count)+'z',Array.from({length:count},(_,i)=>p((i+1)*2,0,0)));assert.equal(planReflow(value,count*2,mono).length,count===256?2:1);}
for(const bad of [NaN,-1])assert.deepEqual(planReflow(input,20,text=>text==='    '?bad:mono(text)),original(input));
const fallback=line('            f(firstarg, secondarg)',[p(14,1,14),p(24,0,14)],12);assert.equal(planReflow(fallback,24,mono)[1].indent,8);
const capped=line('    value = long_processor.run(firstarg, secondarg)',[p(30,1,30),p(40,0,30)],4);assert.equal(planReflow(capped,35,mono)[1].indent,8);
assert.ok(planReflow(line('aaa bbb ccc',[p(4),p(8)]),5,mono).slice(1).every(s=>s.indent>0));
assert.equal(planReflow(line('func_f(a, another_argument)',[p(10,0,7)]),14,mono)[1].indent,7);
assert.deepEqual(planReflow(input,text.length,mono),original(input));
assert.equal(planReflow(line('aaa    bbb ccc ddd',[p(7),p(11,4),p(15,8)]),12,mono)[0].end,11);
assert.equal(planReflow(line('aaa bbb ccc ddd',[p(4),p(8,1),p(12,8)]),10,mono)[0].end,8);
assert.equal(planReflow(line('aa bbb cccccccccc',[p(3),p(7,4)]),8,mono)[0].end,7);
const hanging=line('outerfn('+ 'x'.repeat(11)+', a, bbbbb, ccccccccccccc)',[p(21,0,8),p(24,0,8),p(31,4,8)]);assert.equal(planReflow(hanging,20,mono)[1].end,31);
const splitLiteral=planReflow(literal,16,mono);assert.deepEqual(splitLiteral.map(s=>[s.start,s.end]),[[0,5],[5,38],[38,literal.text.length]]);
const tail=line('call(first,averylongunbreakable_tail)',[p(11,0,5)]);assert.equal(planReflow(tail,12,mono).map(s=>tail.text.slice(s.start,s.end)).join(''),tail.text);
let calls=0;const words=Array.from({length:255},(_,i)=>'w'+i.toString().padStart(3,'0'));const stressful=line(words.join(' ')+' END',words.map((_,i)=>p((i+1)*5,0,0)).filter(point=>point.offset<words.join(' ').length));
planReflow(stressful,100,value=>{calls++;if(!value.trim())return 0;if(value.includes('END'))return 101;return value.trim().includes(' ')?0:99;});assert.ok(calls<=4096,'measurement work stays bounded even for a pathological font provider');
const badEnd={...line(utf,[]),reflow:{leading:0,points:[p(offset),p(bytes.length)]}};assert.deepEqual(planReflow(badEnd,17,mono),original(badEnd));
assert.equal(planReflow(line('aaa bbb ccc ddd',[p(4),p(8),p(12)]),7,mono)[0].end,8);

// Removing a fragment margin increases usable width while source byte partitions stay original.
const shifted=planReflow(input,35,mono,{bytes:4,columns:0});
assert.equal(shifted.map(s=>text.slice(s.start,s.end)).join(''),text);
assert.equal(shifted[1].indent,8,'delimiter alignment uses the displayed column');
assert.deepEqual(planReflow(input,text.length-4,mono,{bytes:4,columns:0}),original(input));
const tabbed=line('\t\tf(firstarg, secondarg)',[p(4,1,4),p(14,0,4)],2);
assert.equal(planReflow(tabbed,18,mono,{bytes:2,columns:2})[1].indent,4);
const topLevel=line('extremely_long_function_name(first_argument, second_argument)',[p(29,1,29),p(45,0,29)]);
assert.equal(planReflow(topLevel,40,mono)[1].indent,4);
const relative=line('        f(first_argument, second_argument)',[p(26,0,0)],8);
assert.equal(planReflow(relative,30,mono,{bytes:8,columns:4})[1].indent,0);
