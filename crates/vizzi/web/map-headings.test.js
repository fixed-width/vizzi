import {test} from 'node:test';import assert from 'node:assert/strict';import {headingPlacement} from './map-headings.js';
test('headings fit completely in their header band at every zoom or are omitted',()=>{
 for(const scale of [.03,.1,.25,.45,.55,.75,1,1.5,2,4])for(const height of [8,12,18,30])for(const metrics of [{actualBoundingBoxAscent:8,actualBoundingBoxDescent:3},{actualBoundingBoxAscent:13,actualBoundingBoxDescent:4}]){
  const box={x:10,y:25,width:200*scale,height:height*scale},label=headingPlacement(box,metrics);
  if(box.width<32 || box.height<metrics.actualBoundingBoxAscent+metrics.actualBoundingBoxDescent+4)assert.equal(label,null);
  else{assert.ok(label.baseline-metrics.actualBoundingBoxAscent>=box.y+2);assert.ok(label.baseline+metrics.actualBoundingBoxDescent<=box.y+box.height-2);assert.ok(label.x>=box.x);}
 }
});
test('font bounding boxes with no descender still centre without cropping',()=>{
 const box={x:2,y:7,width:100,height:18},p=headingPlacement(box,{actualBoundingBoxAscent:9,actualBoundingBoxDescent:0});assert.equal(p.baseline,20.5);
});

test('missing and negative canvas bounds retain finite safe fallback positions',()=>{
 const box={x:0,y:0,width:100,height:24};assert.deepEqual(headingPlacement(box,{}),{x:4,baseline:15});assert.deepEqual(headingPlacement(box,{actualBoundingBoxAscent:-2,actualBoundingBoxDescent:-1}),{x:4,baseline:12});
});
