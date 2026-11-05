import {test} from 'node:test';import assert from 'node:assert/strict';
import {fitCamera,zoomCamera,jumpCamera,splitFraction} from './camera.js';
test('overview fits and centres both dimensions, readable mode preserves labels',()=>{
 const fit=fitCamera(1000,2000,500,500);assert.ok(fit.scale<.25);assert.ok(fit.x>0);assert.ok(fit.y>=18);
 assert.ok(fitCamera(1000,2000,500,500,true).scale>=.55);assert.ok(Number.isFinite(fitCamera(0,0,0,0).scale));
});
test('zoom anchors the world point and jump centres a distant region at the current zoom',()=>{
 const camera={x:20,y:30,scale:.5},point={x:100,y:90};const zoom=zoomCamera(camera,point,2);
 assert.equal((point.x-zoom.x)/zoom.scale,(point.x-camera.x)/camera.scale);assert.equal((point.y-zoom.y)/zoom.scale,(point.y-camera.y)/camera.scale);
 assert.equal(zoomCamera(camera,point,100).scale,4);assert.equal(zoomCamera(camera,point,0).scale,.001);
 assert.deepEqual(jumpCamera(camera,{x:900,y:800},400,300),{x:-250,y:-250,scale:.5});
});
test('split ratios clamp to usable bounds even for narrow containers and bad stored preferences',()=>{
 assert.equal(splitFraction(.5,1000,200,200),.5);assert.equal(splitFraction(0,1000,200,200),.2);assert.equal(splitFraction(1,1000,200,200),.8);
 assert.equal(splitFraction(NaN,1000,200,200),.5);assert.ok(splitFraction(1,100,200,200)<=.9);
});

test('overview respects either limiting dimension and zoom around offset cameras is exact',()=>{
 assert.deepEqual(fitCamera(1000,100,500,500),{x:18,y:226.8,scale:.464});
 assert.deepEqual(fitCamera(100,1000,500,500),{x:227.2,y:22,scale:.456});
 assert.deepEqual(fitCamera(10,10,500,500),{x:242.5,y:242.5,scale:1.5});
 const camera={x:23,y:-17,scale:1.3},point={x:147,y:93},zoom=zoomCamera(camera,point,1.2);
 assert.equal(zoom.scale,1.56);assert.ok(Math.abs(zoom.x-(-1.8))<1e-10);assert.ok(Math.abs(zoom.y-(-39))<1e-10);
});
